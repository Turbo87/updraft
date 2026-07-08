use crate::length::{
    METERS_PER_FOOT, METERS_PER_KILOMETER, METERS_PER_NAUTICAL_MILE, METERS_PER_STATUTE_MILE,
};
use crate::macros::{impl_debug_with_unit, impl_quantity_ops};

const SECONDS_PER_HOUR: f64 = 3600.;
const MPS_PER_KMH: f64 = METERS_PER_KILOMETER / SECONDS_PER_HOUR;
const MPS_PER_KNOT: f64 = METERS_PER_NAUTICAL_MILE / SECONDS_PER_HOUR;
const MPS_PER_MPH: f64 = METERS_PER_STATUTE_MILE / SECONDS_PER_HOUR;
const MPS_PER_FPM: f64 = METERS_PER_FOOT / 60.;

/// A speed, stored internally in meters per second.
///
/// Used for both horizontal speeds (ground speed, airspeed) and vertical
/// speeds (climb/sink rate, vario value); positive vertical speeds mean
/// climbing. Which unit a value is displayed in is a presentation concern
/// for the frontend, so a single type serves both.
#[derive(Clone, Copy, Default, PartialEq, PartialOrd)]
#[cfg_attr(any(test, feature = "approx"), derive(approx::RelativeEq))]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Speed(f64);

impl Speed {
    pub const fn from_meters_per_second(meters_per_second: f64) -> Self {
        Self(meters_per_second)
    }

    pub const fn from_kilometers_per_hour(kilometers_per_hour: f64) -> Self {
        Self(kilometers_per_hour * MPS_PER_KMH)
    }

    pub const fn from_knots(knots: f64) -> Self {
        Self(knots * MPS_PER_KNOT)
    }

    pub const fn from_miles_per_hour(miles_per_hour: f64) -> Self {
        Self(miles_per_hour * MPS_PER_MPH)
    }

    pub const fn from_feet_per_minute(feet_per_minute: f64) -> Self {
        Self(feet_per_minute * MPS_PER_FPM)
    }

    pub const fn as_meters_per_second(self) -> f64 {
        self.0
    }

    pub const fn as_kilometers_per_hour(self) -> f64 {
        self.0 / MPS_PER_KMH
    }

    pub const fn as_knots(self) -> f64 {
        self.0 / MPS_PER_KNOT
    }

    pub const fn as_miles_per_hour(self) -> f64 {
        self.0 / MPS_PER_MPH
    }

    pub const fn as_feet_per_minute(self) -> f64 {
        self.0 / MPS_PER_FPM
    }
}

impl_quantity_ops!(Speed);
impl_debug_with_unit!(Speed, " m/s");

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn conversions() {
        assert_eq!(
            Speed::from_kilometers_per_hour(3.6).as_meters_per_second(),
            1.
        );
        assert_eq!(Speed::from_knots(1.).as_meters_per_second(), 1852. / 3600.);
        assert_eq!(
            Speed::from_miles_per_hour(1.).as_meters_per_second(),
            1609.344 / 3600.
        );
        assert_eq!(
            Speed::from_feet_per_minute(1.).as_meters_per_second(),
            0.3048 / 60.
        );
        assert_eq!(
            Speed::from_meters_per_second(0.3048).as_feet_per_minute(),
            60.
        );

        let knots = Speed::from_meters_per_second(10.).as_knots();
        assert_relative_eq!(knots, 36000. / 1852.);
    }

    #[test]
    fn arithmetic() {
        let a = Speed::from_meters_per_second(30.);
        let b = Speed::from_meters_per_second(12.);
        assert_eq!(a + b, Speed::from_meters_per_second(42.));
        assert_eq!(a - b, Speed::from_meters_per_second(18.));
        assert_eq!(-b, Speed::from_meters_per_second(-12.));
        assert_eq!(a / b, 2.5);
        assert_eq!(Speed::from_meters_per_second(-3.).abs(), b / 4.);
        assert!(b < a);
    }

    #[test]
    fn debug() {
        assert_eq!(
            format!("{:?}", Speed::from_meters_per_second(34.5)),
            "34.5 m/s"
        );
        assert_eq!(
            format!("{:.1?}", Speed::from_kilometers_per_hour(3.6)),
            "1.0 m/s"
        );
        assert_eq!(
            format!("{:?}", Speed::from_meters_per_second(-1.5)),
            "-1.5 m/s"
        );
    }

    #[cfg(feature = "serde")]
    #[test]
    fn serde() {
        let speed = Speed::from_meters_per_second(34.5);
        let json = serde_json::to_string(&speed).unwrap();
        assert_eq!(json, "34.5");
        assert_eq!(serde_json::from_str::<Speed>(&json).unwrap(), speed);
    }
}
