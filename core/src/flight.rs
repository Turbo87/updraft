use std::fmt;

use serde::{Deserialize, Serialize};
use ts_rs::TS;
use updraft_geo::LatLon;
use updraft_units::Angle;

use crate::time::MonotonicTime;

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
}

/// Flight-domain state: selected sensor values and, later, flight modes,
/// warnings, and logging decisions.
#[derive(Default)]
pub(crate) struct Flight {
    position: Option<OwnshipPosition>,
}

impl Flight {
    pub(crate) fn handle(&mut self, input: FlightInput) -> Option<FlightChange> {
        match input {
            FlightInput::PositionObserved(observation) => {
                let position = OwnshipPosition {
                    location: observation.location,
                    track: observation.track,
                };
                self.position = Some(position);
                Some(FlightChange::PositionChanged(position))
            }
        }
    }

    pub(crate) fn position(&self) -> Option<OwnshipPosition> {
        self.position
    }
}
