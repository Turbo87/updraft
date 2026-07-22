use serde::{Deserialize, Serialize};
use std::time::Duration;
use updraft_core::flight::{GnssUpdate, Observation};

#[cfg(feature = "ts")]
pub mod bindings;

#[derive(Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    flight: FlightSnapshot,
}

impl From<updraft_core::Snapshot> for Snapshot {
    fn from(snapshot: updraft_core::Snapshot) -> Self {
        Self {
            flight: snapshot.flight.into(),
        }
    }
}

#[derive(Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
struct FlightSnapshot {
    position: Option<PositionFix>,
    trace_stats: Option<TraceStats>,
}

impl From<updraft_core::flight::FlightSnapshot> for FlightSnapshot {
    fn from(snapshot: updraft_core::flight::FlightSnapshot) -> Self {
        Self {
            position: snapshot.position.map(Into::into),
            trace_stats: snapshot.trace_stats.map(Into::into),
        }
    }
}

#[derive(Deserialize, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct PositionFix {
    observed_at_ms: f64,
    latitude_degrees: f64,
    longitude_degrees: f64,
    altitude_meters: Option<f64>,
    track_degrees: Option<f64>,
    ground_speed_meters_per_second: Option<f64>,
}

/// A wire position fix containing a value outside the accepted input domain.
#[derive(Debug)]
pub struct InvalidPositionFix;

impl From<updraft_core::flight::PositionFix> for PositionFix {
    fn from(fix: updraft_core::flight::PositionFix) -> Self {
        Self {
            observed_at_ms: fix.observed_at.as_secs_f64() * 1_000.,
            latitude_degrees: fix.position.latitude().as_degrees(),
            longitude_degrees: fix.position.longitude().as_degrees(),
            altitude_meters: fix
                .altitude
                .map(|altitude| altitude.into_inner().as_meters()),
            track_degrees: fix.track.map(|track| track.as_degrees()),
            ground_speed_meters_per_second: fix
                .ground_speed
                .map(|speed| speed.as_meters_per_second()),
        }
    }
}

impl TryFrom<PositionFix> for Observation<GnssUpdate> {
    type Error = InvalidPositionFix;

    fn try_from(fix: PositionFix) -> Result<Self, Self::Error> {
        let observed_at = Duration::try_from_secs_f64(fix.observed_at_ms / 1_000.)
            .map_err(|_| InvalidPositionFix)?;
        if !(-90. ..=90.).contains(&fix.latitude_degrees)
            || !(-180. ..=180.).contains(&fix.longitude_degrees)
            || fix
                .altitude_meters
                .is_some_and(|altitude| !altitude.is_finite())
            || fix
                .track_degrees
                .is_some_and(|track| !(0. ..360.).contains(&track))
            || fix
                .ground_speed_meters_per_second
                .is_some_and(|speed| !speed.is_finite() || speed < 0.)
        {
            return Err(InvalidPositionFix);
        }

        Ok(Observation::new(
            observed_at,
            GnssUpdate {
                position: updraft_geo::LatLon::from_degrees(
                    fix.latitude_degrees,
                    fix.longitude_degrees,
                ),
                altitude: fix.altitude_meters.map(|meters| {
                    updraft_units::MslAltitude::new(updraft_units::Length::from_meters(meters))
                }),
                track: fix.track_degrees.map(updraft_units::Angle::from_degrees),
                ground_speed: fix
                    .ground_speed_meters_per_second
                    .map(updraft_units::Speed::from_meters_per_second),
            },
        ))
    }
}

#[derive(Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct TraceStats {
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    fix_count: u64,
    distance_meters: f64,
    max_altitude_meters: Option<f64>,
}

impl From<updraft_core::flight::TraceStats> for TraceStats {
    fn from(stats: updraft_core::flight::TraceStats) -> Self {
        Self {
            fix_count: stats.fix_count,
            distance_meters: stats.distance.as_meters(),
            max_altitude_meters: stats
                .max_altitude
                .map(|altitude| altitude.into_inner().as_meters()),
        }
    }
}

#[derive(Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "group", rename_all = "camelCase")]
pub enum Change {
    Flight(FlightChange),
}

impl From<updraft_core::Change> for Change {
    fn from(change: updraft_core::Change) -> Self {
        match change {
            updraft_core::Change::Flight(change) => Self::Flight(change.into()),
        }
    }
}

#[derive(Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "type", content = "value", rename_all = "camelCase")]
pub enum FlightChange {
    Position(PositionFix),
    TraceStats(Option<TraceStats>),
}

impl From<updraft_core::flight::FlightChange> for FlightChange {
    fn from(change: updraft_core::flight::FlightChange) -> Self {
        match change {
            updraft_core::flight::FlightChange::Position(fix) => Self::Position(fix.into()),
            updraft_core::flight::FlightChange::TraceStats(stats) => {
                Self::TraceStats(stats.map(Into::into))
            }
        }
    }
}
