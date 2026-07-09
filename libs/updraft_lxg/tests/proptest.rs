//! Property tests.
//!
//! The primary property is panic-freedom: [`LxgFile::from_bytes`] must
//! return `Ok`/`Err` for *any* input, never panic (no slice-index or
//! integer-overflow crashes). The secondary property is that encoding is a
//! stable round-trip for any well-formed value tree.

use proptest::prelude::*;
use updraft_lxg::{Arms, Ballast, Flap, Glider, LxgFile, Masses, Polar, Section, Speeds, Value};

/// A strategy for arbitrary values, bounded so a generated tree stays well
/// under the `u16` section-size limit and only exercises the narrow
/// encodings real files use.
fn value_strategy() -> impl Strategy<Value = Value> {
    let leaf = prop_oneof![
        any::<u8>().prop_map(Value::U8),
        any::<u16>().prop_map(Value::U16),
        any::<u32>().prop_map(Value::U32),
        any::<i32>().prop_map(Value::I32),
        any::<f32>().prop_map(Value::F32),
        "[ -~]{0,32}".prop_map(Value::Str),
        proptest::collection::vec(any::<u8>(), 0..16).prop_map(Value::Bin),
        (any::<u8>(), proptest::collection::vec(any::<u8>(), 0..16))
            .prop_map(|(ext_type, data)| Value::Array { ext_type, data }),
    ];

    leaf.prop_recursive(4, 32, 6, |inner| {
        proptest::collection::vec((any::<u16>(), inner), 0..6)
            .prop_map(|entries| Value::Section(Section { entries }))
    })
}

fn section_strategy() -> impl Strategy<Value = Section> {
    proptest::collection::vec((any::<u16>(), value_strategy()), 0..8)
        .prop_map(|entries| Section { entries })
}

/// A value that survives the high-level lossy encoding: an `f32`-exact
/// float that is not the unset sentinel.
fn round_f32() -> impl Strategy<Value = f64> {
    any::<f32>()
        .prop_filter("finite, non-sentinel", |v| v.is_finite() && *v != -16384.0)
        .prop_map(f64::from)
}

/// A whole-number field (masses and arms are stored as integers).
fn round_int() -> impl Strategy<Value = f64> {
    (-30000i32..30000)
        .prop_filter("non-sentinel", |v| *v != -16384)
        .prop_map(f64::from)
}

prop_compose! {
    fn round_polar()(a in prop::option::of(round_f32()), b in prop::option::of(round_f32()),
        c in prop::option::of(round_f32()), wing in prop::option::of(round_f32()),
        load in prop::option::of(round_f32())) -> Polar {
        Polar { a, b, c, wing_area_m2: wing, reference_load_kg_m2: load }
    }
}

prop_compose! {
    fn round_masses()(empty in prop::option::of(round_int()), reference in prop::option::of(round_int()),
        max in prop::option::of(round_int()), max_pilot in prop::option::of(round_f32()),
        max_copilot in prop::option::of(round_f32()), max_water_main in prop::option::of(round_f32()),
        max_water_tail in prop::option::of(round_f32()), max_water_tips in prop::option::of(round_f32()),
        max_fuel_main in prop::option::of(round_f32()), max_fuel_aux in prop::option::of(round_f32())) -> Masses {
        Masses { empty, reference, max, max_pilot, max_copilot, max_water_main,
            max_water_tail, max_water_tips, max_fuel_main, max_fuel_aux }
    }
}

prop_compose! {
    fn round_arms()(empty in prop::option::of(round_int()), pilot in prop::option::of(round_int()),
        copilot in prop::option::of(round_int()), fuel_main in prop::option::of(round_int()),
        fuel_aux in prop::option::of(round_int()), water_tail_fixed in prop::option::of(round_int()),
        water_tail in prop::option::of(round_int()), water_main in prop::option::of(round_int()),
        water_tips in prop::option::of(round_int())) -> Arms {
        Arms { empty, pilot, copilot, fuel_main, fuel_aux, water_tail_fixed, water_tail, water_main, water_tips }
    }
}

prop_compose! {
    fn round_speeds()(stall in prop::option::of(round_f32()), stall_landing in prop::option::of(round_f32()),
        flaps_extended in prop::option::of(round_f32()), maneuvering in prop::option::of(round_f32()),
        never_exceed in prop::option::of(round_f32()), approach in prop::option::of(round_f32())) -> Speeds {
        Speeds { stall, stall_landing, flaps_extended, maneuvering, never_exceed, approach }
    }
}

