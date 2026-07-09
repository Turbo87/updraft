//! Turns the per-base-model TOML files in `data/` into a `const PRESETS`
//! catalogue at build time.
//!
//! Each file describes one base model whose variants are grouped by
//! wingspan. A `[[wingspan]]` block fixes the span (and the `DMSt`
//! handicap that the span's variants share), and each
//! `[[wingspan.variant]]` under it is one build/propulsion variant at that
//! span. A variant that appears in several blocks (e.g. removable tips) is
//! reassembled into one `Variant` with several wingspan leaves. A variant
//! may `inherits_from` another in the same block to copy its fields (the
//! "same airframe, different engine" case). We emit fully-qualified
//! `const` code into `$OUT_DIR/catalogue.rs`.

use std::path::Path;
use std::{env, fmt::Write as _, fs};

use serde::{Deserialize, de};

/// A float field that also accepts a TOML integer literal, so hand-written
/// data can say `span = 18` or `a = 1` (as in the schema examples) rather
/// than being forced to `18.0` / `1.0`.
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
    seats: u8,
    #[serde(default)]
    wingspan: Vec<Block>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Block {
    /// Wingspan in metres, shared by every variant in the block. Absent
    /// for fixed-span gliders that don't quote one.
    span: Option<F64>,
    /// Handicap shared by the block's variants; a variant may override it.
    dmst_index: Option<F64>,
    #[serde(default)]
    variant: Vec<Entry>,
}

