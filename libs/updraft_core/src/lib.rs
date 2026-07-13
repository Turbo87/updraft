//! The deterministic Updraft core.
//!
//! The core is a plain Rust library that owns shared application state and
//! the decisions based on that state. It performs no I/O, spawns no threads,
//! and reads no clocks.

mod app;
pub mod flight;
mod job;
mod protocol;
mod time;

pub use app::{App, AppConfig, Query};
pub use job::ComputeRevision;
pub use protocol::{
    Change, ComputeFailure, ComputeJob, ComputeKind, ComputeResult, Effect, Input, Snapshot, Update,
};
