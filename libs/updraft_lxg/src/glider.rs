//! A high-level, named view of a glider preset.
//!
//! [`Glider`] is a named projection of the raw [`Section`] tree: it walks
//! the known register ids and exposes them as typed fields, and encodes
//! them back the way a device does. It is intentionally lossy — reading
//! drops unknown registers and exact on-wire types, and writing emits a
//! canonical file — so to preserve every field of a specific device file
//! byte-for-byte, use the low-level [`LxgFile`](crate::LxgFile). Every
//! field is optional because a preset may leave any of them unset (stored
//! as the `-16384` sentinel or simply absent), which maps to `None`.
//!
//! Units follow the file: masses in kg, arms in mm (negative = forward of
//! the datum), wing area in m², wing loading in kg/m², tank volumes in
//! litres, and **speeds in m/s** (multiply by 3.6 for km/h).

use crate::value::{Section, Value};

/// The value stored for a numeric field that has not been set.
const SENTINEL: f64 = -16384.0;

/// Register ids used by the glider record. Grouped by the section they
/// live in. See the crate's reverse-engineering notes for the full map.
mod reg {
    // Structure.
    pub const GLIDER_DEFS: u16 = 15900;
    pub const GLIDER_DEF: u16 = 15901;
    pub const POLAR: u16 = 16000;
    pub const BALLASTDUMP: u16 = 16090;
    pub const SPEEDS: u16 = 16500;
    pub const FLAP: u16 = 62100;

    // POLAR fields.
    pub const DESCRIPTION: u16 = 16001;
    pub const A: u16 = 16002;
    pub const B: u16 = 16003;
    pub const C: u16 = 16004;
    pub const MIN_LOAD: u16 = 16005;
    pub const MIN_WEIGHT: u16 = 16007;
    pub const MAX_WEIGHT: u16 = 16008;
    pub const EMPTY_WEIGHT: u16 = 16009;
    pub const WING_AREA: u16 = 16012;
    pub const MAX_WATER_TIPS: u16 = 16013;
    pub const MAX_FUEL_MAIN: u16 = 16014;
    pub const GLIDER_TYPE: u16 = 16020;
    pub const COMPETITION_CLASS: u16 = 16021;
    pub const MAX_PILOT: u16 = 16034;
    pub const MAX_COPILOT: u16 = 16035;
    pub const MAX_FUEL_AUX: u16 = 16036;
    pub const MAX_WATER_TAIL: u16 = 16038;
    pub const MAX_WATER_MAIN: u16 = 16039;
    pub const ARM_EMPTY: u16 = 16040;
    pub const ARM_PILOT: u16 = 16041;
    pub const ARM_COPILOT: u16 = 16042;
    pub const ARM_FUEL_MAIN: u16 = 16043;
    pub const ARM_FUEL_AUX: u16 = 16044;
    pub const ARM_WATER_TAIL_FIXED: u16 = 16045;
    pub const ARM_WATER_TAIL: u16 = 16046;
    pub const ARM_WATER_MAIN: u16 = 16047;
    pub const ARM_WATER_TIPS: u16 = 16048;

    // SPEEDS fields.
    pub const VS0: u16 = 16501;
    pub const VS1: u16 = 16502;
    pub const VFE: u16 = 16503;
    pub const VA: u16 = 16504;
    pub const VNE: u16 = 16505;
    pub const VAPP: u16 = 16506;
    pub const FLAP_SPEEDS: u16 = 16508;

    // BALLASTDUMP fields.
    pub const WING_DUMP_RATES: u16 = 16092;
    pub const WING_LITRES: u16 = 16093;
    pub const TAIL_DUMP_RATE: u16 = 16094;
    pub const TIPS_DUMP_RATE: u16 = 16095;

    // FLAP fields.
    pub const FLAP_TAGS: u16 = 62102;
}

