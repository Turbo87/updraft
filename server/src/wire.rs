use serde::Serialize;

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

impl From<updraft_core::flight::Snapshot> for FlightSnapshot {
    fn from(snapshot: updraft_core::flight::Snapshot) -> Self {
        Self {
            position: snapshot.position.map(Into::into),
            trace_stats: snapshot.trace_stats.map(Into::into),
        }
    }
}

#[derive(Serialize)]
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

impl From<updraft_core::flight::PositionFix> for PositionFix {
    fn from(fix: updraft_core::flight::PositionFix) -> Self {
        Self {
            observed_at_ms: fix.observed_at.as_secs_f64() * 1_000.,
            latitude_degrees: fix.position.latitude().as_degrees(),
            longitude_degrees: fix.position.longitude().as_degrees(),
            altitude_meters: fix.altitude.map(|altitude| altitude.length().as_meters()),
            track_degrees: fix.track.map(|track| track.as_degrees()),
            ground_speed_meters_per_second: fix
                .ground_speed
                .map(|speed| speed.as_meters_per_second()),
        }
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
                .map(|altitude| altitude.length().as_meters()),
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

impl From<updraft_core::flight::Change> for FlightChange {
    fn from(change: updraft_core::flight::Change) -> Self {
        match change {
            updraft_core::flight::Change::Position(fix) => Self::Position(fix.into()),
            updraft_core::flight::Change::TraceStats(stats) => {
                Self::TraceStats(stats.map(Into::into))
            }
        }
    }
}
