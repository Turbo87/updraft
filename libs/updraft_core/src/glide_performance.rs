use serde::{Deserialize, Serialize};
use updraft_units::Mass;

/// Water ballast in litres, added to the polar's reference mass at 1 kg/litre.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Ballast(f64);

impl<'de> Deserialize<'de> for Ballast {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        f64::deserialize(deserializer)?
            .try_into()
            .map_err(serde::de::Error::custom)
    }
}

impl TryFrom<f64> for Ballast {
    type Error = InvalidBallast;

    fn try_from(litres: f64) -> Result<Self, Self::Error> {
        if litres.is_finite() && litres >= 0.0 {
            Ok(Self(litres))
        } else {
            Err(InvalidBallast)
        }
    }
}

#[derive(Debug, thiserror::Error)]
#[error("Ballast must be finite and nonnegative")]
pub struct InvalidBallast;

/// A finite, nonnegative MacCready value in metres per second.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct MacCready(f64);

impl MacCready {
    pub fn meters_per_second(self) -> f64 {
        self.0
    }
}

impl<'de> Deserialize<'de> for MacCready {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        f64::deserialize(deserializer)?
            .try_into()
            .map_err(serde::de::Error::custom)
    }
}

impl TryFrom<f64> for MacCready {
    type Error = InvalidMacCready;

    fn try_from(meters_per_second: f64) -> Result<Self, Self::Error> {
        if meters_per_second.is_finite() && meters_per_second >= 0.0 {
            Ok(Self(meters_per_second))
        } else {
            Err(InvalidMacCready)
        }
    }
}

#[derive(Debug, thiserror::Error)]
#[error("MacCready must be finite and nonnegative")]
pub struct InvalidMacCready;

/// The percentage of clean performance lost, from zero to below 100.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Bugs(f64);

impl<'de> Deserialize<'de> for Bugs {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        f64::deserialize(deserializer)?
            .try_into()
            .map_err(serde::de::Error::custom)
    }
}

impl TryFrom<f64> for Bugs {
    type Error = InvalidBugs;

    fn try_from(percent: f64) -> Result<Self, Self::Error> {
        if (0.0..100.0).contains(&percent) {
            Ok(Self(percent))
        } else {
            Err(InvalidBugs)
        }
    }
}

#[derive(Debug, thiserror::Error)]
#[error("Bugs must be between 0% and less than 100%")]
pub struct InvalidBugs;

/// Session controls that reset when the core starts.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct GlidePerformance {
    pub mac_cready: MacCready,
    pub bugs: Bugs,
    pub ballast: Ballast,
}

impl GlidePerformance {
    pub fn glide_polar(self, polar: crate::PolarId) -> updraft_polar::GlidePolar {
        let polar = polar.glide_polar();
        let mass = polar.reference_mass() + Mass::from_kilograms(self.ballast.0);
        polar.with_total_mass(mass).with_bugs(self.bugs.0 / 100.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use claims::{assert_err, assert_ok};

    #[test]
    fn ballast_adds_water_mass_to_each_selected_polar() {
        let performance = GlidePerformance {
            ballast: assert_ok!(Ballast::try_from(100.5)),
            bugs: assert_ok!(Bugs::try_from(10.0)),
            ..GlidePerformance::default()
        };
        for id in crate::PolarId::all() {
            let dry = id.glide_polar();
            let loaded = performance.glide_polar(id);
            let expected_mass = dry.reference_mass().as_kilograms() + 100.5;
            assert_eq!(loaded.total_mass().as_kilograms(), expected_mass);
            assert_eq!(loaded.bugs(), 0.1);
            let expected_ratio = dry.best_glide_ratio() * 0.9;
            approx::assert_abs_diff_eq!(loaded.best_glide_ratio(), expected_ratio, epsilon = 1e-10);
        }
    }

    #[test]
    fn ballast_accepts_only_finite_nonnegative_litres() {
        assert_eq!(Ballast::default().0, 0.0);
        for value in [0.0, 100.5, f64::MAX] {
            assert_eq!(assert_ok!(Ballast::try_from(value)).0, value);
        }
        for value in [-1.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert_err!(Ballast::try_from(value));
        }
    }

    #[test]
    fn bugs_reduce_the_selected_polars_performance() {
        let id = crate::PolarId::default();
        let clean = GlidePerformance::default().glide_polar(id);
        assert_eq!(clean.bugs(), 0.0);
        let performance = GlidePerformance {
            bugs: assert_ok!(Bugs::try_from(10.0)),
            ..GlidePerformance::default()
        };
        let dirty = performance.glide_polar(id);
        approx::assert_abs_diff_eq!(
            dirty.best_glide_ratio(),
            clean.best_glide_ratio() * 0.9,
            epsilon = 1e-12
        );
    }

    #[test]
    fn bugs_reject_invalid_percentages() {
        for percent in [-1.0, 100.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert_err!(Bugs::try_from(percent));
        }
        assert_ok!(Bugs::try_from(0.0));
        assert_ok!(Bugs::try_from(99.9));
    }

    #[test]
    fn maccready_accepts_only_finite_nonnegative_values() {
        assert_eq!(MacCready::default().0, 0.0);
        for value in [0.0, 1.5, f64::MAX] {
            assert_eq!(assert_ok!(MacCready::try_from(value)).0, value);
        }
        for value in [-1.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert_err!(MacCready::try_from(value));
        }
    }
}
