//! Glide polar model for Updraft.
//!
//! A glide polar describes a glider's still-air sink rate as a function
//! of airspeed. This crate models it with the classic quadratic
//! approximation, built from raw coefficients or fitted through three
//! published `(speed, sink)` points ([`PolarCoefficients`]), adjusts it
//! for wing loading (ballast) and bug contamination ([`GlidePolar`]),
//! and derives the values a glide computer needs: minimum sink, best
//! glide, MacCready speed to fly, and the classic MacCready
//! cross-country speed.
//!
//! This crate owns the polar *math* only. The built-in catalogue of
//! aircraft types and their polars lives in the separate
//! `updraft_aircraft_presets` crate, which depends on this one for the
//! coefficient types.

mod coefficients;
mod glide_polar;

pub use coefficients::PolarCoefficients;
pub use glide_polar::GlidePolar;
