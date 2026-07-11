//! The compute-worker contract: expensive jobs and their results.
//!
//! Cheap derived values stay synchronous inside the core. A known
//! expensive calculation instead leaves the core as an
//! [`Effect::Compute`](crate::Effect::Compute) carrying a self-contained
//! [`ComputeJob`]. The runtime runs the job off the input loop and returns
//! its outcome as an ordinary input
//! ([`Input::ComputeCompleted`](crate::Input::ComputeCompleted) or
//! [`Input::ComputeFailed`](crate::Input::ComputeFailed)).
//!
//! This module is the neutral placeholder for the real workers (live
//! scoring, glide reach). The job simply reduces a batch of samples; its
//! cost is simulated by the runtime worker. When the first invalidating
//! input lands (a flight reset or replay seek), jobs and results will gain
//! a generation `Epoch` so the core can drop work that a reset made stale;
//! that field is deliberately omitted until then.

use serde::{Deserialize, Serialize};

/// The reduced form of a batch of samples: how many there were and their
/// sum.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Reduction {
    /// The number of samples reduced.
    pub count: usize,
    /// The sum of the sample values.
    pub sum: u64,
}

/// Names the worker a job belongs to. Each kind runs at most one job at a
/// time; the variant lets a failure report which worker stopped.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ComputeKind {
    /// The demo reduction worker.
    Demo,
}

/// A self-contained unit of expensive work handed to the runtime.
///
/// The job carries a snapshot of everything it needs, so the worker never
/// reaches back into live core state.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ComputeJob {
    /// Reduce a snapshot of the accumulated samples.
    Demo {
        /// The samples to reduce.
        batch: Vec<u32>,
    },
}

impl ComputeJob {
    /// The worker kind responsible for this job.
    pub const fn kind(&self) -> ComputeKind {
        match self {
            ComputeJob::Demo { .. } => ComputeKind::Demo,
        }
    }

    /// Runs the job to completion. This is a pure function: the runtime
    /// calls it on a worker thread, never the core on the input loop, and
    /// the runtime is what simulates the job's cost.
    pub fn run(&self) -> ComputeResult {
        match self {
            ComputeJob::Demo { batch } => {
                let reduction = Reduction {
                    count: batch.len(),
                    sum: batch.iter().map(|value| u64::from(*value)).sum(),
                };
                ComputeResult::Demo { reduction }
            }
        }
    }
}

/// The successful result of a [`ComputeJob`], returned to the core as an
/// input.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ComputeResult {
    /// The reduced batch.
    Demo {
        /// The reduction the worker computed.
        reduction: Reduction,
    },
}

impl ComputeResult {
    /// The worker kind that produced this result.
    pub const fn kind(&self) -> ComputeKind {
        match self {
            ComputeResult::Demo { .. } => ComputeKind::Demo,
        }
    }
}

/// A worker that stopped without producing a result, reported to the core
/// so a domain never waits forever for a job that has died.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComputeFailure {
    /// The worker that failed.
    pub kind: ComputeKind,
    /// A human-readable description of the failure.
    pub message: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reduces_a_batch() {
        let job = ComputeJob::Demo {
            batch: vec![1, 2, 3, 4],
        };
        assert_eq!(job.kind(), ComputeKind::Demo);
        assert_eq!(
            job.run(),
            ComputeResult::Demo {
                reduction: Reduction { count: 4, sum: 10 }
            }
        );
    }

    #[test]
    fn reduces_an_empty_batch_to_zero() {
        let job = ComputeJob::Demo { batch: vec![] };
        assert_eq!(
            job.run(),
            ComputeResult::Demo {
                reduction: Reduction::default()
            }
        );
    }
}
