//! The shared host runtime around the deterministic core.
//!
//! The runtime owns one [`App`](updraft_core::App), the bounded input
//! queue, the process-wide monotonic clock, compute workers, and the
//! state-stream subscribers (see `docs/design/runtime.md`). The axum
//! server and the Tauri shell use the same runtime and add only transport
//! or platform bindings.
//!
//! Most domain work does not need these details: callers submit typed
//! [`Input`](updraft_core::Input) values through a [`Handle`] and
//! subscribe to a state stream that starts with a snapshot.
//!
//! The core rules the runtime pins down:
//!
//! - inputs pass through a plain bounded FIFO: a full queue blocks the
//!   producer and never drops an input,
//! - the core never reads a clock, so after each input the runtime arms
//!   one timer for [`Update::next_deadline`](updraft_core::Update) and
//!   submits a clock input when it expires,
//! - each compute-worker kind runs at most one job at a time, and a
//!   worker panic becomes a typed
//!   [`Input::ComputeFailed`](updraft_core::Input),
//! - subscriptions capture their snapshot atomically with registration,
//!   change batches arrive in input order, and a subscriber that falls
//!   behind its bounded buffer is dropped and must resubscribe.

mod clock;
mod metrics;
mod runtime;
mod worker;

pub use clock::Clock;
pub use metrics::Metrics;
pub use runtime::{Handle, Runtime, RuntimeBuilder, RuntimeStopped, Subscription};
pub use worker::{PureWorker, Worker};
