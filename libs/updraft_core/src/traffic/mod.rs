mod climb;

use crate::ExternalDeviceId;
use crate::climb::{ClimbEstimates, Velocity};
use crate::ownship::SourceId;
use crate::time::Timestamp;
use crate::topic::LatLon;
use climb::TrafficClimb;
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::time::Duration;
use updraft_flarmnet::{FlarmnetDatabase, FlarmnetRecord};
use updraft_geo::LatLon as GeoLatLon;
use updraft_nmea::{FlarmAircraftType, FlarmAlarmLevel, FlarmIdType, Pflaa};
use updraft_units::{Angle, Length, MslAltitude};

const STALE_AFTER: Duration = Duration::from_secs(5);
const REMOVE_AFTER: Duration = Duration::from_secs(30);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum TrafficTargetIdType {
    Random,
    Icao,
    Flarm,
    Other(u8),
}

impl From<FlarmIdType> for TrafficTargetIdType {
    fn from(id_type: FlarmIdType) -> Self {
        match id_type {
            FlarmIdType::Random => Self::Random,
            FlarmIdType::Icao => Self::Icao,
            FlarmIdType::Flarm => Self::Flarm,
            FlarmIdType::Other(value) => Self::Other(value),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct TrafficTargetId {
    pub id_type: TrafficTargetIdType,
    pub value: u32,
}

impl TrafficTargetId {
    pub fn new(id_type: TrafficTargetIdType, value: u32) -> Self {
        Self { id_type, value }
    }
}

impl fmt::Display for TrafficTargetId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.id_type {
            TrafficTargetIdType::Random => write!(formatter, "random:{:06X}", self.value),
            TrafficTargetIdType::Icao => write!(formatter, "icao:{:06X}", self.value),
            TrafficTargetIdType::Flarm => write!(formatter, "flarm:{:06X}", self.value),
            TrafficTargetIdType::Other(value) => {
                write!(formatter, "other:{value}:{:06X}", self.value)
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub enum TrafficType {
    Unknown,
    Glider,
    TowPlane,
    Helicopter,
    Skydiver,
    DropPlane,
    HangGlider,
    Paraglider,
    PistonAircraft,
    JetAircraft,
    Balloon,
    Airship,
    Uav,
    StaticObstacle,
}

impl From<FlarmAircraftType> for TrafficType {
    fn from(aircraft_type: FlarmAircraftType) -> Self {
        match aircraft_type {
            FlarmAircraftType::Unknown | FlarmAircraftType::Other(_) => Self::Unknown,
            FlarmAircraftType::Glider => Self::Glider,
            FlarmAircraftType::TowPlane => Self::TowPlane,
            FlarmAircraftType::Helicopter => Self::Helicopter,
            FlarmAircraftType::Skydiver => Self::Skydiver,
            FlarmAircraftType::DropPlane => Self::DropPlane,
            FlarmAircraftType::HangGlider => Self::HangGlider,
            FlarmAircraftType::Paraglider => Self::Paraglider,
            FlarmAircraftType::PistonAircraft => Self::PistonAircraft,
            FlarmAircraftType::JetAircraft => Self::JetAircraft,
            FlarmAircraftType::Balloon => Self::Balloon,
            FlarmAircraftType::Airship => Self::Airship,
            FlarmAircraftType::Uav => Self::Uav,
            FlarmAircraftType::StaticObstacle => Self::StaticObstacle,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub enum TrafficAlarmLevel {
    Unknown,
    None,
    Low,
    Important,
    Urgent,
}

impl From<FlarmAlarmLevel> for TrafficAlarmLevel {
    fn from(alarm_level: FlarmAlarmLevel) -> Self {
        match alarm_level {
            FlarmAlarmLevel::None => Self::None,
            FlarmAlarmLevel::Low => Self::Low,
            FlarmAlarmLevel::Important => Self::Important,
            FlarmAlarmLevel::Urgent => Self::Urgent,
            FlarmAlarmLevel::Other(_) => Self::Unknown,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TrafficTarget {
    pub id: TrafficTargetId,
    pub position: GeoLatLon,
    pub altitude_msl: Option<MslAltitude>,
    pub climb: Option<ClimbEstimates>,
    pub traffic_type: TrafficType,
    pub track: Option<Angle>,
    pub alarm_level: TrafficAlarmLevel,
    pub stale: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct StoredTrafficTarget {
    target: TrafficTarget,
    observed_at: Timestamp,
}

#[derive(Debug, Default)]
pub struct TrafficState {
    targets: BTreeMap<TrafficTargetId, StoredTrafficTarget>,
    climbs: BTreeMap<TrafficTargetId, TrafficClimb>,
}

#[derive(Debug, Default, PartialEq)]
pub struct TrafficChanges {
    upserts: BTreeMap<TrafficTargetId, TrafficTarget>,
    removed: BTreeSet<TrafficTargetId>,
}

impl TrafficState {
    pub fn update_climb(
        &mut self,
        target: &mut TrafficTarget,
        source: Option<(ExternalDeviceId, SourceId)>,
        at: Timestamp,
        velocity: Option<Velocity>,
        wind: Option<Velocity>,
    ) {
        target.climb = source
            .zip(target.altitude_msl)
            .and_then(|(source, altitude)| {
                let meters = altitude.into_inner().as_meters();
                if !meters.is_finite() {
                    return None;
                }
                self.climbs.entry(target.id).or_default().observe(
                    source,
                    at,
                    altitude.into_inner(),
                    velocity.map(|value| (at.since_start(), value)),
                    wind,
                )
            });
    }

    pub fn observe(
        &mut self,
        mut target: TrafficTarget,
        at: Timestamp,
        changes: &mut TrafficChanges,
    ) {
        target.stale = false;
        let changed = self
            .targets
            .get(&target.id)
            .is_none_or(|stored| stored.target != target);
        self.targets.insert(
            target.id,
            StoredTrafficTarget {
                target,
                observed_at: at,
            },
        );
        if changed {
            changes.upsert(target);
        }
    }

    pub fn expire(&mut self, at: Timestamp) -> TrafficChanges {
        self.climbs.retain(|_, climb| !climb.expired(at));
        let mut changes = TrafficChanges::default();
        let mut removed = Vec::new();

        for (id, stored) in &mut self.targets {
            let elapsed = at.saturating_since(stored.observed_at);
            if elapsed >= REMOVE_AFTER {
                removed.push(*id);
            } else if elapsed >= STALE_AFTER && !stored.target.stale {
                stored.target.stale = true;
                changes.upsert(stored.target);
            }
        }

        for id in removed {
            self.targets.remove(&id);
            changes.remove(id);
        }

        changes
    }

    pub fn snapshot(&self) -> Vec<TrafficTarget> {
        self.targets.values().map(|stored| stored.target).collect()
    }

    pub fn published_targets(&self, database: &FlarmnetDatabase) -> Vec<PublishedTrafficTarget> {
        self.targets
            .values()
            .map(|stored| stored.target.publish(database))
            .collect()
    }
}

impl TrafficChanges {
    fn upsert(&mut self, target: TrafficTarget) {
        self.removed.remove(&target.id);
        self.upserts.insert(target.id, target);
    }

    fn remove(&mut self, id: TrafficTargetId) {
        self.upserts.remove(&id);
        self.removed.insert(id);
    }

    pub fn into_delta(self, database: &FlarmnetDatabase) -> Option<TrafficDelta> {
        if self.upserts.is_empty() && self.removed.is_empty() {
            None
        } else {
            Some(TrafficDelta {
                upserts: self
                    .upserts
                    .into_values()
                    .map(|target| target.publish(database))
                    .collect(),
                removed: self.removed.into_iter().map(|id| id.to_string()).collect(),
            })
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct PublishedTrafficTarget {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub flarmnet: Option<FlarmnetRecord>,
    pub id: String,
    pub position: LatLon,
    pub altitude_msl_meters: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub climb: Option<ClimbEstimates>,
    pub traffic_type: TrafficType,
    pub track_degrees: Option<f64>,
    pub alarm_level: TrafficAlarmLevel,
    pub stale: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct TrafficDelta {
    pub upserts: Vec<PublishedTrafficTarget>,
    pub removed: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "type", content = "value", rename_all = "camelCase")]
pub enum TrafficUpdate {
    Snapshot(Vec<PublishedTrafficTarget>),
    Delta(TrafficDelta),
}

impl From<TrafficTarget> for PublishedTrafficTarget {
    fn from(target: TrafficTarget) -> Self {
        Self {
            flarmnet: None,
            id: target.id.to_string(),
            position: LatLon {
                latitude_degrees: target.position.latitude().as_degrees(),
                longitude_degrees: target.position.longitude().as_degrees(),
            },
            altitude_msl_meters: target
                .altitude_msl
                .map(|altitude| altitude.into_inner().as_meters()),
            climb: target.climb,
            traffic_type: target.traffic_type,
            track_degrees: target.track.map(Angle::as_degrees),
            alarm_level: target.alarm_level,
            stale: target.stale,
        }
    }
}

impl TrafficTarget {
    fn publish(self, database: &FlarmnetDatabase) -> PublishedTrafficTarget {
        let id = self.id;
        let flarmnet = match id.id_type {
            TrafficTargetIdType::Flarm | TrafficTargetIdType::Icao => database.lookup(id.value),
            TrafficTargetIdType::Random | TrafficTargetIdType::Other(_) => None,
        };
        PublishedTrafficTarget {
            flarmnet: flarmnet.cloned(),
            ..self.into()
        }
    }
}

pub fn within_wind_range(pflaa: &Pflaa) -> bool {
    let Some((north, east)) = pflaa.relative_north.zip(pflaa.relative_east) else {
        return false;
    };
    let distance = Length::from_meters(north.as_meters().hypot(east.as_meters()));
    distance <= Length::from_kilometers(10.)
        && pflaa
            .relative_vertical
            .is_some_and(|height| height.abs() <= Length::from_meters(1500.))
}

pub fn target_from_pflaa(
    pflaa: &Pflaa,
    ownship_position: GeoLatLon,
    ownship_altitude: Option<MslAltitude>,
) -> Option<TrafficTarget> {
    let id = TrafficTargetId::new(pflaa.id_type?.into(), pflaa.id.as_ref()?.address);
    let relative_north = pflaa.relative_north?;
    let relative_east = pflaa.relative_east?;
    let north_meters = relative_north.as_meters();
    let east_meters = relative_east.as_meters();
    let distance = Length::from_meters(north_meters.hypot(east_meters));
    let bearing = Angle::from_radians(east_meters.atan2(north_meters)).normalized();
    let position = ownship_position.destination(bearing, distance);
    let altitude_msl = ownship_altitude
        .zip(pflaa.relative_vertical)
        .map(|(ownship, relative)| MslAltitude::new(ownship.into_inner() + relative));

    Some(TrafficTarget {
        id,
        position,
        altitude_msl,
        climb: None,
        traffic_type: pflaa.aircraft_type.into(),
        track: pflaa.track,
        alarm_level: pflaa.alarm_level.into(),
        stale: false,
    })
}

#[cfg(test)]
mod tests;
