use crate::{GpsInstruments, LatLon};
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
    Waypoint {
        name: String,
        latitude_degrees: f64,
        longitude_degrees: f64,
        elevation_meters: f64,
    },
}

impl NavigationTarget {
    pub fn position(&self) -> LatLon {
        match *self {
            Self::Waypoint {
                latitude_degrees,
                longitude_degrees,
                ..
            } => LatLon {
                latitude_degrees,
                longitude_degrees,
            },
        }
    }

    pub fn validate(&self) -> Result<(), &'static str> {
        let position = self.position();
        let Self::Waypoint {
            elevation_meters, ..
        } = self;
        if !(-90. ..=90.).contains(&position.latitude_degrees)
            || !(-180. ..=180.).contains(&position.longitude_degrees)
            || !elevation_meters.is_finite()
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
    pub position: LatLon,
    pub guidance: Option<NavigationGuidance>,
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

impl Navigation {
    pub fn new(target: NavigationTarget, gps: Option<GpsInstruments>) -> Self {
        let position = target.position();
        let guidance = gps.map(|gps| {
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
                stale: gps.stale,
            }
        });
        Self {
            target,
            position,
            guidance,
        }
    }
}
