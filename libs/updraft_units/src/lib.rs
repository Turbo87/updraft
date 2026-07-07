//! Typed physical quantities for Updraft.
//!
//! Each quantity is a newtype around an `f64` stored in its SI base unit
//! (meters, meters per second, radians, …), so values can never be mixed
//! up with plain numbers or with each other. Conversions happen only at
//! the boundaries: constructors (`Length::from_feet(…)`) and accessors
//! (`length.as_kilometers()`). The `Debug` implementations render the
//! value with its SI unit (degrees for angles) for readable logs and test
//! output; user-facing formatting with configurable display units is a UI
//! concern and comes later with the `units-settings` roadmap step.
//!
//! The set of quantities is intentionally minimal (length, speed, angle);
//! pressure, mass, temperature, etc. will be added when features need
//! them.