#[derive(Deserialize, Clone, Default)]
#[serde(deny_unknown_fields)]
struct Entry {
    name: Option<String>,
    inherits_from: Option<String>,
    propulsion: Option<String>,
    wing_area: Option<F64>,
    reference_mass: Option<F64>,
    polar_coefficients: Option<Coeffs>,
    empty_mass: Option<F64>,
    max_takeoff_mass: Option<F64>,
    water_ballast_capacity: Option<F64>,
    vne: Option<F64>,
    stall_speed: Option<F64>,
    weglide_id: Option<u32>,
    /// Overrides the block's `dmst_index` for this variant.
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
    /// Fills every unset field of `self` from `parent`. The identity
    /// fields (`name`, `inherits_from`) are never inherited.
    fn inherit(&mut self, parent: &Entry) {
        macro_rules! fill {
            ($($f:ident),*) => { $( if self.$f.is_none() { self.$f = parent.$f.clone(); } )* };
        }
        fill!(
            propulsion,
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

/// One wingspan configuration (leaf) after grouping and inheritance.
struct Leaf {
    wingspan: Option<f64>,
    wing_area: Option<f64>,
    coefficients: Option<(f64, f64, f64)>,
    reference_mass: Option<f64>,
    empty_mass: Option<f64>,
    max_takeoff_mass: Option<f64>,
    water_ballast_capacity: Option<f64>,
    vne: Option<f64>,
    stall_speed: Option<f64>,
    weglide_id: Option<u32>,
    dmst_index: Option<f64>,
}

/// A variant accumulated across the blocks it appears in.
struct Variant {
    name: String,
    propulsion: Option<String>,
    leaves: Vec<Leaf>,
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
    let mut variants: Vec<Variant> = Vec::new();

    for block in &file.wingspan {
        // Resolve inheritance within the block, in declaration order.
        let mut resolved: Vec<Entry> = Vec::new();
        for entry in &block.variant {
            let mut entry = entry.clone();
            if let Some(parent_name) = entry.inherits_from.clone() {
                let parent = resolved
                    .iter()
                    .find(|e| e.name.as_deref() == Some(parent_name.as_str()))
                    .unwrap_or_else(|| {
                        panic!(
                            "{}: inherits_from = {parent_name:?} not found earlier in the block",
                            path.display()
                        )
                    })
                    .clone();
                entry.inherit(&parent);
            }
            resolved.push(entry);
        }

        for entry in resolved {
            let name = entry.name.clone().unwrap_or_else(|| {
                panic!("{}: a wingspan variant is missing `name`", path.display())
            });
            let leaf = Leaf {
                wingspan: block.span.map(|v| v.0),
                wing_area: entry.wing_area.map(|v| v.0),
                coefficients: entry.polar_coefficients.map(|c| (c.a.0, c.b.0, c.c.0)),
                reference_mass: entry.reference_mass.map(|v| v.0),
                empty_mass: entry.empty_mass.map(|v| v.0),
                max_takeoff_mass: entry.max_takeoff_mass.map(|v| v.0),
                water_ballast_capacity: entry.water_ballast_capacity.map(|v| v.0),
                vne: entry.vne.map(|v| v.0),
                stall_speed: entry.stall_speed.map(|v| v.0),
                weglide_id: entry.weglide_id,
                // A variant's own handicap wins over the block default.
                dmst_index: entry.dmst_index.or(block.dmst_index).map(|v| v.0),
            };
            if entry.coefficients_without_mass() {
                panic!(
                    "{}: variant {name:?} has polar_coefficients but no reference_mass",
                    path.display()
                );
            }
            match variants.iter_mut().find(|v| v.name == name) {
                Some(v) => {
                    if v.propulsion.is_none() {
                        v.propulsion = entry.propulsion.clone();
                    }
                    v.leaves.push(leaf);
                }
                None => variants.push(Variant {
                    name,
                    propulsion: entry.propulsion.clone(),
                    leaves: vec![leaf],
                }),
            }
        }
    }

    variants.sort_by(|a, b| a.name.cmp(&b.name));

    let mut vs = String::new();
    for variant in &mut variants {
        variant.leaves.sort_by(|a, b| {
            let (av, bv) = (a.wingspan, b.wingspan);
            av.partial_cmp(&bv).unwrap_or(std::cmp::Ordering::Equal)
        });
        let mut ws = String::new();
        for leaf in &variant.leaves {
            write!(ws, "{}, ", render_leaf(leaf)).unwrap();
        }
        write!(
            vs,
            "crate::Variant {{ name: {}, propulsion: {}, wingspans: &[{}] }}, ",
            rust_str(&variant.name),
            opt_propulsion(variant.propulsion.as_deref(), path),
            ws.trim_end(),
        )
        .unwrap();
    }

    format!(
        "    crate::AircraftPreset {{ name: {}, manufacturer: {}, seats: {}, variants: &[{}] }},\n",
        rust_str(&file.model),
        opt(file.manufacturer.as_ref().map(|m| rust_str(m))),
        file.seats,
        vs.trim_end(),
    )
}

impl Entry {
    fn coefficients_without_mass(&self) -> bool {
        self.polar_coefficients.is_some() && self.reference_mass.is_none()
    }
}

fn render_leaf(leaf: &Leaf) -> String {
    let polar = match (leaf.coefficients, leaf.reference_mass) {
        (Some((a, b, c)), Some(ref_mass)) => format!(
            "Some(crate::Polar::new(({}, {}, {}), {}))",
            f(a),
            f(b),
            f(c),
            mass(ref_mass)
        ),
        _ => "None".to_string(),
    };
    format!(
        "crate::WingspanConfig {{ wingspan: {}, wing_area: {}, polar: {}, empty_mass: {}, \
         max_takeoff_mass: {}, water_ballast_capacity: {}, vne: {}, stall_speed: {}, \
         flap_speeds: &[], cg_limits: None, weglide_id: {}, fallback_handicap: {} }}",
        opt(leaf
            .wingspan
            .map(|v| format!("updraft_units::Length::from_meters({})", f(v)))),
        opt(leaf
            .wing_area
            .map(|v| format!("updraft_units::Area::from_square_meters({})", f(v)))),
        polar,
        opt(leaf.empty_mass.map(mass)),
        opt(leaf.max_takeoff_mass.map(mass)),
        opt(leaf.water_ballast_capacity.map(mass)),
        opt(leaf.vne.map(speed)),
        opt(leaf.stall_speed.map(speed)),
        opt(leaf.weglide_id.map(|v| v.to_string())),
        opt(leaf.dmst_index.map(f)),
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
