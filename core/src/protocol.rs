use serde::{Deserialize, Serialize};
use ts_rs::TS;
use updraft_geo::LatLon;
use updraft_units::Length;

use crate::flight::{FlightChange, FlightInput};
use crate::time::MonotonicTime;

/// One message entering the core, the unit of recording and replay.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Input {
    Flight(FlightInput),
    /// Scheduling time advanced to the given point on the monotonic
    /// timeline. Only clock inputs fire timers.
    Clock(MonotonicTime),
    /// A compute job finished (see [`Effect::Compute`]).
    Job(JobOutcome),
}

/// A client-visible state change, published on the state stream.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum Change {
    Flight(FlightChange),
}

/// A request to the outside world, executed by the runtime.
#[derive(Clone, Debug, PartialEq)]
pub enum Effect {
    /// Run a compute job on the persistent worker for its kind; the
    /// worker replies with an [`Input::Job`] outcome.
    Compute(ComputeJob),
}

/// A job for a compute worker, carrying a snapshot of everything the
/// computation needs plus the epoch it was spawned under.
#[derive(Clone, Debug, PartialEq)]
pub enum ComputeJob {
    /// Extend the flown-track distance by the given new track points.
    TrackDistance { epoch: Epoch, points: Vec<LatLon> },
}

impl ComputeJob {
    pub fn kind(&self) -> JobKind {
        match self {
            ComputeJob::TrackDistance { .. } => JobKind::TrackDistance,
        }
    }

    pub fn epoch(&self) -> Epoch {
        match self {
            ComputeJob::TrackDistance { epoch, .. } => *epoch,
        }
    }
}

/// The compute worker kinds; one persistent worker exists per kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobKind {
    TrackDistance,
}

/// How a compute job ended, re-entering the core as an input.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobOutcome {
    Completed {
        epoch: Epoch,
        result: JobResult,
    },
    /// The worker panicked. Without this input, one-in-flight
    /// bookkeeping would wedge the job kind for the rest of the flight.
    Failed {
        kind: JobKind,
        epoch: Epoch,
    },
}

/// A compute job's payload, recorded verbatim for replay.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobResult {
    /// Total flown-track distance after the job's points.
    TrackDistance(Length),
}

/// Per-kind job generation.
///
/// Semantically breaking changes (a track discontinuity, later a
/// replaced task) bump the epoch: results stamped with an old epoch are
/// dropped, and the worker resets its retained state when it sees a
/// newer one.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Epoch(u32);

impl Epoch {
    pub(crate) fn bump(&mut self) {
        self.0 = self.0.wrapping_add(1);
    }
}

/// The full client-visible state, delivered at the start of every state
/// stream so that reconnecting is just resubscribing.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct Snapshot {
    pub position: Option<PositionFix>,
    /// Flown-track distance in meters.
    pub track_distance: Length,
}

pub use crate::flight::PositionFix;

/// The result of handling one input.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Update {
    pub changes: Vec<Change>,
    pub effects: Vec<Effect>,
    /// The earliest pending timer deadline; the host arms one sleep from
    /// this and delivers an [`Input::Clock`] when it elapses.
    pub next_deadline: Option<MonotonicTime>,
}
