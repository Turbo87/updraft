//! Turns the per-base-model TOML files in `data/` into a `const PRESETS`
//! catalogue at build time.
//!
//! Each file describes one base model and holds a flat list of `[[preset]]`
//! entries, one per (variant, wingspan) leaf. An entry may `inherits_from`
//! an earlier entry in the same file to copy its fields and override a few
//! (the classic "same airframe, different engine" case). We resolve that
//! inheritance, group the entries into the base -> variant -> wingspan tree,
//! and emit fully-qualified `const` code into `$OUT_DIR/catalogue.rs`.

use std::path::Path;
use std::{env, fmt::Write as _, fs};

use serde::{Deserialize, de};

/// A float field that also accepts a TOML integer literal, so hand-written
/// data can say `wingspan = 18` or `a = 1` (as in the schema examples)
/// rather than being forced to `18.0` / `1.0`.
#[derive(Clone, Copy)]
struct F64(f64);

impl<'de> Deserialize<'de> for F64 {
    fn deserialize<D: de::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct V;
        impl de::Visitor<'_> for V {
            type Value = f64;
            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("a number")
            }
            fn visit_f64<E>(self, v: f64) -> Result<f64, E> {
                Ok(v)
            }
            fn visit_i64<E>(self, v: i64) -> Result<f64, E> {
                Ok(v as f64)
            }
            fn visit_u64<E>(self, v: u64) -> Result<f64, E> {
                Ok(v as f64)
            }
        }
        d.deserialize_any(V).map(F64)
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PresetFile {
    model: String,
    manufacturer: Option<String>,
    /// Seat count for the whole model; the seat count does not vary
    /// between variants.
    seats: Option<u8>,
    #[serde(default)]
    preset: Vec<Entry>,
}

#[derive(Deserialize, Clone, Default)]
#[serde(deny_unknown_fields)]
struct Entry {
    id: Option<String>,
    inherits_from: Option<String>,
    variant: Option<String>,
    propulsion: Option<String>,
    wingspan: Option<F64>,
    wing_area: Option<F64>,
    reference_mass: Option<F64>,
    polar_coefficients: Option<Coeffs>,
    empty_mass: Option<F64>,
    max_takeoff_mass: Option<F64>,
    water_ballast_capacity: Option<F64>,
    vne: Option<F64>,
    stall_speed: Option<F64>,
    weglide_id: Option<u32>,
    dmst_index: Option<F64>,
}

#[derive(Deserialize, Clone, Copy)]
#[serde(deny_unknown_fields)]
struct Coeffs {
    a: F64,
    b: F64,
    c: F64,
}

impl Entry {
    fn id(&self) -> &str {
        self.id.as_deref().or(self.variant.as_deref()).unwrap_or("")
    }

    /// Fills every unset field of `self` from `parent`. The identity
    /// fields (`variant`, `id`, `inherits_from`) are never inherited.
    fn inherit(&mut self, parent: &Entry) {
        macro_rules! fill {
            ($($f:ident),*) => { $( if self.$f.is_none() { self.$f = parent.$f.clone(); } )* };
        }
        fill!(
            propulsion,
            wingspan,
            wing_area,
            reference_mass,
            polar_coefficients,
            empty_mass,
            max_takeoff_mass,
            water_ballast_capacity,
            vne,
            stall_speed,
            weglide_id,
            dmst_index
        );
    }
}

fn main() {
    let manifest = env::var("CARGO_MANIFEST_DIR").unwrap();
    let data_dir = Path::new(&manifest).join("data");
    println!("cargo:rerun-if-changed={}", data_dir.display());
    println!("cargo:rerun-if-changed=build.rs");

    let mut paths: Vec<_> = fs::read_dir(&data_dir)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", data_dir.display()))
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|x| x == "toml"))
        .collect();
    paths.sort();

    let mut bases: Vec<(String, String)> = Vec::new();
    for path in &paths {
        let text = fs::read_to_string(path).unwrap();
        let file: PresetFile =
            toml::from_str(&text).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        bases.push((file.model.clone(), render_base(&file, path)));
    }

    // Order by base-model name the same way Rust compares `&str` (byte
    // order), which the `sorted_by_name` test relies on.
    bases.sort_by(|a, b| a.0.cmp(&b.0));

    let mut out = String::new();
    out.push_str("#[allow(clippy::approx_constant, clippy::unreadable_literal)]\n");
    out.push_str("pub const PRESETS: &[crate::AircraftPreset] = &[\n");
    for (_, code) in &bases {
        out.push_str(code);
    }
    out.push_str("];\n");

    let dest = Path::new(&env::var("OUT_DIR").unwrap()).join("catalogue.rs");
    fs::write(dest, out).unwrap();
}

