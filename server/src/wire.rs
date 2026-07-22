use serde::{Deserialize, Serialize};
use std::time::Duration;
use updraft_core::flight::{
    Availability as CoreAvailability, GnssData as CoreGnssData, GnssUpdate,
    Observation as CoreObservation,
};

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
    gnss: GnssData,
    pressure_altitude_meters: Availability<f64>,
    trace_stats: Option<TraceStats>,
}

impl From<updraft_core::flight::FlightSnapshot> for FlightSnapshot {
    fn from(snapshot: updraft_core::flight::FlightSnapshot) -> Self {
        Self {
            gnss: snapshot.gnss.into(),
            pressure_altitude_meters: map_availability(snapshot.pressure_altitude, |altitude| {
                altitude.into_inner().as_meters()
            }),
            trace_stats: snapshot.trace_stats.map(Into::into),
        }
    }
}

#[derive(Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "status", content = "value", rename_all = "camelCase")]
pub enum Availability<T> {
    Unavailable,
    Current(T),
    LastKnown(T),
}

fn map_availability<T, U>(
    availability: CoreAvailability<T>,
    map: impl FnOnce(T) -> U,
) -> Availability<U> {
    match availability {
        CoreAvailability::Unavailable => Availability::Unavailable,
        CoreAvailability::Current(value) => Availability::Current(map(value)),
        CoreAvailability::LastKnown(value) => Availability::LastKnown(map(value)),
    }
}

/// A geographic latitude and longitude in degrees.
#[derive(Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct LatLon {
    latitude_degrees: f64,
    longitude_degrees: f64,
}

/// Selected GNSS components.
#[derive(Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct GnssData {
    position: Availability<LatLon>,
    altitude_meters: Availability<f64>,
    track_degrees: Availability<f64>,
    ground_speed_meters_per_second: Availability<f64>,
}

impl From<CoreGnssData> for GnssData {
    fn from(gnss: CoreGnssData) -> Self {
        Self {
            position: map_availability(gnss.position, |position| LatLon {
                latitude_degrees: position.latitude().as_degrees(),
                longitude_degrees: position.longitude().as_degrees(),
            }),
            altitude_meters: map_availability(gnss.altitude, |altitude| {
                altitude.into_inner().as_meters()
            }),
            track_degrees: map_availability(gnss.track, |track| track.as_degrees()),
            ground_speed_meters_per_second: map_availability(gnss.ground_speed, |speed| {
                speed.as_meters_per_second()
            }),
        }
    }
}

/// A position submitted through interactive simulator mode.
#[derive(Deserialize, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct SimulationPosition {
    observed_at_ms: f64,
    latitude_degrees: f64,
    longitude_degrees: f64,
    altitude_meters: Option<f64>,
    track_degrees: Option<f64>,
    ground_speed_meters_per_second: Option<f64>,
}

/// A simulator position containing a value outside the accepted input domain.
#[derive(Debug)]
pub struct InvalidSimulationPosition;

impl TryFrom<SimulationPosition> for CoreObservation<GnssUpdate> {
    type Error = InvalidSimulationPosition;

    fn try_from(position: SimulationPosition) -> Result<Self, Self::Error> {
        let observed_at = Duration::try_from_secs_f64(position.observed_at_ms / 1_000.)
            .map_err(|_| InvalidSimulationPosition)?;
        if !(-90. ..=90.).contains(&position.latitude_degrees)
            || !(-180. ..=180.).contains(&position.longitude_degrees)
            || position
                .altitude_meters
                .is_some_and(|altitude| !altitude.is_finite())
            || position
                .track_degrees
                .is_some_and(|track| !(0. ..360.).contains(&track))
            || position
                .ground_speed_meters_per_second
                .is_some_and(|speed| !speed.is_finite() || speed < 0.)
        {
            return Err(InvalidSimulationPosition);
        }

        Ok(CoreObservation::new(
            observed_at,
            GnssUpdate {
                position: updraft_geo::LatLon::from_degrees(
                    position.latitude_degrees,
                    position.longitude_degrees,
                ),
                altitude: position.altitude_meters.map(|meters| {
                    updraft_units::MslAltitude::new(updraft_units::Length::from_meters(meters))
                }),
                track: position
                    .track_degrees
                    .map(updraft_units::Angle::from_degrees),
                ground_speed: position
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
    Gnss(GnssData),
    PressureAltitudeMeters(Availability<f64>),
    TraceStats(Option<TraceStats>),
}

impl From<updraft_core::flight::FlightChange> for FlightChange {
    fn from(change: updraft_core::flight::FlightChange) -> Self {
        match change {
            updraft_core::flight::FlightChange::Gnss(gnss) => Self::Gnss(gnss.into()),
            updraft_core::flight::FlightChange::PressureAltitude(altitude) => {
                Self::PressureAltitudeMeters(map_availability(altitude, |altitude| {
                    altitude.into_inner().as_meters()
                }))
            }
            updraft_core::flight::FlightChange::TraceStats(stats) => {
                Self::TraceStats(stats.map(Into::into))
            }
        }
    }
}
