//! The deterministic Updraft core.
//!
//! The core owns shared application state and the decisions based on it.
//! It performs no I/O, spawns no threads, and reads no clocks.

#[cfg(feature = "ts")]
pub mod bindings;
mod connection;
mod connection_diagnostics;
mod core;
mod decoder;
mod effect;
mod fix;
mod input;
mod time;
mod topic;

pub use connection::{ConnectionId, ConnectionSpec, ConnectionState};
pub use core::{Core, CoreConfig};
pub use decoder::Decoder;
pub use effect::Effect;
pub use fix::Fix;
pub use input::Input;
pub use time::Timestamp;
pub use topic::{Instruments, LatLon, Topic};
