//! The deterministic Updraft core.
//!
//! The core is a plain Rust library that owns shared application state and
//! the decisions based on that state. It performs no I/O, spawns no threads,
//! and reads no clocks.

mod app;
mod protocol;

pub use app::App;
pub use protocol::{Input, Update};
