use crate::macros::{impl_debug_with_unit, impl_quantity_ops};

const KILOGRAMS_PER_POUND: f64 = 0.45359237;

/// A mass, stored internally in kilograms.
///
/// Used for aircraft weights and ballast.
#[derive(Clone, Copy, Default, PartialEq, PartialOrd)]
#[cfg_attr(any(test, feature = "approx"), derive(approx::RelativeEq))]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Mass(f64);

impl Mass {
    pub const fn from_kilograms(kilograms: f64) -> Self {
        Self(kilograms)
    }

    pub const fn from_pounds(pounds: f64) -> Self {
        Self(pounds * KILOGRAMS_PER_POUND)
    }

    pub const fn as_kilograms(self) -> f64 {
        self.0
    }

    pub const fn as_pounds(self) -> f64 {
        self.0 / KILOGRAMS_PER_POUND
    }
}

impl_quantity_ops!(Mass);
impl_debug_with_unit!(Mass, " kg");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conversions() {
        assert_eq!(Mass::from_kilograms(350.).as_kilograms(), 350.);
        assert_eq!(Mass::from_pounds(1.).as_kilograms(), 0.45359237);
        assert_eq!(Mass::from_kilograms(0.45359237).as_pounds(), 1.);
    }

    #[test]
    fn arithmetic() {
        let a = Mass::from_kilograms(300.);
        let b = Mass::from_kilograms(50.);
        assert_eq!(a + b, Mass::from_kilograms(350.));
        assert_eq!(a - b, Mass::from_kilograms(250.));
        assert_eq!(-b, Mass::from_kilograms(-50.));
        assert_eq!(a * 2., Mass::from_kilograms(600.));
        assert_eq!(a / b, 6.);
        assert_eq!(Mass::from_kilograms(-3.).abs(), Mass::from_kilograms(3.));
        assert!(b < a);
    }

    #[test]
    fn debug() {
        assert_eq!(format!("{:?}", Mass::from_kilograms(361.5)), "361.5 kg");
        assert_eq!(format!("{:.0?}", Mass::from_kilograms(361.5)), "362 kg");
    }

    #[cfg(feature = "serde")]
    #[test]
    fn serde() {
        let mass = Mass::from_kilograms(361.5);
        let json = serde_json::to_string(&mass).unwrap();
        assert_eq!(json, "361.5");
        assert_eq!(serde_json::from_str::<Mass>(&json).unwrap(), mass);
    }
}
