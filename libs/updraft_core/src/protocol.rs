use std::time::Duration;

use crate::flight;
use crate::job::ComputeRevision;

/// A recorded event or request that may change shared state.
#[derive(Clone, Debug, PartialEq)]
pub enum Input {
    /// Advances the core's clock.
    Clock {
        /// Time since the runtime started.
        clock_time: Duration,
    },
    /// An input owned by the flight domain.
    Flight(flight::Input),
    /// A completed compute job.
    ComputeResult(ComputeResult),
}

/// A client-visible state update produced after handling an input.
#[derive(Clone, Debug, PartialEq)]
pub enum Change {
    Flight(flight::Change),
}

/// A request for work outside the core.
#[derive(Clone, Debug, PartialEq)]
pub enum Effect {
    /// Runs an expensive calculation on a runtime compute worker.
    Compute(ComputeJob),
}

/// One expensive calculation, carrying a snapshot of everything it needs.
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

    pub fn revision(&self) -> ComputeRevision {
        match self {
            Self::Flight(job) => job.revision(),
        }
    }

    pub fn run(self) -> ComputeResult {
        match self {
            Self::Flight(job) => ComputeResult::Flight(job.run()),
        }
    }
}

/// Identifies a compute-job kind.
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
    /// Requests for outside work, executed by a host runtime.
    pub effects: Vec<Effect>,
}
