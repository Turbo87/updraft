use serde::{Deserialize, Serialize};

use crate::flight::{self, PositionFix, TraceStats};
use crate::job::Epoch;
use crate::time::MonotonicTime;

/// A recorded event or request that may change authoritative state.
///
/// Inputs cover four sources: user commands from a host transport,
/// normalized sensor observations, monotonic clock advancement, and
/// completed effect and computation results. The runtime feeds them to
/// [`App::handle`](crate::App::handle) one at a time. The recorded input
/// sequence is exactly what the core observed.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Input {
    /// The process clock advanced to `now`. Delivered by the runtime when
    /// the timer set from [`Update::next_deadline`] expires.
    Clock { now: MonotonicTime },
    /// A user command from a host transport.
    Command(Command),
    /// A normalized sensor observation stamped by its adapter.
    Observation(Observation),
    /// A completed compute job, returned by a runtime worker.
    ComputeResult(ComputeResult),
    /// A failed compute job. Without this input the core could keep
    /// waiting forever for a job that has stopped.
    ComputeFailed(ComputeFailure),
}

/// A user command that mutates authoritative state.
///
/// Commands become recorded inputs. Read-only requests use [`Query`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Command {
    /// Discards the flown trace and its statistics, invalidating all
    /// in-flight trace computations.
    ClearTrace,
}

/// A normalized sensor observation carrying its source timestamp.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum Observation {
    /// An own-position fix from a positioning source.
    Position(PositionFix),
}

/// A read-only request against current state. Never recorded.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Query {
    /// The current own-position fix.
    Position,
    /// The most recent trace statistics.
    TraceStats,
}

/// The answer to a [`Query`].
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum QueryResult {
    Position(Option<PositionFix>),
    TraceStats(Option<TraceStats>),
}

/// A client-visible state update, published to all state-stream
/// subscribers in input order.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Change {
    /// The own-position last-value update.
    Position(PositionFix),
    /// New trace statistics, or `None` after the trace was cleared.
    TraceStats(Option<TraceStats>),
}

/// A request for work outside the core.
///
/// Effects keep I/O and expensive computation out of the core while the
/// core still decides when that work is needed. The runtime executes
/// effects and handles failures. Effects that need a result return a
/// typed [`Input`].
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Effect {
    /// Runs an expensive calculation on a runtime compute worker. The
    /// result returns as [`Input::ComputeResult`] or
    /// [`Input::ComputeFailed`].
    Compute(ComputeJob),
}

/// One expensive calculation, carrying a snapshot of everything it needs.
///
/// The calculation itself is pure core code ([`ComputeJob::run`]). The
/// runtime only decides where it executes.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ComputeJob {
    /// Statistics over the flown trace.
    TraceStats {
        epoch: Epoch,
        fixes: Vec<PositionFix>,
    },
}

impl ComputeJob {
    pub fn kind(&self) -> ComputeKind {
        match self {
            Self::TraceStats { .. } => ComputeKind::TraceStats,
        }
    }

    /// The epoch the job was started under. The core rejects results from
    /// an older epoch. The runtime resets worker caches on a new epoch.
    pub fn epoch(&self) -> Epoch {
        match self {
            Self::TraceStats { epoch, .. } => *epoch,
        }
    }

    /// Runs the calculation to completion.
    ///
    /// This is a pure function: it uses only the data carried by the job,
    /// so the runtime can execute it on any worker thread and CI can rerun
    /// it to verify recorded results.
    pub fn run(self) -> ComputeResult {
        match self {
            Self::TraceStats { epoch, fixes } => ComputeResult::TraceStats {
                epoch,
                stats: flight::trace_stats(&fixes),
            },
        }
    }
}

/// The kind of a compute job. Each kind has one runtime worker that runs
/// at most one job at a time.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ComputeKind {
    TraceStats,
}

/// A completed compute job, entering the core as an ordinary input.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum ComputeResult {
    TraceStats { epoch: Epoch, stats: TraceStats },
}

/// A failed compute job (a worker error or panic), entering the core as
/// an ordinary input so the job slot never waits forever.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ComputeFailure {
    pub kind: ComputeKind,
    pub epoch: Epoch,
    pub message: String,
}

/// The shared current state for a newly subscribing client.
///
/// The snapshot carries current shared values and active sets only, so
/// reconnecting stays cheap for the entire flight. Datasets, histories,
/// and display-specific configuration use queries or resources instead.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Snapshot {
    pub position: Option<PositionFix>,
    pub trace_stats: Option<TraceStats>,
}

/// Everything produced by handling one input.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Update {
    /// Client-visible state updates, published in input order.
    pub changes: Vec<Change>,
    /// Requests for outside work, executed by the runtime.
    pub effects: Vec<Effect>,
    /// When the runtime must deliver the next [`Input::Clock`], if ever.
    pub next_deadline: Option<MonotonicTime>,
}
