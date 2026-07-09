//! EGM96 geoid undulation for Updraft.
//!
//! A GNSS fix gives height above the smooth WGS84 **ellipsoid**, but
//! airspace, terrain, charts, and altimetry are all referenced to **mean
//! sea level** (MSL / orthometric height). The two differ by the geoid
//! undulation *N* (the height of the geoid above the ellipsoid) which ranges
//! from about −107 m to +85 m across the Earth. This crate looks up *N*
//! from the EGM96 model so the two altitude frames can be converted:
//!
//! ```text
//! msl         = ellipsoidal − N
//! ellipsoidal = msl         + N
//! ```
//!
//! [`undulation()`] returns *N* at a position, [`ellipsoidal_to_msl()`] and
//! [`msl_to_ellipsoidal()`] apply it in the two directions.
//!
//! The lookup is a bilinear interpolation over a 1°×1° grid embedded in
//! the binary (~64 KB), downsampled from the official 15′ `WW15MGH` grid.

#[cfg(feature = "gen")]
pub mod downsample;
mod embedded;

pub use embedded::{ellipsoidal_to_msl, msl_to_ellipsoidal, undulation};
