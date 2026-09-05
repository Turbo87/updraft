//! Glide polar model for Updraft.
//!
//! A glide polar describes a glider's still-air sink rate as a function
//! of airspeed. This crate models it with the classic quadratic
//! approximation, built from raw coefficients or fitted through three
//! published `(speed, sink)` points ([`PolarCoefficients`]), adjusts it
//! for wing loading (ballast) and bug contamination ([`GlidePolar`]),
//! and derives the values a glide computer needs: minimum sink, best
//! glide, MacCready speed to fly, and the classic MacCready
//! cross-country speed. A built-in library of 200+ glider polars ships
//! as [`POLAR_STORE`].

mod atmosphere;
mod coefficients;
mod glide;
mod glide_polar;
mod store;

pub use atmosphere::isa_density_ratio;
pub use coefficients::PolarCoefficients;
pub use glide::GlideSolution;
pub use glide_polar::GlidePolar;
pub use store::{POLAR_STORE, PolarStoreEntry};