/// The quadratic glide polar `sink = a·v² + b·v + c`, plus the wing
/// geometry the coefficients are referenced to.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Polar {
    /// Quadratic coefficient `a`.
    pub a: Option<f64>,
    /// Linear coefficient `b`.
    pub b: Option<f64>,
    /// Constant coefficient `c`.
    pub c: Option<f64>,
    /// Wing area, m².
    pub wing_area_m2: Option<f64>,
    /// Reference wing loading, kg/m².
    pub reference_load_kg_m2: Option<f64>,
}

/// Masses, in kg.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Masses {
    /// Empty mass.
    pub empty: Option<f64>,
    /// Reference mass the polar is defined at.
    pub reference: Option<f64>,
    /// Maximum take-off mass.
    pub max: Option<f64>,
    /// Maximum pilot mass.
    pub max_pilot: Option<f64>,
    /// Maximum co-pilot mass.
    pub max_copilot: Option<f64>,
    /// Maximum main (wing) water ballast.
    pub max_water_main: Option<f64>,
    /// Maximum tail water ballast.
    pub max_water_tail: Option<f64>,
    /// Maximum wing-tip water ballast.
    pub max_water_tips: Option<f64>,
    /// Maximum main fuel.
    pub max_fuel_main: Option<f64>,
    /// Maximum auxiliary fuel.
    pub max_fuel_aux: Option<f64>,
}

/// Centre-of-gravity arms, in mm from the datum (negative = forward).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Arms {
    /// Empty-glider CG arm.
    pub empty: Option<f64>,
    /// Pilot arm.
    pub pilot: Option<f64>,
    /// Co-pilot arm.
    pub copilot: Option<f64>,
    /// Main fuel-tank arm.
    pub fuel_main: Option<f64>,
    /// Auxiliary fuel-tank arm.
    pub fuel_aux: Option<f64>,
    /// Fixed tail-ballast arm.
    pub water_tail_fixed: Option<f64>,
    /// Tail water-ballast arm.
    pub water_tail: Option<f64>,
    /// Main (wing) water-ballast arm.
    pub water_main: Option<f64>,
    /// Wing-tip water-ballast arm.
    pub water_tips: Option<f64>,
}

/// Characteristic speeds, in m/s (multiply by 3.6 for km/h).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Speeds {
    /// Stall speed, clean (Vs0).
    pub stall: Option<f64>,
    /// Stall speed, landing configuration (Vs1).
    pub stall_landing: Option<f64>,
    /// Maximum flap-extended speed (Vfe).
    pub flaps_extended: Option<f64>,
    /// Manoeuvring speed (Va).
    pub maneuvering: Option<f64>,
    /// Never-exceed speed (Vne).
    pub never_exceed: Option<f64>,
    /// Approach speed (Vapp).
    pub approach: Option<f64>,
}

/// A flap position: its label and the top of its speed range (m/s).
#[derive(Clone, Debug, PartialEq)]
pub struct Flap {
    /// The flap-position label, e.g. `"+8"`, `"0"`, `"-14"`.
    pub label: String,
    /// The upper speed of this position's range, m/s, if set.
    pub max_speed: Option<f64>,
}

/// Water-ballast and fuel dump configuration.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Ballast {
    /// Wing-tank capacities, litres (one entry per configured step).
    pub wing_litres: Vec<f64>,
    /// Wing-tank dump rates paired with [`wing_litres`](Self::wing_litres).
    pub wing_dump_rates: Vec<f64>,
    /// Tail-tank dump rate.
    pub tail_dump_rate: Option<f64>,
    /// Wing-tip dump rate.
    pub tips_dump_rate: Option<f64>,
}

