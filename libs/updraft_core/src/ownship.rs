use crate::connection::ExternalDeviceId;
use crate::fix::{FixTime, UtcInstant, UtcTime};
use crate::time::Timestamp;
use crate::topic::{GpsInstruments, LatLon, PressureAltitudeInstruments, TrueAirspeedInstruments};
use std::time::Duration;
use updraft_geo::LatLon as GeoLatLon;
use updraft_units::{Angle, MslAltitude, PressureAltitude, Speed};

const DOMAIN_FRESHNESS_LIMIT: Duration = Duration::from_secs(3);

/// A source value and the monotonic time when the core ingested it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Timed<T> {
    pub value: T,
    pub ingested_at: Timestamp,
}

impl<T> Timed<T> {
    /// Uses the supplied monotonic ingestion time without reading a clock.
    pub fn new(value: T, ingested_at: Timestamp) -> Self {
        Self { value, ingested_at }
    }
}

impl<T: Copy> Timed<T> {
    fn fresh_value(self, at: Timestamp) -> Option<T> {
        (at.saturating_since(self.ingested_at) < DOMAIN_FRESHNESS_LIMIT).then_some(self.value)
    }
}

/// Stores the latest GPS values from one source with independent ingestion times.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct GpsCandidate {
    pub position: Option<Timed<GeoLatLon>>,
    pub altitude: Option<Timed<MslAltitude>>,
    pub track: Option<Timed<Angle>>,
    pub ground_speed: Option<Timed<Speed>>,
    pub fix_time: GpsTimeCandidate,
}

/// Stores the latest canonical GPS fix times from one source.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct GpsTimeCandidate {
    pub full: Option<Timed<UtcInstant>>,
    pub time_only: Option<Timed<UtcTime>>,
}

/// Identifies the external device or internal sensor that supplied a domain.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceId {
    External(ExternalDeviceId),
    InternalGps,
}

/// Stores a selected domain snapshot with its source and anchor ingestion time.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Selected<T> {
    pub source: SourceId,
    pub ingested_at: Timestamp,
    pub value: T,
}

/// Represents an unavailable, current, or frozen last-known domain snapshot.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum DomainState<T> {
    #[default]
    Unavailable,
    Current(Selected<T>),
    LastKnown(Selected<T>),
}

impl<T> DomainState<T> {
    /// Returns the selected snapshot for current and last-known states.
    pub fn selected(&self) -> Option<&Selected<T>> {
        match self {
            Self::Unavailable => None,
            Self::Current(selected) | Self::LastKnown(selected) => Some(selected),
        }
    }
}

impl DomainState<GpsSnapshot> {
    /// Projects the selected GPS state without its source metadata.
    pub fn published(self) -> Option<GpsInstruments> {
        match self {
            Self::Unavailable => None,
            Self::Current(selected) => Some(selected.value.published(false)),
            Self::LastKnown(selected) => Some(selected.value.published(true)),
        }
    }
}

impl DomainState<PressureAltitude> {
    /// Projects the selected pressure-altitude state without its source metadata.
    pub fn published(self) -> Option<PressureAltitudeInstruments> {
        match self {
            Self::Unavailable => None,
            Self::Current(selected) => Some(PressureAltitudeInstruments {
                meters: selected.value.into_inner().as_meters(),
                stale: false,
            }),
            Self::LastKnown(selected) => Some(PressureAltitudeInstruments {
                meters: selected.value.into_inner().as_meters(),
                stale: true,
            }),
        }
    }
}

impl DomainState<Speed> {
    /// Projects the selected true-airspeed state without its source metadata.
    pub fn published(self) -> Option<TrueAirspeedInstruments> {
        match self {
            Self::Unavailable => None,
            Self::Current(selected) => Some(TrueAirspeedInstruments {
                meters_per_second: selected.value.as_meters_per_second(),
                stale: false,
            }),
            Self::LastKnown(selected) => Some(TrueAirspeedInstruments {
                meters_per_second: selected.value.as_meters_per_second(),
                stale: true,
            }),
        }
    }
}