prop_compose! {
    fn round_flap()(label in "[A-Za-z0-9+\\-]{1,3}", max_speed in prop::option::of(round_f32())) -> Flap {
        Flap { label, max_speed }
    }
}

prop_compose! {
    fn round_ballast()(
        wing_litres in proptest::collection::vec(round_f32(), 0..=20),
        wing_dump_rates in proptest::collection::vec(round_f32(), 0..=20),
        tail_dump_rate in prop::option::of(round_f32()),
        tips_dump_rate in prop::option::of(round_f32()),
    ) -> Ballast {
        Ballast { wing_litres, wing_dump_rates, tail_dump_rate, tips_dump_rate }
    }
}

prop_compose! {
    /// A glider constrained to values that survive the lossy high-level
    /// encoding, so `from_bytes(g.to_bytes()) == g`.
    fn roundtrippable_glider()(
        name in prop::option::of("[A-Za-z0-9 ]{1,20}"),
        description in prop::option::of("[A-Za-z0-9 ]{1,20}"),
        competition_class in prop::option::of(any::<u16>()),
        polar in round_polar(),
        masses in round_masses(),
        arms in round_arms(),
        speeds in round_speeds(),
        flaps in proptest::collection::vec(round_flap(), 0..=10),
        ballast in round_ballast(),
    ) -> Glider {
        Glider { name, description, competition_class, polar, masses, arms, speeds, flaps, ballast }
    }
}

prop_compose! {
    /// A glider with hostile values (NaN/inf, over-long labels and arrays,
    /// empty strings) to prove the writer never panics.
    fn wild_glider()(
        name in prop::option::of(".{0,40}"),
        competition_class in prop::option::of(any::<u16>()),
        a in prop::option::of(any::<f64>()),
        empty in prop::option::of(any::<f64>()),
        flaps in proptest::collection::vec(
            (".{0,6}", prop::option::of(any::<f64>())).prop_map(|(label, max_speed)| Flap { label, max_speed }),
            0..15),
        wing in proptest::collection::vec(any::<f64>(), 0..30),
    ) -> Glider {
        Glider {
            name,
            competition_class,
            polar: Polar { a, ..Polar::default() },
            masses: Masses { empty, ..Masses::default() },
            flaps,
            ballast: Ballast { wing_litres: wing, ..Ballast::default() },
            ..Glider::default()
        }
    }
}

proptest! {
    /// Arbitrary bytes never make the decoder panic.
    #[test]
    fn from_bytes_never_panics(bytes in proptest::collection::vec(any::<u8>(), 0..4096)) {
        let _ = LxgFile::from_bytes(&bytes);
    }

    /// Arbitrary bytes behind a valid version prefix never panic either —
    /// this drives the section parser far past the shallow rejections.
    #[test]
    fn from_bytes_never_panics_after_version(rest in proptest::collection::vec(any::<u8>(), 0..4096)) {
        let mut bytes = vec![0xCC, 0xFF];
        bytes.extend(rest);
        let _ = LxgFile::from_bytes(&bytes);
    }

    /// Encoding any well-formed file and decoding it back yields an
    /// identical byte stream (encode is a stable fixed point). Comparing
    /// bytes rather than values sidesteps `NaN != NaN`.
    #[test]
    fn encode_decode_is_stable(version in any::<u8>(), root in section_strategy()) {
        let file = LxgFile { version, root };
        let bytes = file.to_bytes();
        let decoded = LxgFile::from_bytes(&bytes).expect("self-encoded file must decode");
        prop_assert_eq!(decoded.to_bytes(), bytes);
    }

    /// The high-level writer never panics, however hostile the glider.
    #[test]
    fn glider_write_never_panics(glider in wild_glider()) {
        let bytes = glider.to_bytes();
        let _ = Glider::from_bytes(&bytes);
    }

    /// A glider of encodable values survives a write/read cycle unchanged.
    #[test]
    fn glider_write_round_trips(glider in roundtrippable_glider()) {
        let back = Glider::from_bytes(&glider.to_bytes())
            .expect("self-written glider must decode");
        prop_assert_eq!(back, glider);
    }
}