/// A glider preset in named, typed form.
///
/// Read one with [`Glider::from_bytes`] or
/// [`LxgFile::glider`](crate::LxgFile::glider), and write it back with
/// [`Glider::to_bytes`].
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Glider {
    /// Glider model / type name.
    pub name: Option<String>,
    /// Free-text description.
    pub description: Option<String>,
    /// Competition-class code.
    pub competition_class: Option<u16>,
    /// Glide polar and wing geometry.
    pub polar: Polar,
    /// Masses.
    pub masses: Masses,
    /// Centre-of-gravity arms.
    pub arms: Arms,
    /// Characteristic speeds.
    pub speeds: Speeds,
    /// Flap positions with their speed ranges.
    pub flaps: Vec<Flap>,
    /// Ballast tanks and dump rates.
    pub ballast: Ballast,
}

impl Glider {
    /// Reads a high-level glider view straight from `.lxg` bytes.
    ///
    /// # Errors
    ///
    /// Returns an [`Error`](crate::Error) if the bytes are not a
    /// well-formed `.lxg` file. A well-formed file with missing fields
    /// decodes fine; those fields are simply `None`.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, crate::Error> {
        Ok(crate::LxgFile::from_bytes(bytes)?.glider())
    }

    /// Extracts the high-level view from a decoded root section.
    pub(crate) fn from_root(root: &Section) -> Self {
        let Some(def) = root
            .get(reg::GLIDER_DEFS)
            .and_then(Value::as_section)
            .and_then(|s| s.get(reg::GLIDER_DEF))
            .and_then(Value::as_section)
        else {
            return Glider::default();
        };

        let polar = def.get(reg::POLAR).and_then(Value::as_section);
        let speeds = def.get(reg::SPEEDS).and_then(Value::as_section);
        let flap = def.get(reg::FLAP).and_then(Value::as_section);
        let dump = def.get(reg::BALLASTDUMP).and_then(Value::as_section);

        Glider {
            name: polar.and_then(|p| non_empty_str(p, reg::GLIDER_TYPE)),
            description: polar.and_then(|p| non_empty_str(p, reg::DESCRIPTION)),
            competition_class: polar
                .and_then(|p| p.get(reg::COMPETITION_CLASS))
                .and_then(Value::as_int)
                .and_then(|v| u16::try_from(v).ok()),
            polar: Polar {
                a: scalar(polar, reg::A),
                b: scalar(polar, reg::B),
                c: scalar(polar, reg::C),
                wing_area_m2: scalar(polar, reg::WING_AREA),
                reference_load_kg_m2: scalar(polar, reg::MIN_LOAD),
            },
            masses: Masses {
                empty: scalar(polar, reg::EMPTY_WEIGHT),
                reference: scalar(polar, reg::MIN_WEIGHT),
                max: scalar(polar, reg::MAX_WEIGHT),
                max_pilot: scalar(polar, reg::MAX_PILOT),
                max_copilot: scalar(polar, reg::MAX_COPILOT),
                max_water_main: scalar(polar, reg::MAX_WATER_MAIN),
                max_water_tail: scalar(polar, reg::MAX_WATER_TAIL),
                max_water_tips: scalar(polar, reg::MAX_WATER_TIPS),
                max_fuel_main: scalar(polar, reg::MAX_FUEL_MAIN),
                max_fuel_aux: scalar(polar, reg::MAX_FUEL_AUX),
            },
            arms: Arms {
                empty: scalar(polar, reg::ARM_EMPTY),
                pilot: scalar(polar, reg::ARM_PILOT),
                copilot: scalar(polar, reg::ARM_COPILOT),
                fuel_main: scalar(polar, reg::ARM_FUEL_MAIN),
                fuel_aux: scalar(polar, reg::ARM_FUEL_AUX),
                water_tail_fixed: scalar(polar, reg::ARM_WATER_TAIL_FIXED),
                water_tail: scalar(polar, reg::ARM_WATER_TAIL),
                water_main: scalar(polar, reg::ARM_WATER_MAIN),
                water_tips: scalar(polar, reg::ARM_WATER_TIPS),
            },
            speeds: Speeds {
                stall: scalar(speeds, reg::VS0),
                stall_landing: scalar(speeds, reg::VS1),
                flaps_extended: scalar(speeds, reg::VFE),
                maneuvering: scalar(speeds, reg::VA),
                never_exceed: scalar(speeds, reg::VNE),
                approach: scalar(speeds, reg::VAPP),
            },
            flaps: flaps(flap, speeds),
            ballast: Ballast {
                wing_litres: array(dump, reg::WING_LITRES),
                wing_dump_rates: array(dump, reg::WING_DUMP_RATES),
                tail_dump_rate: scalar(dump, reg::TAIL_DUMP_RATE),
                tips_dump_rate: scalar(dump, reg::TIPS_DUMP_RATE),
            },
        }
    }

    /// Encodes this glider to `.lxg` bytes.
    ///
    /// This is a canonical, normalizing writer: it emits the modeled fields
    /// with the on-wire types and array layout a device uses, and drops
    /// anything it does not model. Reading the result back yields an equal
    /// `Glider`. To preserve every field of a specific device file
    /// byte-for-byte, edit the low-level [`LxgFile`](crate::LxgFile).
    pub fn to_bytes(&self) -> Vec<u8> {
        self.to_file().to_bytes()
    }

    /// Builds the low-level [`LxgFile`](crate::LxgFile) representation of
    /// this glider.
    pub fn to_file(&self) -> crate::LxgFile {
        crate::LxgFile {
            version: 255,
            root: self.to_root(),
        }
    }

    fn to_root(&self) -> Section {
        let mut polar = Section::new();
        push_str(&mut polar, reg::GLIDER_TYPE, self.name.as_deref());
        push_str(&mut polar, reg::DESCRIPTION, self.description.as_deref());
        if let Some(class) = self.competition_class {
            polar.set(reg::COMPETITION_CLASS, Value::U16(class));
        }
        push_f32(&mut polar, reg::A, self.polar.a);
        push_f32(&mut polar, reg::B, self.polar.b);
        push_f32(&mut polar, reg::C, self.polar.c);
        push_f32(&mut polar, reg::WING_AREA, self.polar.wing_area_m2);
        push_f32(&mut polar, reg::MIN_LOAD, self.polar.reference_load_kg_m2);
        push_i32(&mut polar, reg::EMPTY_WEIGHT, self.masses.empty);
        push_i32(&mut polar, reg::MIN_WEIGHT, self.masses.reference);
        push_i32(&mut polar, reg::MAX_WEIGHT, self.masses.max);
        push_f32(&mut polar, reg::MAX_PILOT, self.masses.max_pilot);
        push_f32(&mut polar, reg::MAX_COPILOT, self.masses.max_copilot);
        push_f32(&mut polar, reg::MAX_WATER_MAIN, self.masses.max_water_main);
        push_f32(&mut polar, reg::MAX_WATER_TAIL, self.masses.max_water_tail);
        push_f32(&mut polar, reg::MAX_WATER_TIPS, self.masses.max_water_tips);
        push_f32(&mut polar, reg::MAX_FUEL_MAIN, self.masses.max_fuel_main);
        push_f32(&mut polar, reg::MAX_FUEL_AUX, self.masses.max_fuel_aux);
        push_i32(&mut polar, reg::ARM_EMPTY, self.arms.empty);
        push_i32(&mut polar, reg::ARM_PILOT, self.arms.pilot);
        push_i32(&mut polar, reg::ARM_COPILOT, self.arms.copilot);
        push_i32(&mut polar, reg::ARM_FUEL_MAIN, self.arms.fuel_main);
        push_i32(&mut polar, reg::ARM_FUEL_AUX, self.arms.fuel_aux);
        push_i32(
            &mut polar,
            reg::ARM_WATER_TAIL_FIXED,
            self.arms.water_tail_fixed,
        );
        push_i32(&mut polar, reg::ARM_WATER_TAIL, self.arms.water_tail);
        push_i32(&mut polar, reg::ARM_WATER_MAIN, self.arms.water_main);
        push_i32(&mut polar, reg::ARM_WATER_TIPS, self.arms.water_tips);

        let mut speeds = Section::new();
        push_f32(&mut speeds, reg::VS0, self.speeds.stall);
        push_f32(&mut speeds, reg::VS1, self.speeds.stall_landing);
        push_f32(&mut speeds, reg::VFE, self.speeds.flaps_extended);
        push_f32(&mut speeds, reg::VA, self.speeds.maneuvering);
        push_f32(&mut speeds, reg::VNE, self.speeds.never_exceed);
        push_f32(&mut speeds, reg::VAPP, self.speeds.approach);
        if !self.flaps.is_empty() {
            speeds.set(reg::FLAP_SPEEDS, flap_speed_array(&self.flaps));
        }

        let mut flap = Section::new();
        if !self.flaps.is_empty() {
            flap.set(reg::FLAP_TAGS, flap_tag_array(&self.flaps));
        }

        let mut dump = Section::new();
        if !self.ballast.wing_litres.is_empty() {
            dump.set(
                reg::WING_LITRES,
                f32_array(&self.ballast.wing_litres, BALLAST_SLOTS),
            );
        }
        if !self.ballast.wing_dump_rates.is_empty() {
            let rates = f32_array(&self.ballast.wing_dump_rates, BALLAST_SLOTS);
            dump.set(reg::WING_DUMP_RATES, rates);
        }
        push_f32(&mut dump, reg::TAIL_DUMP_RATE, self.ballast.tail_dump_rate);
        push_f32(&mut dump, reg::TIPS_DUMP_RATE, self.ballast.tips_dump_rate);

        let mut def = Section::new();
        push_section(&mut def, reg::POLAR, polar);
        push_section(&mut def, reg::SPEEDS, speeds);
        push_section(&mut def, reg::FLAP, flap);
        push_section(&mut def, reg::BALLASTDUMP, dump);

        let mut defs = Section::new();
        defs.set(reg::GLIDER_DEF, Value::Section(def));
        let mut root = Section::new();
        root.set(reg::GLIDER_DEFS, Value::Section(defs));
        root
    }
}