/// Stores one source-consistent GPS snapshot with its required position anchor.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GpsSnapshot {
    pub position: GeoLatLon,
    pub altitude_msl: Option<MslAltitude>,
    pub track: Option<Angle>,
    pub ground_speed: Option<Speed>,
    pub fix_time: Option<FixTime>,
}

impl GpsSnapshot {
    /// Projects typed GPS values to the scalar units in the instruments topic.
    pub fn published(self, stale: bool) -> GpsInstruments {
        GpsInstruments {
            position: LatLon {
                latitude_degrees: self.position.latitude().as_degrees(),
                longitude_degrees: self.position.longitude().as_degrees(),
            },
            track_degrees: self.track.map(Angle::as_degrees),
            ground_speed_meters_per_second: self.ground_speed.map(Speed::as_meters_per_second),
            altitude_meters: self
                .altitude_msl
                .map(|altitude| altitude.into_inner().as_meters()),
            fix_time: self.fix_time.map(Into::into),
            stale,
        }
    }
}

/// Creates a GPS selection when the candidate position is fresh.
pub fn select_gps_candidate(
    source: SourceId,
    candidate: GpsCandidate,
    at: Timestamp,
) -> Option<Selected<GpsSnapshot>> {
    let position = candidate.position?;
    let position_value = position.fresh_value(at)?;

    Some(Selected {
        source,
        ingested_at: position.ingested_at,
        value: GpsSnapshot {
            position: position_value,
            altitude_msl: candidate
                .altitude
                .and_then(|altitude| altitude.fresh_value(at)),
            track: candidate.track.and_then(|track| track.fresh_value(at)),
            ground_speed: candidate
                .ground_speed
                .and_then(|speed| speed.fresh_value(at)),
            fix_time: candidate
                .fix_time
                .full
                .and_then(|time| time.fresh_value(at))
                .map(FixTime::UtcInstant)
                .or_else(|| {
                    candidate
                        .fix_time
                        .time_only
                        .and_then(|time| time.fresh_value(at))
                        .map(FixTime::UtcTimeOfDay)
                }),
        },
    })
}

/// Creates a pressure-altitude selection when the candidate is fresh.
pub fn select_pressure_altitude_candidate(
    source: SourceId,
    candidate: Option<Timed<PressureAltitude>>,
    at: Timestamp,
) -> Option<Selected<PressureAltitude>> {
    let altitude = candidate?;
    Some(Selected {
        source,
        ingested_at: altitude.ingested_at,
        value: altitude.fresh_value(at)?,
    })
}

/// Creates a true-airspeed selection when the candidate is fresh.
pub fn select_true_airspeed_candidate(
    source: SourceId,
    candidate: Option<Timed<Speed>>,
    at: Timestamp,
) -> Option<Selected<Speed>> {
    let speed = candidate?;
    Some(Selected {
        source,
        ingested_at: speed.ingested_at,
        value: speed.fresh_value(at)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::topic::LatLon;
    use claims::assert_some_eq;
    use updraft_units::Length;

    #[test]
    fn projects_domain_values_to_the_instruments_topic() {
        let snapshot = GpsSnapshot {
            position: GeoLatLon::from_degrees(50.823, 6.186),
            altitude_msl: Some(MslAltitude::new(Length::from_meters(200.0))),
            track: Some(Angle::from_degrees(270.0)),
            ground_speed: Some(Speed::from_meters_per_second(45.0)),
            fix_time: None,
        };

        let published = snapshot.published(false);

        assert_some_eq!(published.altitude_meters, 200.0);
        assert_some_eq!(published.track_degrees, 270.0);
        assert_some_eq!(published.ground_speed_meters_per_second, 45.0);
        assert_eq!(
            published.position,
            LatLon {
                latitude_degrees: 50.823,
                longitude_degrees: 6.186,
            }
        );
        assert!(!published.stale);
    }
}
