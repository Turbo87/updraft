use crate::connection::ExternalDeviceId;
use crate::fix::{FixTime, UtcInstant, UtcTime};
use crate::signal_state::SignalState;
use crate::time::Timestamp;
use crate::topic::{AltitudeInstrument, GpsInstruments, LatLon, SpeedInstrument};
use std::time::Duration;
use updraft_geo::LatLon as GeoLatLon;
use updraft_units::{Angle, MslAltitude, PressureAltitude, Speed};

const DOMAIN_FRESHNESS_LIMIT: Duration = Duration::from_secs(3);

/// Longest GPS fix interval between two tracks that gives a turn rate.
const TURN_RATE_INTERVAL_LIMIT: Duration = Duration::from_secs(3);

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
    pub fn fresh(self, at: Timestamp) -> Option<Self> {
        (at.saturating_since(self.ingested_at) < DOMAIN_FRESHNESS_LIMIT).then_some(self)
    }

    fn fresh_value(self, at: Timestamp) -> Option<T> {
        self.fresh(at).map(|timed| timed.value)
    }

    fn map<U>(self, map: impl FnOnce(T) -> U) -> Timed<U> {
        Timed::new(map(self.value), self.ingested_at)
    }
}

/// Stores the latest GPS values from one source with independent ingestion times.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct GpsCandidate {
    pub position: Option<Timed<GeoLatLon>>,
    pub altitude: Option<Timed<MslAltitude>>,
    pub track: Option<Timed<Angle>>,
    /// Ground-track turn rate in degrees per second, clockwise positive.
    pub turn_rate: Option<Timed<f64>>,
    pub ground_speed: Option<Timed<Speed>>,
    pub fix_time: GpsTimeCandidate,
    /// The latest track at its GPS fix time.
    track_fix: Option<(FixTime, Angle)>,
}

impl GpsCandidate {
    /// Stores a track and the turn rate since the previous track.
    ///
    /// The turn rate is the track change over the GPS fix interval. It needs
    /// a previous track at most [`TURN_RATE_INTERVAL_LIMIT`] earlier. A
    /// repeated fix time keeps the previous turn rate.
    pub fn observe_track(&mut self, track: Angle, fix_time: Option<FixTime>, at: Timestamp) {
        self.track = Some(Timed::new(track, at));
        let previous = self
            .track_fix
            .zip(fix_time)
            .and_then(|((time, previous), next)| Some((previous, fix_interval(time, next)?)));
        self.turn_rate = match previous {
            Some((_, Duration::ZERO)) => self.turn_rate.map(|rate| Timed::new(rate.value, at)),
            Some((previous, interval)) if interval <= TURN_RATE_INTERVAL_LIMIT => {
                let change = (track - previous).normalized_signed().as_degrees();
                Some(Timed::new(change / interval.as_secs_f64(), at))
            }
            _ => None,
        };
        if previous.is_none_or(|(_, interval)| interval > Duration::ZERO) {
            self.track_fix = fix_time.map(|time| (time, track));
        }
    }
}

/// The time from `previous` to `time`, if `time` is not earlier.
///
/// A time of day after midnight follows a time of day before midnight.
fn fix_interval(previous: FixTime, time: FixTime) -> Option<Duration> {
    const DAY: i64 = 86_400_000;
    let milliseconds = match (previous, time) {
        (FixTime::UtcInstant(previous), FixTime::UtcInstant(time)) => {
            time.unix_milliseconds() - previous.unix_milliseconds()
        }
        (FixTime::UtcTimeOfDay(previous), FixTime::UtcTimeOfDay(time)) => {
            let change = i64::from(time.milliseconds_since_midnight())
                - i64::from(previous.milliseconds_since_midnight());
            change.rem_euclid(DAY)
        }
        _ => return None,
    };
    u64::try_from(milliseconds).ok().map(Duration::from_millis)
}

/// Stores the latest canonical GPS fix times from one source.
///
/// A time of day is newer than the full instant. A new full instant
/// clears it.
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

/// Stores the availability and freshness of a selected domain snapshot.
pub type DomainState<T> = SignalState<Selected<T>>;

impl<T> SignalState<Selected<T>> {
    pub fn discard_last_known_from(&mut self, source: SourceId) {
        if let Self::LastKnown(selected) = self
            && selected.source == source
        {
            *self = Self::Unavailable;
        }
    }