/// Reads a numeric field as `f64`, treating the unset sentinel and any
/// non-numeric or missing value as `None`.
fn scalar(section: Option<&Section>, id: u16) -> Option<f64> {
    let value = section?.get(id)?;
    let n = match value {
        Value::F32(v) => f64::from(*v),
        Value::U8(v) => f64::from(*v),
        Value::U16(v) => f64::from(*v),
        Value::U32(v) => f64::from(*v),
        Value::I32(v) => f64::from(*v),
        _ => return None,
    };
    (n != SENTINEL).then_some(n)
}

/// Reads a string field, mapping missing or empty to `None`.
fn non_empty_str(section: &Section, id: u16) -> Option<String> {
    section
        .get(id)
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
}

/// Reads a float array, dropping the trailing unset-sentinel padding.
fn array(section: Option<&Section>, id: u16) -> Vec<f64> {
    let Some(values) = section
        .and_then(|s| s.get(id))
        .and_then(Value::as_f32_array)
    else {
        return Vec::new();
    };
    let kept = values
        .iter()
        .rposition(|v| f64::from(*v) != SENTINEL)
        .map_or(0, |i| i + 1);
    values[..kept].iter().map(|v| f64::from(*v)).collect()
}

/// Pairs flap labels (`FLAP.Tags`) with their speed-range tops
/// (`SPEEDS.Flap`), keeping only the positions that have a label.
fn flaps(flap: Option<&Section>, speeds: Option<&Section>) -> Vec<Flap> {
    let labels = flap
        .and_then(|s| s.get(reg::FLAP_TAGS))
        .and_then(flap_labels)
        .unwrap_or_default();
    let max_speeds = speeds
        .and_then(|s| s.get(reg::FLAP_SPEEDS))
        .and_then(Value::as_f32_array)
        .unwrap_or_default();

    labels
        .into_iter()
        .enumerate()
        .filter(|(_, label)| !label.is_empty())
        .map(|(i, label)| {
            let max_speed = max_speeds
                .get(i)
                .map(|v| f64::from(*v))
                .filter(|v| *v != SENTINEL);
            Flap { label, max_speed }
        })
        .collect()
}

