//! The small typed flow between hosts and the core.
//!
//! Everything that can change authoritative state enters as an [`Input`].
//! Read-only requests are [`Query`] values and never enter the input log.
//! Handling an input produces client-visible [`Change`] values and
//! [`Effect`] requests for work outside the core. A newly subscribing
//! client first receives a [`Snapshot`].

use serde::{Deserialize, Serialize};

use crate::compute::{ComputeFailure, ComputeJob, ComputeResult, Reduction};
use crate::time::MonotonicTime;

/// A normalized sample observation carrying its monotonic timestamp.
///
/// Adapters stamp `observed_at` from the runtime's clock, which is how
/// time enters the core as data.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Sample {
    /// The observed value.
    pub value: u32,
    /// When the observation was stamped on the process timeline.
    pub observed_at: MonotonicTime,
}

/// A recorded event or request that may change authoritative state.
///
/// Inputs are the core's only mutation entry point and the exact sequence
/// captured for deterministic replay.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Input {
    /// A normalized sample observation.
    Observe(Sample),
    /// Monotonic clock advancement delivered by the runtime.
    Tick(MonotonicTime),
    /// A worker job finished successfully.
    ComputeCompleted(ComputeResult),
    /// A worker job stopped without a result.
    ComputeFailed(ComputeFailure),
}

/// A read-only request against current state. Queries are never recorded.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Query {
    /// How many samples have been observed.
    SampleCount,
    /// The most recent reduction the worker produced.
    LatestReduction,
}

/// The answer to a [`Query`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum QueryResult {
    /// The number of samples observed.
    SampleCount(usize),
    /// The most recent reduction, if any has been computed.
    LatestReduction(Option<Reduction>),
}

/// A client-visible state update produced after handling an input.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Change {
    /// The sample count changed (the cheap, synchronous side).
    Samples(usize),
    /// A new reduction was accepted from the worker.
    Computed(Reduction),
}

/// A request for work outside the core.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Effect {
    /// Run an expensive computation on a worker and return its outcome as
    /// an input.
    Compute(ComputeJob),
}

/// The shared current state handed to a newly subscribing client.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Snapshot {
    /// How many samples have been observed.
    pub sample_count: usize,
    /// The most recent reduction, if any has been computed.
    pub latest: Option<Reduction>,
}

/// The outcome of [`App::handle`](crate::App::handle): the changes to
/// broadcast, the effects to execute, and when the core next needs a clock
/// input.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Update {
    /// Client-visible changes, in the order they were produced.
    pub changes: Vec<Change>,
    /// Work for the runtime to execute outside the core.
    pub effects: Vec<Effect>,
    /// The earliest pending timer deadline, or `None` when no timer is set.
    pub next_deadline: Option<MonotonicTime>,
}
