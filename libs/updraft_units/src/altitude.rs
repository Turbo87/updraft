//! Reference-qualified altitude types.
//!
//! Absolute altitude values need a vertical reference. [`MslAltitude`] uses
//! mean sea level, while [`EllipsoidAltitude`] uses the WGS84 ellipsoid.

use crate::Length;

/// An altitude above mean sea level (i.e. the geoid).
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
#[cfg_attr(any(test, feature = "approx"), derive(approx::RelativeEq))]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct MslAltitude(Length);

impl MslAltitude {
    pub const ZERO: Self = Self(Length::ZERO);

    pub const fn new(length: Length) -> Self {
        Self(length)
    }

    pub const fn into_inner(self) -> Length {
        self.0
    }
}

/// An altitude above the WGS84 ellipsoid.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
#[cfg_attr(any(test, feature = "approx"), derive(approx::RelativeEq))]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct EllipsoidAltitude(Length);

impl EllipsoidAltitude {
    pub const fn new(length: Length) -> Self {
        Self(length)
    }

    pub const fn into_inner(self) -> Length {
        self.0
    }
}
