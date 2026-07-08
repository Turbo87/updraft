use crate::macros::{impl_debug_with_unit, impl_quantity_ops};

pub(crate) const PASCALS_PER_HECTOPASCAL: f64 = 100.;
pub(crate) const PASCALS_PER_INCH_OF_MERCURY: f64 = 3386.389;

/// A pressure, stored internally in pascals.
///
/// Used for barometric quantities such as QNH and static pressure.
/// Aviation altimeter settings are usually expressed in hectopascals
/// (equivalently millibars) or inches of mercury, so those are the
/// conversion units offered alongside the SI pascal.
#[derive(Clone, Copy, Default, PartialEq, PartialOrd)]
#[cfg_attr(any(test, feature = "approx"), derive(approx::RelativeEq))]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Pressure(f64);

impl Pressure {
    pub const fn from_pascals(pascals: f64) -> Self {
        Self(pascals)
    }

    pub const fn from_hectopascals(hectopascals: f64) -> Self {
        Self(hectopascals * PASCALS_PER_HECTOPASCAL)
    }

    /// Millibars are numerically identical to hectopascals.
    pub const fn from_millibars(millibars: f64) -> Self {
        Self::from_hectopascals(millibars)
    }

    pub const fn from_inches_of_mercury(inches_of_mercury: f64) -> Self {
        Self(inches_of_mercury * PASCALS_PER_INCH_OF_MERCURY)
    }

    pub const fn as_pascals(self) -> f64 {
        self.0
    }

    pub const fn as_hectopascals(self) -> f64 {
        self.0 / PASCALS_PER_HECTOPASCAL
    }

    /// Millibars are numerically identical to hectopascals.
    pub const fn as_millibars(self) -> f64 {
        self.as_hectopascals()
    }

    pub const fn as_inches_of_mercury(self) -> f64 {
        self.0 / PASCALS_PER_INCH_OF_MERCURY
    }
}

impl_quantity_ops!(Pressure);
impl_debug_with_unit!(Pressure, " Pa");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conversions() {
        assert_eq!(Pressure::from_hectopascals(1013.25).as_pascals(), 101_325.);
        assert_eq!(Pressure::from_millibars(1013.25).as_hectopascals(), 1013.25);
        assert_eq!(Pressure::from_inches_of_mercury(1.).as_pascals(), 3386.389);
        assert_eq!(Pressure::from_pascals(101_325.).as_hectopascals(), 1013.25);
    }

    #[test]
    fn arithmetic() {
        let a = Pressure::from_hectopascals(1000.);
        let b = Pressure::from_hectopascals(13.25);
        assert_eq!(a + b, Pressure::from_hectopascals(1013.25));
        assert_eq!(a - b, Pressure::from_hectopascals(986.75));
        assert_eq!(-b, Pressure::from_hectopascals(-13.25));
        assert_eq!(a * 2., Pressure::from_hectopascals(2000.));
        assert_eq!(a / b, 1000. / 13.25);
        assert_eq!(
            Pressure::from_pascals(-3.).abs(),
            Pressure::from_pascals(3.)
        );
        assert!(b < a);
    }

    #[test]
    fn debug() {
        assert_eq!(
            format!("{:?}", Pressure::from_hectopascals(1013.25)),
            "101325 Pa"
        );
    }

    #[cfg(feature = "serde")]
    #[test]
    fn serde() {
        let pressure = Pressure::from_hectopascals(1013.25);
        let json = serde_json::to_string(&pressure).unwrap();
        assert_eq!(json, "101325.0");
        assert_eq!(serde_json::from_str::<Pressure>(&json).unwrap(), pressure);
    }
}
