mod climb;
mod correction;
pub mod reference;
mod velocity;

use crate::ExternalDeviceId;
use crate::climb::ClimbEstimates;
use crate::ownship::SourceId;
use crate::time::Timestamp;
use crate::topic::LatLon;
use climb::TrafficClimb;
pub use climb::TrafficMotion;
pub use correction::FlarmCorrection;
use serde::Serialize;
use std::collections::btree_map::Entry;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::time::Duration;
use updraft_flarmnet::{FlarmnetDatabase, FlarmnetRecord};
use updraft_geo::LatLon as GeoLatLon;
use updraft_nmea::{FlarmAircraftType, FlarmAlarmLevel, FlarmIdType, Pflaa, PflamIdentity};
use updraft_units::{Angle, Length, MslAltitude};

const REFERENCE_HOLD: Duration = Duration::from_secs(2);
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
    position_epoch: Option<(ExternalDeviceId, i64)>,
    position_observed_at: Timestamp,
}

/// Whether this report can use the experimental FLARM position correction.
#[derive(Clone, Copy, Debug)]
pub enum TrafficPositionReference {
    Uncorrected,
    Unavailable {
        source: ExternalDeviceId,
    },
    Aligned {
        source: ExternalDeviceId,
        epoch: i64,
    },
}

impl TrafficPositionReference {
    fn source(self) -> Option<ExternalDeviceId> {
        match self {
            Self::Uncorrected => None,
            Self::Unavailable { source } | Self::Aligned { source, .. } => Some(source),
        }
    }

    fn epoch(self) -> Option<i64> {
        match self {
            Self::Aligned { epoch, .. } => Some(epoch),
            _ => None,
        }
    }
}

#[derive(Debug, Default)]
pub struct TrafficState {
    targets: BTreeMap<TrafficTargetId, StoredTrafficTarget>,
    climbs: BTreeMap<TrafficTargetId, TrafficClimb>,
    broadcast_identities: BTreeMap<TrafficTargetId, FlarmBroadcastIdentity>,
}

#[derive(Debug, Default, PartialEq)]
pub struct TrafficChanges {
    upserts: BTreeMap<TrafficTargetId, TrafficTarget>,
    removed: BTreeSet<TrafficTargetId>,
}

impl TrafficState {
    pub fn observe_broadcast_identity(
        &mut self,
        id: TrafficTargetId,
        identity: PflamIdentity,
        changes: &mut TrafficChanges,
    ) {
        let changed = match self.broadcast_identities.entry(id) {
            Entry::Occupied(mut entry) => entry.get_mut().update(identity),
            Entry::Vacant(entry) => {
                let Some(identity) = FlarmBroadcastIdentity::new(identity) else {
                    return;
                };
                entry.insert(identity);
                true
            }
        };
        if changed && let Some(stored) = self.targets.get(&id) {
            changes.upsert(stored.target);
        }
    }

    pub fn reset_position_epochs(&mut self, device_id: Option<ExternalDeviceId>) {
        for stored in self.targets.values_mut() {
            let source = stored.position_epoch.map(|(source, _)| source);
            if device_id.is_none() || device_id == source {
                stored.position_epoch = None;
            }
        }
    }