    /// Returns the selected snapshot for current and last-known states.
    pub fn selected(&self) -> Option<&Selected<T>> {
        match self {
            Self::Unavailable => None,
            Self::Current(selected) | Self::LastKnown(selected) => Some(selected),
        }
    }
}

impl SignalState<Selected<GpsSnapshot>> {
    /// Projects the selected GPS state without its source metadata.
    pub fn published(self) -> Option<GpsInstruments> {
        let (selected, stale) = self.value_with_stale()?;
        Some(selected.value.published(stale))
    }
}

impl SignalState<Selected<PressureAltitude>> {
    /// Projects the selected pressure-altitude state without its source metadata.
    pub fn published(self) -> Option<AltitudeInstrument> {
        let (selected, stale) = self.value_with_stale()?;
        Some(AltitudeInstrument {
            meters: selected.value.into_inner().as_meters(),
            stale,
        })
    }
}

impl SignalState<Selected<Speed>> {
    /// Projects the selected true-airspeed state without its source metadata.
    pub fn published(self) -> Option<SpeedInstrument> {
        let (selected, stale) = self.value_with_stale()?;
        Some(SpeedInstrument {
            meters_per_second: selected.value.as_meters_per_second(),
            stale,
        })
    }
}

/// Stores one source-consistent GPS snapshot with its required position anchor.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GpsSnapshot {
    pub position: GeoLatLon,
    pub altitude_msl: Option<Timed<MslAltitude>>,
    pub track: Option<Timed<Angle>>,
    /// Ground-track turn rate in degrees per second, clockwise positive.
    pub turn_rate: Option<Timed<f64>>,
    pub ground_speed: Option<Timed<Speed>>,
    pub fix_time: Option<Timed<FixTime>>,
}

impl GpsSnapshot {
    /// Projects typed GPS values to the scalar units in the instruments topic.
    pub fn published(self, stale: bool) -> GpsInstruments {
        GpsInstruments {
            position: LatLon {
                latitude_degrees: self.position.latitude().as_degrees(),
                longitude_degrees: self.position.longitude().as_degrees(),
            },
            track_degrees: self.track.map(|track| track.value.as_degrees()),
            turn_rate_degrees_per_second: self.turn_rate.map(|rate| rate.value),
            ground_speed_meters_per_second: self
                .ground_speed
                .map(|speed| speed.value.as_meters_per_second()),
            altitude_meters: self
                .altitude_msl
                .map(|altitude| altitude.value.into_inner().as_meters()),
            fix_time: self.fix_time.map(|time| time.value.into()),
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
    let altitude_msl = candidate.altitude.and_then(|altitude| altitude.fresh(at));
    let track = candidate.track.and_then(|track| track.fresh(at));
    let turn_rate = candidate.turn_rate.and_then(|rate| rate.fresh(at));
    let ground_speed = candidate.ground_speed.and_then(|speed| speed.fresh(at));
    let full = candidate.fix_time.full.and_then(|time| time.fresh(at));
    let time_only = candidate.fix_time.time_only.and_then(|time| time.fresh(at));
    let fix_time = match (full, time_only) {
        (Some(full), Some(time)) => {
            let elapsed = time.ingested_at.saturating_since(full.ingested_at);
            let reference = full.value.saturating_add(elapsed);
            Some(time.map(|time| FixTime::UtcInstant(reference.nearest_with_time_of_day(time))))
        }
        (Some(full), None) => Some(full.map(FixTime::UtcInstant)),
        (None, time) => time.map(|time| time.map(FixTime::UtcTimeOfDay)),
    };

    Some(Selected {
        source,
        ingested_at: position.ingested_at,
        value: GpsSnapshot {
            position: position_value,
            altitude_msl,
            track,
            turn_rate,
            ground_speed,
            fix_time,
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
            altitude_msl: Some(Timed::new(
                MslAltitude::new(Length::from_meters(200.0)),
                Timestamp::from_millis(0),
            )),
            track: Some(Timed::new(
                Angle::from_degrees(270.0),
                Timestamp::from_millis(0),
            )),
            turn_rate: Some(Timed::new(-12.5, Timestamp::from_millis(0))),
            ground_speed: Some(Timed::new(
                Speed::from_meters_per_second(45.0),
                Timestamp::from_millis(0),
            )),
            fix_time: None,
        };

        let published = snapshot.published(false);

        assert_some_eq!(published.altitude_meters, 200.0);
        assert_some_eq!(published.track_degrees, 270.0);
        assert_some_eq!(published.turn_rate_degrees_per_second, -12.5);
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
