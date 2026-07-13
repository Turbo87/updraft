//! The shared host runtime around the deterministic core.
//!
//! Each runtime owns one [`App`](updraft_core::App), a bounded input
//! queue, a monotonic clock, compute workers, and state-stream subscribers
//! (see `docs/design/runtime.md`). The axum server and Tauri shell will use
//! this runtime and add only transport or platform bindings.
//!
//! Most domain work does not need these details: callers submit typed
//! [`Input`](updraft_core::Input) values through a [`Handle`] and
//! subscribe to a state stream that starts with a snapshot.
//!
//! The runtime guarantees:
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
pub use runtime::{ChangeFilter, Handle, Runtime, RuntimeBuilder, RuntimeStopped, Subscription};
pub use worker::{CancellationToken, PureWorker, Worker, WorkerResult};
