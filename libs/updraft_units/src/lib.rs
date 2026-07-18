//! Typed physical quantities for Updraft.
//!
//! Base quantities are newtypes around an `f64` stored in their SI base unit
//! (meters, meters per second, radians, …), so values can never be mixed up
//! with plain numbers or with each other. Reference-qualified quantities such
//! as altitude wrap a base quantity to distinguish values that have the same
//! dimension but different meanings.
//!
//! Conversions happen only at the boundaries: constructors
//! (`Length::from_feet(…)`) and accessors (`length.as_kilometers()`). The
//! `Debug` implementations render the value with its SI unit (degrees for
//! angles) for readable logs and test output; user-facing formatting with
//! configurable display units is a UI concern and comes later with the
//! `units-settings` roadmap step.
//!
//! With the `approx` feature, the quantity types implement the `approx`
//! crate's `AbsDiffEq` and `RelativeEq` traits, comparing the underlying
//! SI values with an `f64` tolerance. This is meant for approximate
//! equality assertions in tests (`assert_abs_diff_eq!`,
//! `assert_relative_eq!`), where floating point math makes exact
//! comparisons too strict.

mod altitude;
mod angle;
mod area;
mod length;
mod macros;
mod mass;
mod pressure;
mod speed;

pub use altitude::MslAltitude;
pub use angle::Angle;
pub use area::Area;
pub use length::Length;
pub use mass::Mass;
pub use pressure::Pressure;
pub use speed::Speed;
