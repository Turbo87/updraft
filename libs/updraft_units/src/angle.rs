use std::f64::consts::{PI, TAU};
use std::fmt;

use crate::macros::impl_quantity_ops;

/// An angle (bearing, track, heading, …), stored internally in radians
/// so that it can be used in calculations without conversions.
///
/// Angles are not normalized automatically; use [`Angle::normalized`] or
/// [`Angle::normalized_signed`] where a canonical range is needed.
///
/// With the `serde` feature, an angle (de)serializes as **degrees**, not
/// the internal radians, matching the human-facing convention used by the
/// rest of the API (`from_degrees`, `as_degrees`, `Debug`).
#[derive(Clone, Copy, Default, PartialEq, PartialOrd)]
pub struct Angle(f64);

impl Angle {
    pub const fn from_radians(radians: f64) -> Self {
        Self(radians)
    }

    pub const fn from_degrees(degrees: f64) -> Self {
        Self(degrees.to_radians())
    }

    pub const fn as_radians(self) -> f64 {
        self.0
    }

    pub const fn as_degrees(self) -> f64 {
        self.0.to_degrees()
    }

    /// Normalizes into the compass range `[0, 2π)` (i.e. `[0°, 360°)`).
    pub fn normalized(self) -> Self {
        // `rem_euclid` can round up to exactly `TAU` for tiny negative
        // inputs (the true remainder is below `TAU`'s ULP), which would
        // escape the half-open range; clamp that back to zero.
        let radians = self.0.rem_euclid(TAU);
        Self(if radians < TAU { radians } else { 0. })
    }

    /// Normalizes into the signed range `(-π, π]` (i.e. `(-180°, 180°]`),
    /// e.g. for relative bearings and turn directions.
    pub fn normalized_signed(self) -> Self {
        let radians = self.0.rem_euclid(TAU);
        Self(if radians > PI { radians - TAU } else { radians })
    }

    pub fn sin(self) -> f64 {
        self.0.sin()
    }

    pub fn cos(self) -> f64 {
        self.0.cos()
    }

    pub fn sin_cos(self) -> (f64, f64) {
        self.0.sin_cos()
    }
}

impl_quantity_ops!(Angle);

impl fmt::Debug for Angle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.as_degrees(), f)?;
        f.write_str("°")
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for Angle {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_f64(self.as_degrees())
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for Angle {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        f64::deserialize(deserializer).map(Self::from_degrees)
    }
}

#[cfg(test)]
mod tests {
    use std::f64::consts::FRAC_PI_2;

    use super::*;

    #[track_caller]
    fn assert_close(actual: Angle, expected_radians: f64) {
        let actual = actual.as_radians();
        assert!(
            (actual - expected_radians).abs() < 1e-12,
            "{actual} != {expected_radians}"
        );
    }

    #[test]
    fn conversions() {
        assert_eq!(Angle::from_degrees(180.).as_radians(), PI);
        assert_eq!(Angle::from_radians(FRAC_PI_2).as_degrees(), 90.);
    }

    #[test]
    fn normalization() {
        assert_close(Angle::from_radians(TAU).normalized(), 0.);
        assert_close(Angle::from_radians(-FRAC_PI_2).normalized(), 1.5 * PI);
        assert_close(Angle::from_degrees(725.).normalized(), 5f64.to_radians());

        assert_close(Angle::from_degrees(270.).normalized_signed(), -FRAC_PI_2);
        assert_close(Angle::from_radians(PI).normalized_signed(), PI);
        assert_close(Angle::from_radians(-PI).normalized_signed(), PI);
        assert_close(Angle::from_radians(0.5).normalized_signed(), 0.5);
    }

    #[test]
    fn normalized_stays_below_tau() {
        // A tiny negative input makes `rem_euclid` round up to exactly
        // `TAU`; `normalized` must keep the result inside `[0, 2π)`.
        for radians in [-1e-18, -1e-30, -f64::MIN_POSITIVE] {
            let normalized = Angle::from_radians(radians).normalized().as_radians();
            assert!(normalized < TAU, "{radians:e} normalized to {normalized}");
            assert!(normalized >= 0.);
        }
    }

    #[test]
    fn trigonometry() {
        assert_eq!(Angle::from_degrees(90.).sin(), 1.);
        assert_eq!(Angle::ZERO.cos(), 1.);
        let (sin, cos) = Angle::from_degrees(90.).sin_cos();
        assert_eq!(sin, 1.);
        assert!(cos.abs() < 1e-15);
    }

    #[test]
    fn debug() {
        assert_eq!(format!("{:?}", Angle::from_degrees(270.)), "270°");
        assert_eq!(format!("{:.1?}", Angle::from_radians(PI)), "180.0°");
    }

    #[cfg(feature = "serde")]
    #[test]
    fn serde() {
        // Angles serialize as degrees, not the internal radians.
        let angle = Angle::from_degrees(90.);
        let json = serde_json::to_string(&angle).unwrap();
        assert_eq!(json, "90.0");
        assert_eq!(serde_json::from_str::<Angle>(&json).unwrap(), angle);

        // A value authored as degrees deserializes to the right angle.
        assert_eq!(
            serde_json::from_str::<Angle>("180").unwrap(),
            Angle::from_degrees(180.)
        );
    }
}
