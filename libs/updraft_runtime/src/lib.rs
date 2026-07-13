//! The shared host runtime around the deterministic core.
//!
//! Each runtime owns one [`App`](updraft_core::App), a bounded input queue,
//! a monotonic clock, and state-stream subscribers. Hosts add transport or
//! platform bindings.

mod clock;
mod metrics;
mod runtime;

pub use clock::Clock;
pub use metrics::Metrics;
pub use runtime::{ChangeFilter, Handle, Runtime, RuntimeBuilder, RuntimeStopped, Subscription};