/// Renders one file into a single `AircraftPreset { ... },` line.
fn render_base(file: &PresetFile, path: &Path) -> String {
    // Resolve inheritance in declaration order.
    let mut resolved: Vec<Entry> = Vec::new();
    for entry in &file.preset {
        let mut entry = entry.clone();
        if let Some(parent_id) = entry.inherits_from.clone() {
            let parent = resolved
                .iter()
                .find(|e| e.id() == parent_id)
                .unwrap_or_else(|| {
                    panic!(
                        "{}: inherits_from = {parent_id:?} not found earlier",
                        path.display()
                    )
                })
                .clone();
            entry.inherit(&parent);
        }
        assert!(
            entry.variant.is_some(),
            "{}: a preset entry is missing `variant`",
            path.display()
        );
        resolved.push(entry);
    }

    // Group into variants (in first-seen order), each variant into wingspan
    // leaves.
    let mut variants: Vec<(String, Vec<Entry>)> = Vec::new();
    for entry in resolved {
        let name = entry.variant.clone().unwrap();
        match variants.iter_mut().find(|(n, _)| *n == name) {
            Some((_, leaves)) => leaves.push(entry),
            None => variants.push((name, vec![entry])),
        }
    }
    variants.sort_by(|a, b| a.0.cmp(&b.0));

    let mut vs = String::new();
    for (name, mut leaves) in variants {
        leaves.sort_by(|a, b| {
            let (av, bv) = (a.wingspan.map(|v| v.0), b.wingspan.map(|v| v.0));
            av.partial_cmp(&bv).unwrap_or(std::cmp::Ordering::Equal)
        });
        let propulsion = leaves.iter().find_map(|e| e.propulsion.clone());
        let mut ws = String::new();
        for leaf in &leaves {
            write!(ws, "{}, ", render_leaf(leaf, path)).unwrap();
        }
        write!(
            vs,
            "crate::Variant {{ name: {}, propulsion: {}, wingspans: &[{}] }}, ",
            rust_str(&name),
            opt_propulsion(propulsion.as_deref(), path),
            ws.trim_end(),
        )
        .unwrap();
    }

    format!(
        "    crate::AircraftPreset {{ name: {}, manufacturer: {}, seats: {}, variants: &[{}] }},\n",
        rust_str(&file.model),
        opt(file.manufacturer.as_ref().map(|m| rust_str(m))),
        opt(file.seats.map(|s| s.to_string())),
        vs.trim_end(),
    )
}

fn render_leaf(e: &Entry, path: &Path) -> String {
    let polar = match (&e.polar_coefficients, e.reference_mass) {
        (Some(c), Some(ref_mass)) => format!(
            "Some(crate::Polar::new(({}, {}, {}), {}))",
            f(c.a.0),
            f(c.b.0),
            f(c.c.0),
            mass(ref_mass.0)
        ),
        (Some(_), None) => {
            panic!(
                "{}: variant {:?} has polar_coefficients but no reference_mass",
                path.display(),
                e.variant
            )
        }
        (None, _) => "None".to_string(),
    };
    format!(
        "crate::WingspanConfig {{ wingspan: {}, wing_area: {}, polar: {}, empty_mass: {}, \
         max_takeoff_mass: {}, water_ballast_capacity: {}, vne: {}, stall_speed: {}, \
         flap_speeds: &[], cg_limits: None, weglide_id: {}, fallback_handicap: {} }}",
        opt(e
            .wingspan
            .map(|v| format!("updraft_units::Length::from_meters({})", f(v.0)))),
        opt(e
            .wing_area
            .map(|v| format!("updraft_units::Area::from_square_meters({})", f(v.0)))),
        polar,
        opt(e.empty_mass.map(|v| mass(v.0))),
        opt(e.max_takeoff_mass.map(|v| mass(v.0))),
        opt(e.water_ballast_capacity.map(|v| mass(v.0))),
        opt(e.vne.map(|v| speed(v.0))),
        opt(e.stall_speed.map(|v| speed(v.0))),
        opt(e.weglide_id.map(|v| v.to_string())),
        opt(e.dmst_index.map(|v| f(v.0))),
    )
}

fn mass(v: f64) -> String {
    format!("updraft_units::Mass::from_kilograms({})", f(v))
}

fn speed(v: f64) -> String {
    format!("updraft_units::Speed::from_kilometers_per_hour({})", f(v))
}

fn opt_propulsion(p: Option<&str>, path: &Path) -> String {
    let Some(p) = p else {
        return "None".to_string();
    };
    let variant = match p {
        "pure" => "Pure",
        "self_launch" => "SelfLaunch",
        "sustainer" => "Sustainer",
        "electric_sustainer" => "ElectricSustainer",
        other => panic!("{}: unknown propulsion {other:?}", path.display()),
    };
    format!("Some(crate::Propulsion::{variant})")
}

/// Wraps a rendered inner value in `Some(...)`, or `None`.
fn opt(v: Option<String>) -> String {
    v.map(|v| format!("Some({v})"))
        .unwrap_or_else(|| "None".to_string())
}

/// Formats an `f64` as a round-tripping Rust literal (`322.0`, `-2.4`).
fn f(v: f64) -> String {
    format!("{v:?}")
}

fn rust_str(s: &str) -> String {
    format!("{s:?}")
}
