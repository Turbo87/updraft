//! Reference-qualified altitude types.

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

    pub const fn length(self) -> Length {
        self.0
    }
}
