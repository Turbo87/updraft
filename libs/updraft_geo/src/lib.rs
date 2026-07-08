//! Geographic types and geodesy for Updraft.
//!
//! The crate supplies the data structures and geodesic algorithms shared
//! by everything that deals with positions: a [`LatLon`] coordinate pair
//! built on the [`updraft_units`] quantities, and an antimeridian-aware
//! [`BoundingBox`]. Distances, bearings, and destination points are
//! solved on the WGS84 ellipsoid via [`geographiclib_rs`] (Karney's
//! algorithm, the same earth model FAI, OLC, and `WeGlide` score on),
//! with a spherical haversine fast path for uses that tolerate up to
//! ~0.5% error.
//!
//! Parsing and formatting of coordinate strings is deliberately out of
//! scope: every data format (NMEA, IGC, CUP, `OpenAir`, …) owns the
//! parsing of its wire representation, and user-facing display
//! formatting is a UI concern.
//!
//! With the `approx` feature, [`LatLon`] and [`BoundingBox`] implement
//! the `approx` crate's `AbsDiffEq` and `RelativeEq` traits, comparing
//! the underlying radians with an `f64` tolerance. This is meant for
//! approximate equality assertions in tests (`assert_abs_diff_eq!`,
//! `assert_relative_eq!`), where floating point math makes exact
//! comparisons too strict.
//!
//! With the `geo-types` feature, [`LatLon`] converts to and from
//! `geo_types` points and coords.

mod bounding_box;
#[cfg(feature = "geo-types")]
mod convert;
mod lat_lon;

pub use bounding_box::BoundingBox;
pub use lat_lon::LatLon;
