//! Aircraft preset catalogue for Updraft.
//!
//! A *preset* is a read-only catalogue entry describing an aircraft type:
//! a base model, its build/propulsion variants, and each variant's
//! wingspan configurations (see [`AircraftPreset`]). The catalogue ships
//! built-in as embedded data ([`PRESETS`]). User aircraft *profiles*
//! (created by copying a preset or from scratch, with per-field
//! overrides) are a separate, owned concept and live elsewhere.
//!
//! The polar *math* lives in [`updraft_polar`]; this crate owns only the
//! catalogue and depends on `updraft_polar` for the coefficient types.
//!
//! The catalogue data lives as per-base-model TOML files under `data/`;
//! `build.rs` turns them into the [`PRESETS`] `const` at build time.

mod model;

pub use model::{
    AircraftPreset, CgLimits, FlapSpeedRange, Polar, Propulsion, Variant, WingspanConfig,
};

mod catalogue {
    //! The `PRESETS` const, generated from `data/*.toml` by `build.rs`.
    include!(concat!(env!("OUT_DIR"), "/catalogue.rs"));
}

pub use catalogue::PRESETS;

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_abs_diff_eq;

    /// Every wingspan configuration in the catalogue that carries a
    /// polar, paired with its variant name for diagnostics and lookup.
    fn polars() -> impl Iterator<Item = (&'static str, Polar)> {
        PRESETS
            .iter()
            .flat_map(|preset| preset.variants.iter())
            .flat_map(|variant| {
                variant
                    .wingspans
                    .iter()
                    .map(move |leaf| (variant.name, leaf.polar))
            })
            .filter_map(|(name, polar)| polar.map(|polar| (name, polar)))
    }

    #[test]
    fn sorted_by_name() {
        for pair in PRESETS.windows(2) {
            assert!(pair[0].name <= pair[1].name, "{:?}", pair[1].name);
        }
    }

    #[test]
    fn every_preset_has_a_plausible_seat_count() {
        for preset in PRESETS {
            assert!(
                (1..=2).contains(&preset.seats),
                "{}: implausible seat count {}",
                preset.name,
                preset.seats
            );
        }
    }

    #[test]
    fn every_preset_has_a_variant_with_a_wingspan() {
        for preset in PRESETS {
            assert!(!preset.variants.is_empty(), "{}: no variants", preset.name);
            for variant in preset.variants {
                assert!(
                    !variant.wingspans.is_empty(),
                    "{}: variant {} has no wingspans",
                    preset.name,
                    variant.name
                );
            }
        }
    }

    #[test]
    fn polars_are_sane() {
        for (name, polar) in polars() {
            let polar = polar.glide_polar();

            let min_sink_speed = polar.min_sink_speed().as_kilometers_per_hour();
            let min_sink_rate = polar.min_sink_rate().as_meters_per_second();
            let best_glide_speed = polar.best_glide_speed().as_kilometers_per_hour();
            let best_glide_ratio = polar.best_glide_ratio();

            // Plausibility bands covering everything from the SGS 1-26
            // to an EB 29R. The quadratic model is only valid between
            // roughly minimum sink speed and maximum speed, so the
            // extrapolated vertex can sit below the real stall speed.
            assert!(
                (40. ..110.).contains(&min_sink_speed),
                "{name}: min sink speed {min_sink_speed} km/h",
            );
            assert!(
                (0.25..1.2).contains(&min_sink_rate),
                "{name}: min sink rate {min_sink_rate} m/s",
            );
            assert!(
                (60. ..130.).contains(&best_glide_speed),
                "{name}: best glide speed {best_glide_speed} km/h",
            );
            assert!(
                (20. ..80.).contains(&best_glide_ratio),
                "{name}: best glide ratio {best_glide_ratio}",
            );
            assert!(best_glide_speed > min_sink_speed, "{name}");
        }
    }

    #[test]
    fn wing_areas_are_sane() {
        for preset in PRESETS {
            for leaf in preset.wingspans() {
                if let Some(wing_area) = leaf.wing_area {
                    let wing_area = wing_area.as_square_meters();
                    assert!(
                        (5. ..25.).contains(&wing_area),
                        "{}: {wing_area} m²",
                        preset.name
                    );
                }
            }
        }
    }

    #[test]
    fn inheritance_is_resolved_by_the_build_script() {
        // `Arcus M` in data/arcus.toml carries no polar of its own; it
        // `inherits_from` `Arcus`, so build.rs must copy the coefficients
        // and reference mass onto it.
        let (_, polar) = polars()
            .find(|(name, _)| *name == "Arcus M")
            .expect("Arcus M should carry an inherited polar");
        let coefficients = polar.coefficients();
        let arcus = polars().find(|(name, _)| *name == "Arcus").unwrap().1;
        assert_eq!(coefficients, arcus.coefficients());
    }

    #[test]
    fn matches_stated_performance() {
        // Spot checks against the performance values stated alongside
        // the coefficients in the source list.
        #[track_caller]
        fn check(name: &str, best_ld: f64, at_kmh: f64, min_sink: f64, at_min_sink_kmh: f64) {
            let (_, polar) = polars().find(|(entry, _)| *entry == name).unwrap();
            let polar = polar.glide_polar();
            assert_abs_diff_eq!(polar.best_glide_ratio(), best_ld, epsilon = 0.1);
            assert_abs_diff_eq!(
                polar.best_glide_speed().as_kilometers_per_hour(),
                at_kmh,
                epsilon = 1.
            );
            assert_abs_diff_eq!(
                polar.min_sink_rate().as_meters_per_second(),
                min_sink,
                epsilon = 0.001
            );
            assert_abs_diff_eq!(
                polar.min_sink_speed().as_kilometers_per_hour(),
                at_min_sink_kmh,
                epsilon = 1.
            );
        }

        check("Antares 18S", 54.3, 112., 0.54, 99.);
        check("LS 4", 40.4, 104., 0.654, 86.);
        check("ASK 21", 33.9, 98., 0.735, 82.);
        check("Nimbus 4", 59.6, 94., 0.387, 72.);
    }
}
