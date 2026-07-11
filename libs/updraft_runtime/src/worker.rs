//! The compute worker: one thread per computation kind.
//!
//! The worker receives a self-contained [`ComputeJob`], spends the
//! configured "cost" delay (standing in for expensive work), runs the job
//! inside [`catch_unwind`](std::panic::catch_unwind), and feeds the outcome
//! back into the input loop as [`Input::ComputeCompleted`] or, on a panic,
//! [`Input::ComputeFailed`]. Because the core's job slot keeps at most one
//! job outstanding per kind, the job channel never backs up.

use std::any::Any;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::mpsc::Receiver;
use std::time::Duration;

use updraft_core::{ComputeFailure, ComputeJob, Input};

use crate::{ComputeFn, LoopSender};

/// A message to the worker thread.
pub(crate) enum WorkerMsg {
    /// Run this job and report the outcome.
    Run(ComputeJob),
    /// Stop the worker; the runtime is shutting down.
    Stop,
}

/// The worker thread body. Returns when told to stop or when the input loop
/// has gone away.
pub(crate) fn run(
    rx: &Receiver<WorkerMsg>,
    loop_tx: &LoopSender,
    delay: Duration,
    compute: ComputeFn,
) {
    while let Ok(WorkerMsg::Run(job)) = rx.recv() {
        // The simulated cost of the expensive work lives here, off the
        // input loop, which is the whole point of the worker.
        if !delay.is_zero() {
            std::thread::sleep(delay);
        }

        let input = match catch_unwind(AssertUnwindSafe(|| compute(&job))) {
            Ok(result) => Input::ComputeCompleted(result),
            Err(payload) => Input::ComputeFailed(ComputeFailure {
                kind: job.kind(),
                message: panic_message(payload),
            }),
        };

        // A send error means the loop has stopped; nothing left to do.
        if loop_tx.send_input(input).is_err() {
            break;
        }
    }
}

/// Extracts a human-readable message from a panic payload.
fn panic_message(payload: Box<dyn Any + Send>) -> String {
    if let Some(message) = payload.downcast_ref::<&str>() {
        (*message).to_owned()
    } else if let Some(message) = payload.downcast_ref::<String>() {
        message.clone()
    } else {
        "worker panicked".to_owned()
    }
}
