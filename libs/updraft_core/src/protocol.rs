use std::time::Duration;

use crate::flight;
use crate::job::ComputeRevision;

/// A recorded event or request that may change shared state.
///
/// Inputs include commands, sensor observations, clock advancement, and
/// outcomes from outside work. The runtime feeds them to
/// [`App::handle()`](crate::App::handle) one at a time. The recorded input
/// sequence is exactly what the core observed.
#[derive(Clone, Debug, PartialEq)]
pub enum Input {
    /// Advances the core's clock when [`Update::next_deadline`] expires.
    Clock {
        /// Time since the runtime started.
        clock_time: Duration,
    },
    /// An input owned by the flight domain.
    Flight(flight::Input),
    /// A completed compute job, returned by a runtime worker.
    ComputeResult(ComputeResult),
    /// A failed compute job. Without this input the core could keep
    /// waiting forever for a job that has stopped.
    ComputeFailed(ComputeFailure),
}

/// A client-visible state update produced after handling an input.
#[derive(Clone, Debug, PartialEq)]
pub enum Change {
    Flight(flight::Change),
}

/// A request for work outside the core.
///
/// Effects keep I/O and expensive computation out of the core while the
/// core still decides when that work is needed. The runtime executes
/// effects and handles failures. Effects that need a result return a
/// typed [`Input`].
#[derive(Clone, Debug, PartialEq)]
pub enum Effect {
    /// Runs an expensive calculation on a runtime compute worker. The
    /// outcome returns as [`Input::ComputeResult`] or
    /// [`Input::ComputeFailed`].
    Compute(ComputeJob),
}

/// One expensive calculation, carrying a snapshot of everything it needs.
///
/// The calculation itself is pure core code ([`ComputeJob::run()`]). The
/// runtime only decides where it executes.
#[derive(Clone, Debug, PartialEq)]
pub enum ComputeJob {
    Flight(flight::ComputeJob),
}

impl ComputeJob {
    pub fn kind(&self) -> ComputeKind {
        match self {
            Self::Flight(job) => ComputeKind::Flight(job.kind()),
        }
    }

    /// The revision the job was started under. The core rejects results from
    /// older revisions. The runtime resets worker caches when it changes.
    pub fn revision(&self) -> ComputeRevision {
        match self {
            Self::Flight(job) => job.revision(),
        }
    }

    /// Runs the calculation to completion.
    ///
    /// This is a pure function: it uses only the data carried by the job,
    /// so the runtime can execute it on any worker thread and CI can rerun
    /// it to verify recorded results.
    pub fn run(self) -> ComputeResult {
        match self {
            Self::Flight(job) => ComputeResult::Flight(job.run()),
        }
    }
}

/// Identifies a compute-job kind.
///
/// The runtime permits at most one worker and one running job per kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ComputeKind {
    Flight(flight::ComputeKind),
}

/// A completed compute job, entering the core as an ordinary input.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ComputeResult {
    Flight(flight::ComputeResult),
}

impl ComputeResult {
    pub fn kind(&self) -> ComputeKind {
        match self {
            Self::Flight(result) => ComputeKind::Flight(result.kind()),
        }
    }

    pub fn revision(&self) -> ComputeRevision {
        match self {
            Self::Flight(result) => result.revision(),
        }
    }
}

/// A compute job that failed rather than completing.
///
/// It enters the core as an ordinary input so the job slot never waits
/// forever.
#[derive(Clone, Debug, PartialEq)]
pub struct ComputeFailure {
    pub kind: ComputeKind,
    pub revision: ComputeRevision,
    pub message: String,
}

/// The shared current state for a newly subscribing client.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Snapshot {
    pub flight: flight::Snapshot,
}

/// Everything produced by handling one input.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Update {
    /// Client-visible state updates, published in input order.
    pub changes: Vec<Change>,
    /// Requests for outside work, executed by the runtime.
    pub effects: Vec<Effect>,
    /// When the runtime must deliver the next [`Input::Clock`], if ever.
    pub next_deadline: Option<Duration>,
}
