use std::fmt;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use ts_rs::TS;
use updraft_geo::LatLon;
use updraft_units::{Angle, Length};

use crate::protocol::{ComputeJob, Effect, Epoch, JobOutcome, JobResult};
use crate::time::MonotonicTime;
use crate::timers::{Timer, Timers};

/// A position is shown as stale when no observation arrived for this long.
const POSITION_STALE_AFTER: Duration = Duration::from_secs(10);

/// Consecutive fixes further apart than this are a discontinuity
/// (simulator teleport, replay seek — supported interactions), not
/// flight: the flown-track accumulation restarts via an epoch bump.
const TRACK_DISCONTINUITY: Length = Length::from_kilometers(10.);

/// Which source category produced an observation (see
/// `docs/design/devices.md`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObservationSource {
    Simulation,
}

/// Rejected observation values: non-finite or outside the valid range.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InvalidPosition;

impl fmt::Display for InvalidPosition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("position observation with non-finite or out-of-range values")
    }
}

impl std::error::Error for InvalidPosition {}

/// A validated own-position observation.
///
/// Validation happens at construction, the ingress boundary; deserialized
/// observations (replay) were validated when they were recorded.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct PositionObservation {
    source: ObservationSource,
    observed_at: MonotonicTime,
    location: LatLon,
    track: Option<Angle>,
}

impl PositionObservation {
    pub fn new(
        source: ObservationSource,
        observed_at: MonotonicTime,
        location: LatLon,
        track: Option<Angle>,
    ) -> Result<Self, InvalidPosition> {
        let latitude = location.latitude().as_degrees();
        let longitude = location.longitude().as_degrees();
        if !(-90. ..=90.).contains(&latitude)
            || !(-180. ..=180.).contains(&longitude)
            || track.is_some_and(|track| !track.as_degrees().is_finite())
        {
            return Err(InvalidPosition);
        }

        Ok(Self {
            source,
            observed_at,
            location,
            track,
        })
    }
}

/// The own position as clients see it. `track` is degrees.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct OwnshipPosition {
    pub location: LatLon,
    pub track: Option<Angle>,
}

/// The freshness of the own position: an explicit little state machine
/// (none → current → stale → current), never a loose boolean.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum PositionFix {
    /// A live position from the active source.
    Current(OwnshipPosition),
    /// The last known position; the source stopped delivering.
    Stale(OwnshipPosition),
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FlightInput {
    PositionObserved(PositionObservation),
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum FlightChange {
    PositionChanged(OwnshipPosition),
    /// The current position aged out without a newer observation.
    PositionStale,
    /// Flown-track distance in meters.
    TrackDistanceChanged(Length),
}

/// Flight-domain state: selected sensor values and the flown-track
/// computation, later flight modes, warnings, and logging decisions.
#[derive(Default)]
pub(crate) struct Flight {
    position: Option<PositionFix>,
    track_distance: Length,
    /// Track-distance job scheduling: dirty flag (pending points) plus at
    /// most one job in flight per kind, invalidated by epoch bumps
    /// (see `docs/design/core.md`, "Computation").
    pending_points: Vec<LatLon>,
    job_in_flight: bool,
    epoch: Epoch,
    previous_location: Option<LatLon>,
}

impl Flight {
    pub(crate) fn handle(
        &mut self,
        input: FlightInput,
        timers: &mut Timers,
        changes: &mut Vec<FlightChange>,
        effects: &mut Vec<Effect>,
    ) {
        match input {
            FlightInput::PositionObserved(observation) => {
                self.check_track_discontinuity(observation.location, changes);

                let position = OwnshipPosition {
                    location: observation.location,
                    track: observation.track,
                };
                self.position = Some(PositionFix::Current(position));
                changes.push(FlightChange::PositionChanged(position));

                timers.arm(
                    Timer::PositionStaleness,
                    observation.observed_at + POSITION_STALE_AFTER,
                );

                self.pending_points.push(observation.location);
                self.previous_location = Some(observation.location);
                self.spawn_track_distance_job_if_dirty(effects);
            }
        }
    }

    pub(crate) fn position_became_stale(&mut self, changes: &mut Vec<FlightChange>) {
        if let Some(PositionFix::Current(position)) = self.position {
            self.position = Some(PositionFix::Stale(position));
            changes.push(FlightChange::PositionStale);
        }
    }

    pub(crate) fn job_finished(
        &mut self,
        outcome: JobOutcome,
        changes: &mut Vec<FlightChange>,
        effects: &mut Vec<Effect>,
    ) {
        // The worker is idle again either way; only *applying* the result
        // depends on the epoch.
        self.job_in_flight = false;

        match outcome {
            JobOutcome::Completed {
                epoch,
                result: JobResult::TrackDistance(total),
            } => {
                if epoch == self.epoch && total != self.track_distance {
                    self.track_distance = total;
                    changes.push(FlightChange::TrackDistanceChanged(total));
                }
                self.spawn_track_distance_job_if_dirty(effects);
            }
            // The worker panicked: its retained state is gone, so restart
            // the accumulation visibly rather than continuing from a
            // corrupt total. No immediate respawn — the next observation
            // retries, which naturally rate-limits a failing worker.
            JobOutcome::Failed { .. } => {
                self.epoch.bump();
                self.pending_points.clear();
                self.reset_track_distance(changes);
            }
        }
    }

    pub(crate) fn position(&self) -> Option<PositionFix> {
        self.position
    }

    pub(crate) fn track_distance(&self) -> Length {
        self.track_distance
    }

    fn check_track_discontinuity(&mut self, location: LatLon, changes: &mut Vec<FlightChange>) {
        let jumped = self
            .previous_location
            .is_some_and(|previous| previous.haversine_distance(location) > TRACK_DISCONTINUITY);
        if jumped {
            self.epoch.bump();
            self.pending_points.clear();
            self.reset_track_distance(changes);
        }
    }

    fn reset_track_distance(&mut self, changes: &mut Vec<FlightChange>) {
        if self.track_distance != Length::default() {
            self.track_distance = Length::default();
            changes.push(FlightChange::TrackDistanceChanged(Length::default()));
        }
    }

    fn spawn_track_distance_job_if_dirty(&mut self, effects: &mut Vec<Effect>) {
        if !self.job_in_flight && !self.pending_points.is_empty() {
            effects.push(Effect::Compute(ComputeJob::TrackDistance {
                epoch: self.epoch,
                points: std::mem::take(&mut self.pending_points),
            }));
            self.job_in_flight = true;
        }
    }
}
