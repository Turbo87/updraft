//! The deterministic Updraft core.
//!
//! The core is a plain Rust library that owns the authoritative flight state
//! and the decisions based on that state. It performs **no I/O, spawns no
//! threads, and reads no clocks**. A shared runtime (`updraft_runtime`)
//! owns clocks, queues, workers, and effect execution.
//!
//! The architecture has five concepts (see `docs/design/core.md`):
//!
//! - [`Input`]: a recorded event or request that may change authoritative
//!   state: a command, a sensor observation, clock advancement, or a
//!   completed effect.
//! - [`Query`]: a read-only request against current state. Queries are not
//!   inputs and are not recorded.
//! - [`Change`]: a client-visible state update produced after handling an
//!   input.
//! - [`Effect`]: a request for work outside the core, such as expensive
//!   computation or I/O.
//! - **Resource**: bulk or growing data served by reference instead of
//!   copied through the state stream (not implemented yet).
//!
//! [`App::handle()`] is the only mutation entry point. The same ordered
//! inputs produce the same state changes, so whole-flight scenario tests
//! are a plain loop over [`App::handle()`] with no async runtime, sleeps,
//! or wall clock, and a recorded input sequence replays a field failure
//! exactly.

mod app;
mod flight;
mod job;
mod protocol;
mod time;

pub use app::{App, AppConfig};
pub use flight::{PositionFix, TraceStats};
pub use job::Epoch;
pub use protocol::{
    Change, Command, ComputeFailure, ComputeJob, ComputeKind, ComputeResult, Effect, Input,
    Observation, Query, QueryResult, Snapshot, Update,
};
pub use time::MonotonicTime;