/// Decodes the flap-label array: fixed [`TAG_STRIDE`]-byte, NUL-padded slots.
fn flap_labels(value: &Value) -> Option<Vec<String>> {
    let Value::Array { data, .. } = value else {
        return None;
    };
    Some(
        data.chunks(TAG_STRIDE)
            .map(|slot| {
                let end = slot.iter().position(|&b| b == 0).unwrap_or(slot.len());
                String::from_utf8_lossy(&slot[..end]).into_owned()
            })
            .collect(),
    )
}

/// Extension-type byte for `f32` arrays (flap speeds, ballast tables).
const EXT_F32_ARRAY: u8 = 0x2B;
/// Extension-type byte for the flap-label array.
const EXT_TAGS: u8 = 0x27;
/// Number of slots in a flap array (labels and speeds).
const FLAP_SLOTS: usize = 10;
/// Bytes per flap-label slot.
const TAG_STRIDE: usize = 3;
/// Number of slots in a ballast-table array.
const BALLAST_SLOTS: usize = 20;

/// Writes a non-empty string field.
fn push_str(section: &mut Section, id: u16, value: Option<&str>) {
    if let Some(s) = value.filter(|s| !s.is_empty()) {
        section.set(id, Value::Str(s.to_owned()));
    }
}

/// Writes a set numeric field as `f32`.
fn push_f32(section: &mut Section, id: u16, value: Option<f64>) {
    if let Some(v) = value {
        section.set(id, Value::F32(v as f32));
    }
}

