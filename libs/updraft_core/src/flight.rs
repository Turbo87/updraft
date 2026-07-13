//! Flight state for own position.

use std::time::Duration;

use updraft_geo::LatLon;
use updraft_units::{Angle, Length, Speed};

use crate::protocol::{Change as AppChange, Update};

/// An altitude above mean sea level.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct MslAltitude(Length);

impl MslAltitude {
    pub const ZERO: Self = Self(Length::ZERO);

    pub const fn new(length: Length) -> Self {
        Self(length)
    }

    pub const fn length(self) -> Length {
        self.0
    }
}

/// A normalized own-position observation from a positioning source.
///
/// This doubles as the published kinematic state vector: clients use it
/// to estimate the current render position, so frame-rate animation never
/// crosses the transport.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PositionFix {
    /// Clock time when the fix was observed.
    pub observed_at: Duration,
    pub position: LatLon,
    pub altitude: Option<MslAltitude>,
    /// Track over ground.
    pub track: Option<Angle>,
    pub ground_speed: Option<Speed>,
}

/// A recorded event or request owned by the flight domain.
#[derive(Clone, Debug, PartialEq)]
pub enum Input {
    /// A normalized sensor observation.
    Observation(Observation),
}

impl Input {
    pub(crate) fn observed_at(&self) -> Option<Duration> {
        match self {
            Self::Observation(observation) => Some(observation.observed_at()),
        }
    }
}

/// A normalized sensor observation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Observation {
    /// An own-position fix.
    Position(PositionFix),
}

impl Observation {
    fn observed_at(&self) -> Duration {
        match self {
            Self::Position(fix) => fix.observed_at,
        }
    }
}

/// Requests the current own-position fix.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GetPosition;

impl crate::Query for GetPosition {
    type Output = Option<PositionFix>;

    fn execute(self, app: &crate::App) -> Self::Output {
        app.flight.position()
    }
}

/// A client-visible flight-state update.
#[derive(Clone, Debug, PartialEq)]
pub enum Change {
    /// The own-position last-value update.
    Position(PositionFix),
}

/// The shared current flight state for a newly subscribing client.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Snapshot {
    pub position: Option<PositionFix>,
}

/// The flight domain state.
#[derive(Debug, Default)]
pub(crate) struct Flight {
    position: Option<PositionFix>,
}

impl Flight {
    pub(crate) fn position(&self) -> Option<PositionFix> {
        self.position
    }

    pub(crate) fn handle(&mut self, input: Input, update: &mut Update) {
        let Input::Observation(Observation::Position(fix)) = input;
        self.position = Some(fix);
        update
            .changes
            .push(AppChange::Flight(Change::Position(fix)));
    }

    pub(crate) fn snapshot(&self) -> Snapshot {
        Snapshot {
            position: self.position,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn input_reports_observation_time() {
        let observed_at = Duration::from_micros(42);
        let input = Input::Observation(Observation::Position(PositionFix {
            observed_at,
            position: LatLon::from_degrees(50., 6.),
            altitude: None,
            track: None,
            ground_speed: None,
        }));

        assert_eq!(input.observed_at(), Some(observed_at));
    }
}
