//! Compute-worker implementations.
//!
//! Pure computation over job inputs plus state retained between rounds.
//! The runtime owns one instance per [`JobKind`](crate::JobKind) on its
//! own thread; one-in-flight scheduling serializes all access, and an
//! epoch bump resets the state (see `docs/design/core.md`,
//! "Computation").

use updraft_geo::LatLon;
use updraft_units::Length;

use crate::protocol::{ComputeJob, Epoch, JobResult};

/// Accumulates the flown-track distance incrementally: each job carries
/// only the new track points, and the worker retains the connection
/// point and running total between rounds.
#[derive(Default)]
pub struct TrackDistanceWorker {
    epoch: Epoch,
    last: Option<LatLon>,
    total: Length,
}

impl TrackDistanceWorker {
    pub fn run(&mut self, job: &ComputeJob) -> JobResult {
        match job {
            ComputeJob::TrackDistance { epoch, points } => {
                if *epoch != self.epoch {
                    *self = Self {
                        epoch: *epoch,
                        ..Self::default()
                    };
                }
                for &point in points {
                    if let Some(last) = self.last {
                        self.total += last.distance(point);
                    }
                    self.last = Some(point);
                }
                JobResult::TrackDistance(self.total)
            }
        }
    }
}