    pub fn update_climb(
        &mut self,
        target: &mut TrafficTarget,
        source: Option<(ExternalDeviceId, SourceId)>,
        at: Timestamp,
        motion: TrafficMotion,
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
                    motion,
                )
            });
    }

    pub fn observe(
        &mut self,
        mut target: TrafficTarget,
        at: Timestamp,
        reference: TrafficPositionReference,
        changes: &mut TrafficChanges,
    ) {
        if self
            .targets
            .get(&target.id)
            .is_some_and(|previous| at < previous.observed_at)
        {
            return;
        }
        let mut position_epoch = reference.source().zip(reference.epoch());
        let mut position_observed_at = at;
        if let Some(source) = reference.source()
            && let Some(previous) = self.targets.get(&target.id)
            && let Some((previous_source, previous_epoch)) = previous.position_epoch
            && source == previous_source
            && at >= previous.observed_at
        {
            let hold_missing_reference = reference.epoch().is_none()
                && at.saturating_since(previous.position_observed_at) < REFERENCE_HOLD;
            if reference.epoch() == Some(previous_epoch) || hold_missing_reference {
                target.position = previous.target.position;
                target.track = previous.target.track;
                position_epoch = previous.position_epoch;
                position_observed_at = previous.position_observed_at;
            } else if reference
                .epoch()
                .is_some_and(|epoch| epoch > previous_epoch)
                && at.saturating_since(previous.observed_at) < STALE_AFTER
            {
                let (distance, bearing) =
                    previous.target.position.distance_bearing(target.position);
                // Small steps give an unstable bearing at FLARM's meter resolution.
                if distance >= Length::from_meters(5.) {
                    target.track = Some(bearing);
                }
            }
        }
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
                position_epoch,
                position_observed_at,
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

    pub fn reset_climb(&mut self) -> TrafficChanges {
        self.climbs.clear();
        let mut changes = TrafficChanges::default();
        for stored in self.targets.values_mut() {
            if stored.target.climb.take().is_some() {
                changes.upsert(stored.target);
            }
        }
        changes
    }

    pub fn snapshot(&self) -> Vec<TrafficTarget> {
        self.targets.values().map(|stored| stored.target).collect()
    }

    pub fn published_targets(&self, database: &FlarmnetDatabase) -> Vec<PublishedTrafficTarget> {
        self.targets
            .values()
            .map(|stored| {
                stored
                    .target
                    .publish(database, self.broadcast_identities.get(&stored.target.id))
            })
            .collect()
    }

    pub fn published_delta(
        &self,
        changes: TrafficChanges,
        database: &FlarmnetDatabase,
    ) -> Option<TrafficDelta> {
        if changes.upserts.is_empty() && changes.removed.is_empty() {
            return None;
        }

        Some(TrafficDelta {
            upserts: changes
                .upserts
                .into_values()
                .map(|target| target.publish(database, self.broadcast_identities.get(&target.id)))
                .collect(),
            removed: changes
                .removed
                .into_iter()
                .map(|id| id.to_string())
                .collect(),
        })
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
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct FlarmBroadcastIdentity {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub registration: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub pilot_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub aircraft_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub callsign: Option<String>,
}

impl FlarmBroadcastIdentity {
    fn new(identity: PflamIdentity) -> Option<Self> {
        let mut result = Self {
            registration: None,
            pilot_name: None,
            aircraft_type: None,
            callsign: None,
        };
        result.update(identity).then_some(result)
    }

    fn update(&mut self, identity: PflamIdentity) -> bool {
        let (field, value) = match identity {
            PflamIdentity::Registration(value) => (&mut self.registration, value),
            PflamIdentity::PilotName(value) => (&mut self.pilot_name, value),
            PflamIdentity::AircraftType(value) => (&mut self.aircraft_type, value),
            PflamIdentity::Callsign(value) => (&mut self.callsign, value),
            _ => return false,
        };
        if field.as_deref() == Some(&value) {
            return false;
        }
        *field = Some(value.into());
        true
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct PublishedTrafficTarget {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub broadcast_identity: Option<FlarmBroadcastIdentity>,
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
            broadcast_identity: None,
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
    fn publish(
        self,
        database: &FlarmnetDatabase,
        broadcast_identity: Option<&FlarmBroadcastIdentity>,
    ) -> PublishedTrafficTarget {
        let id = self.id;
        let flarmnet = match id.id_type {
            TrafficTargetIdType::Flarm | TrafficTargetIdType::Icao => database.lookup(id.value),
            TrafficTargetIdType::Random | TrafficTargetIdType::Other(_) => None,
        };
        PublishedTrafficTarget {
            broadcast_identity: broadcast_identity.cloned(),
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