/// Writes a set numeric field as `i32` (masses and arms are stored as
/// integers), rounding to the nearest whole value.
fn push_i32(section: &mut Section, id: u16, value: Option<f64>) {
    if let Some(v) = value {
        section.set(id, Value::I32(v.round() as i32));
    }
}

/// Adds a sub-section to `def`, skipping it if it has no fields.
fn push_section(def: &mut Section, id: u16, section: Section) {
    if !section.entries.is_empty() {
        def.set(id, Value::Section(section));
    }
}

/// Builds an `f32` array value, padding to `slots` with the unset sentinel
/// and truncating anything past `slots`.
fn f32_array(values: &[f64], slots: usize) -> Value {
    let mut data = Vec::with_capacity(slots * 4);
    for i in 0..slots {
        let v = values.get(i).map_or(SENTINEL, |v| *v) as f32;
        data.extend_from_slice(&v.to_le_bytes());
    }
    Value::Array {
        ext_type: EXT_F32_ARRAY,
        data,
    }
}

/// Builds the per-flap speed array (`SPEEDS.Flap`), one slot per flap.
fn flap_speed_array(flaps: &[Flap]) -> Value {
    let speeds: Vec<f64> = flaps
        .iter()
        .map(|f| f.max_speed.unwrap_or(SENTINEL))
        .collect();
    f32_array(&speeds, FLAP_SLOTS)
}

/// Builds the flap-label array (`FLAP.Tags`): `FLAP_SLOTS` fixed
/// `TAG_STRIDE`-byte slots. Labels longer than a slot are truncated.
fn flap_tag_array(flaps: &[Flap]) -> Value {
    let mut data = vec![0u8; FLAP_SLOTS * TAG_STRIDE];
    for (i, flap) in flaps.iter().take(FLAP_SLOTS).enumerate() {
        let bytes = flap.label.as_bytes();
        let n = bytes.len().min(TAG_STRIDE);
        data[i * TAG_STRIDE..i * TAG_STRIDE + n].copy_from_slice(&bytes[..n]);
    }
    Value::Array {
        ext_type: EXT_TAGS,
        data,
    }
}
