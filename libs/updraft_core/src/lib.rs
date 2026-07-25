//! The deterministic Updraft core.
//!
//! The core owns shared application state and the decisions based on it.
//! It performs no I/O, spawns no threads, and reads no clocks.

mod connection;
mod time;

pub use connection::{ConnectionId, ConnectionSpec, ConnectionState};
pub use time::Timestamp;
