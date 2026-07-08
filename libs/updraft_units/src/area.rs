use crate::length::METERS_PER_FOOT;
use crate::macros::{impl_debug_with_unit, impl_quantity_ops};

const SQUARE_METERS_PER_SQUARE_FOOT: f64 = METERS_PER_FOOT * METERS_PER_FOOT;

/// An area, stored internally in square meters.
///
/// Used for wing areas.
#[derive(Clone, Copy, Default, PartialEq, PartialOrd)]
#[cfg_attr(any(test, feature = "approx"), derive(approx::RelativeEq))]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Area(f64);

impl Area {
    pub const fn from_square_meters(square_meters: f64) -> Self {
        Self(square_meters)
    }

    pub const fn from_square_feet(square_feet: f64) -> Self {
        Self(square_feet * SQUARE_METERS_PER_SQUARE_FOOT)
    }

    pub const fn as_square_meters(self) -> f64 {
        self.0
    }

    pub const fn as_square_feet(self) -> f64 {
        self.0 / SQUARE_METERS_PER_SQUARE_FOOT
    }
}

impl_quantity_ops!(Area);
impl_debug_with_unit!(Area, " m²");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conversions() {
        assert_eq!(Area::from_square_meters(10.5).as_square_meters(), 10.5);
        assert_eq!(Area::from_square_feet(1.).as_square_meters(), 0.09290304);
        assert_eq!(Area::from_square_meters(0.09290304).as_square_feet(), 1.);
    }

    #[test]
    fn arithmetic() {
        let a = Area::from_square_meters(10.);
        let b = Area::from_square_meters(4.);
        assert_eq!(a + b, Area::from_square_meters(14.));
        assert_eq!(a - b, Area::from_square_meters(6.));
        assert_eq!(-b, Area::from_square_meters(-4.));
        assert_eq!(a * 2., Area::from_square_meters(20.));
        assert_eq!(a / b, 2.5);
        assert_eq!(
            Area::from_square_meters(-3.).abs(),
            Area::from_square_meters(3.)
        );
        assert!(b < a);
    }

    #[test]
    fn debug() {
        assert_eq!(format!("{:?}", Area::from_square_meters(10.5)), "10.5 m²");
        assert_eq!(format!("{:.0?}", Area::from_square_meters(10.5)), "10 m²");
    }

    #[cfg(feature = "serde")]
    #[test]
    fn serde() {
        let area = Area::from_square_meters(10.5);
        let json = serde_json::to_string(&area).unwrap();
        assert_eq!(json, "10.5");
        assert_eq!(serde_json::from_str::<Area>(&json).unwrap(), area);
    }
}
