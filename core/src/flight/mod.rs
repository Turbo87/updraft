use std::time::Duration;

use serde::Serialize;
use updraft_geo::LatLon;
use updraft_units::Angle;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct MonotonicTime(Duration);

impl MonotonicTime {
    pub const fn from_duration(duration: Duration) -> Self {
        Self(duration)
    }

    pub const fn as_duration(self) -> Duration {
        self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ObservationSource {
    Simulation,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InvalidPosition;

#[derive(Clone, Copy, Debug, PartialEq)]
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
        if !latitude.is_finite() || !(-90. ..=90.).contains(&latitude) {
            return Err(InvalidPosition);
        }
        let longitude = location.longitude().as_degrees();
        if !longitude.is_finite() || !(-180. ..=180.).contains(&longitude) {
            return Err(InvalidPosition);
        }
        if track.is_some_and(|track| !track.as_degrees().is_finite()) {
            return Err(InvalidPosition);
        }

        Ok(Self {
            source,
            observed_at,
            location,
            track,
        })
    }

    fn into_position(self) -> OwnshipPosition {
        let Self {
            source: _,
            observed_at: _,
            location,
            track,
        } = self;
        OwnshipPosition::new(location, track)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub struct OwnshipPosition {
    location: LatLon,
    track: Option<Angle>,
}

impl OwnshipPosition {
    pub const fn new(location: LatLon, track: Option<Angle>) -> Self {
        Self { location, track }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FlightInput {
    PositionObserved(PositionObservation),
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FlightChange {
    PositionChanged(OwnshipPosition),
}

#[derive(Default)]
pub(crate) struct Flight {
    position: Option<OwnshipPosition>,
}

impl Flight {
    pub(crate) fn handle(&mut self, input: FlightInput) -> Option<FlightChange> {
        match input {
            FlightInput::PositionObserved(position) => {
                let position = position.into_position();
                self.position = Some(position);
                Some(FlightChange::PositionChanged(position))
            }
        }
    }

    pub(crate) fn position(&self) -> Option<OwnshipPosition> {
        self.position
    }
}
