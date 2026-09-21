use crate::{
    AltitudeInstrument, GlideSnapshot, LatLon, PublishedTrafficTarget, Timestamp, TrafficTargetId,
};
use serde::{Deserialize, Serialize};
use updraft_geo::LatLon as Position;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum NavigationTarget {
    Task,
    Traffic {
        #[cfg_attr(feature = "ts", ts(type = "string"))]
        id: TrafficTargetId,
    },
    MapPosition {
        latitude_degrees: f64,
        longitude_degrees: f64,
    },
    Waypoint {
        name: String,
        latitude_degrees: f64,
        longitude_degrees: f64,
        elevation_meters: f64,
    },
}

impl NavigationTarget {
    pub fn matches(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Task, Self::Task) => return true,
            (Self::Traffic { id }, Self::Traffic { id: other }) => return id == other,
            (Self::Waypoint { name, .. }, Self::Waypoint { name: other, .. }) if name == other => {}
            (Self::MapPosition { .. }, Self::MapPosition { .. }) => {}
            _ => return false,
        }
        let Some((a, b)) = self.position().zip(other.position()) else {
            return false;
        };
        let longitude = (a.longitude_degrees - b.longitude_degrees).abs();
        (a.latitude_degrees - b.latitude_degrees).abs() <= 0.0001
            && longitude.min(360. - longitude) <= 0.0001
    }

    pub fn position(&self) -> Option<LatLon> {
        match *self {
            Self::Traffic { .. } | Self::Task => None,
            Self::MapPosition {
                latitude_degrees,
                longitude_degrees,
            }
            | Self::Waypoint {
                latitude_degrees,
                longitude_degrees,
                ..
            } => Some(LatLon {
                latitude_degrees,
                longitude_degrees,
            }),
        }
    }

    pub fn validate(&self) -> Result<(), &'static str> {
        let Some(position) = self.position() else {
            return Ok(());
        };
        let invalid_elevation = matches!(self, Self::Waypoint { elevation_meters, .. } if !elevation_meters.is_finite());
        if !(-90. ..=90.).contains(&position.latitude_degrees)
            || !(-180. ..=180.).contains(&position.longitude_degrees)
            || invalid_elevation
        {
            return Err("Invalid navigation target");
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct Navigation {
    pub target: NavigationTarget,
    pub position: Option<LatLon>,
    pub guidance: Option<NavigationGuidance>,
    pub arrival: Option<NavigationArrival>,
    pub traffic: Option<NavigationTraffic>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub traffic_name: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct NavigationGuidance {
    pub distance_meters: f64,
    pub bearing_degrees: f64,
    pub relative_bearing_degrees: Option<f64>,
    pub stale: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct NavigationArrival {
    pub margin_meters: f64,
    pub stale: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct NavigationTraffic {
    pub name: String,
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub age_seconds: u64,
    pub stale: bool,
    pub relative_altitude: Option<AltitudeInstrument>,
}

impl Navigation {
    pub fn resolve_traffic_name(&mut self, database: &updraft_flarmnet::FlarmnetDatabase) {
        let NavigationTarget::Traffic { id } = self.target else {
            return;
        };
        if !matches!(
            id.id_type,
            crate::TrafficTargetIdType::Flarm | crate::TrafficTargetIdType::Icao
        ) {
            return;
        }
        self.traffic_name = database.lookup(id.value).and_then(|record| {
            [&record.call_sign, &record.registration]
                .into_iter()
                .find(|name| !name.is_empty())
                .cloned()
        });
    }

    pub fn new(
        target: NavigationTarget,
        glide: &GlideSnapshot,
        terrain_elevation: Option<f64>,
        report: Option<&(PublishedTrafficTarget, Timestamp)>,
        at: Timestamp,
    ) -> Self {
        let traffic = report.map(|(report, observed_at)| {
            let age = at.saturating_since(*observed_at);
            let stale = report.stale || age >= crate::traffic::STALE_AFTER;
            let name = report
                .broadcast_identity
                .as_ref()
                .and_then(|identity| {
                    identity
                        .callsign
                        .as_ref()
                        .or(identity.registration.as_ref())
                })
                .or_else(|| {
                    report.flarmnet.as_ref().map(|record| {
                        if record.call_sign.is_empty() {
                            &record.registration
                        } else {
                            &record.call_sign
                        }
                    })
                })
                .filter(|name| !name.is_empty())
                .cloned()
                .unwrap_or_else(|| report.id.clone());
            let relative_altitude = glide
                .instruments
                .derived
                .as_ref()
                .and_then(|derived| derived.altitude)
                .zip(report.altitude_msl_meters)
                .map(|(ownship, target)| AltitudeInstrument {
                    meters: target - ownship.altitude_msl_meters,
                    stale: stale || ownship.stale,
                });
            NavigationTraffic {
                name,
                age_seconds: age.as_secs(),
                stale,
                relative_altitude,
            }
        });
        let position = target
            .position()
            .or_else(|| report.map(|(report, _)| report.position));
        let guidance = glide.instruments.gps.zip(position).map(|(gps, position)| {
            let ownship = Position::from_degrees(
                gps.position.latitude_degrees,
                gps.position.longitude_degrees,
            );
            let destination =
                Position::from_degrees(position.latitude_degrees, position.longitude_degrees);
            let (distance, bearing) = ownship.distance_bearing(destination);
            let bearing_degrees = bearing.as_degrees().rem_euclid(360.);
            NavigationGuidance {
                distance_meters: distance.as_meters(),
                bearing_degrees,
                relative_bearing_degrees: gps
                    .track_degrees
                    .map(|track| (bearing_degrees - track + 180.).rem_euclid(360.) - 180.),
                stale: gps.stale || traffic.as_ref().is_some_and(|traffic| traffic.stale),
            }
        });
        let elevation = match target {
            NavigationTarget::Waypoint {
                elevation_meters, ..
            } => Some(elevation_meters),
            NavigationTarget::MapPosition { .. } => terrain_elevation,
            NavigationTarget::Traffic { .. } | NavigationTarget::Task => None,
        };
        let arrival = elevation
            .zip(position)
            .and_then(|(elevation_meters, position)| {
                glide
                    .arrival_at_position(
                        Position::from_degrees(
                            position.latitude_degrees,
                            position.longitude_degrees,
                        ),
                        updraft_units::Length::from_meters(elevation_meters),
                    )
                    .map(|arrival| NavigationArrival {
                        margin_meters: arrival.margin.as_meters(),
                        stale: arrival.stale,
                    })
            });
        Self {
            traffic_name: None,
            traffic,
            arrival,
            target,
            position,
            guidance,
        }
    }
}
