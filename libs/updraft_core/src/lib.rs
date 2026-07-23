//! The deterministic Updraft core.
//!
//! The core is a plain Rust library that owns shared application state and
//! the decisions based on that state. It performs **no I/O, spawns no
//! threads, and reads no clocks**. A shared runtime (`updraft_runtime`)
//! owns clocks, queues, workers, and effect execution.
//!
//! The architecture has five concepts (see `docs/design/core.md`):
//!
//! - [`Input`]: a recorded event or request that may change shared state:
//!   a command, sensor observation, clock advancement, or outcome from
//!   outside work.
//! - [`Query`]: a read-only request against current state. Queries are not
//!   inputs and are not recorded.
//! - [`Change`]: a client-visible state update produced after handling an
//!   input.
//! - [`Effect`]: a request for work outside the core, such as expensive
//!   computation or I/O.
//! - **Resource**: bulk or growing data served by reference instead of
//!   copied through the state stream.
//!
//! [`App::handle()`] applies inputs whose observation times control the clock.
//! [`App::handle_at_clock_time()`] additionally supplies their current
//! monotonic processing time. The same ordered calls produce the same state
//! changes, so whole-flight scenario tests need no async runtime, sleeps, or
//! wall clock.

mod app;
pub mod device;
pub mod flight;
mod job;
mod protocol;
mod time;

pub use app::{App, AppConfig, Query};
pub use job::ComputeRevision;
pub use protocol::{
    Change, ChangeGroup, ComputeCancellation, ComputeFailure, ComputeJob, ComputeKind,
    ComputeResult, Effect, Input, Snapshot, Update,
};
