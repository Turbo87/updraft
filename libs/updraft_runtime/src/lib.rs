//! The shared host runtime around the deterministic core.
//!
//! Each runtime owns one [`App`](updraft_core::App), a bounded input queue,
//! and a monotonic clock. Hosts add transport or platform bindings.

mod clock;
mod runtime;

pub use clock::Clock;
pub use runtime::{Handle, Runtime, RuntimeBuilder, RuntimeStopped};
