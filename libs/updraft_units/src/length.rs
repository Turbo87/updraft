use crate::macros::{impl_debug_with_unit, impl_quantity_ops};

pub(crate) const METERS_PER_KILOMETER: f64 = 1000.;
pub(crate) const METERS_PER_FOOT: f64 = 0.3048;
pub(crate) const METERS_PER_NAUTICAL_MILE: f64 = 1852.;
pub(crate) const METERS_PER_STATUTE_MILE: f64 = 1609.344;

/// A length, stored internally in meters.
///
/// Used for distances, altitudes, and heights alike.
#[derive(Clone, Copy, Default, PartialEq, PartialOrd)]
#[cfg_attr(any(test, feature = "approx"), derive(approx::RelativeEq))]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Length(f64);

impl Length {
    pub const fn from_meters(meters: f64) -> Self {
        Self(meters)
    }

    pub const fn from_kilometers(kilometers: f64) -> Self {
        Self(kilometers * METERS_PER_KILOMETER)
    }

    pub const fn from_feet(feet: f64) -> Self {
        Self(feet * METERS_PER_FOOT)
    }

    pub const fn from_nautical_miles(nautical_miles: f64) -> Self {
        Self(nautical_miles * METERS_PER_NAUTICAL_MILE)
    }

    pub const fn from_statute_miles(statute_miles: f64) -> Self {
        Self(statute_miles * METERS_PER_STATUTE_MILE)
    }

    pub const fn as_meters(self) -> f64 {
        self.0
    }

    pub const fn as_kilometers(self) -> f64 {
        self.0 / METERS_PER_KILOMETER
    }

    pub const fn as_feet(self) -> f64 {
        self.0 / METERS_PER_FOOT
    }

    pub const fn as_nautical_miles(self) -> f64 {
        self.0 / METERS_PER_NAUTICAL_MILE
    }

    pub const fn as_statute_miles(self) -> f64 {
        self.0 / METERS_PER_STATUTE_MILE
    }
}

impl_quantity_ops!(Length);
impl_debug_with_unit!(Length, " m");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conversions() {
        assert_eq!(Length::from_feet(1.).as_meters(), 0.3048);
        assert_eq!(Length::from_nautical_miles(1.).as_meters(), 1852.);
        assert_eq!(Length::from_statute_miles(1.).as_meters(), 1609.344);
        assert_eq!(Length::from_kilometers(1.5).as_meters(), 1500.);
        assert_eq!(Length::from_meters(1852.).as_nautical_miles(), 1.);
    }

    #[test]
    fn arithmetic() {
        let a = Length::from_meters(100.);
        let b = Length::from_meters(50.);
        assert_eq!(a + b, Length::from_meters(150.));
        assert_eq!(a - b, Length::from_meters(50.));
        assert_eq!(-a, Length::from_meters(-100.));
        assert_eq!(a * 2., Length::from_meters(200.));
        assert_eq!(2. * a, Length::from_meters(200.));
        assert_eq!(a / 2., Length::from_meters(50.));
        assert_eq!(a / b, 2.);
        assert!(b < a);
        assert_eq!(Length::from_meters(-3.).abs(), Length::from_meters(3.));

        let mut c = a;
        c += b;
        c -= Length::from_meters(25.);
        assert_eq!(c, Length::from_meters(125.));
    }

    #[test]
    fn debug() {
        assert_eq!(format!("{:?}", Length::from_meters(1234.5)), "1234.5 m");
        assert_eq!(format!("{:.0?}", Length::from_meters(1234.4)), "1234 m");
    }

    #[cfg(feature = "serde")]
    #[test]
    fn serde() {
        let length = Length::from_meters(1234.5);
        let json = serde_json::to_string(&length).unwrap();
        assert_eq!(json, "1234.5");
        assert_eq!(serde_json::from_str::<Length>(&json).unwrap(), length);
    }
}
