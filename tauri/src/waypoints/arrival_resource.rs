use serde_json::json;
use updraft_core::GlideSnapshot;
use updraft_geo::BoundingBox;

/// Serialized map data built from the same catalog and inputs as its arrivals.
#[derive(Debug)]
pub struct ArrivalResource {
    pub generation: u64,
    pub body: Vec<u8>,
}

impl ArrivalResource {
    pub fn calculate(snapshot: &GlideSnapshot, bounds: BoundingBox) -> serde_json::Result<Self> {
        let arrivals = snapshot.arrivals_in(bounds);
        let sources: Vec<_> = snapshot.waypoints.catalog.sources.values().collect();
        let mut features = Vec::with_capacity(arrivals.entries.len());
        for entry in arrivals.entries {
            let dataset = sources[entry.source_index]
                .as_ref()
                .expect("arrival source is loaded");
            let point = &dataset.waypoints()[entry.waypoint_index];
            let id = format!(
                "{}:{}:{}",
                arrivals.generation, entry.source_index, entry.waypoint_index
            );
            let mut properties = json!({"name": point.name, "kind": point.kind as u8});
            properties["catalogGeneration"] = json!(arrivals.generation);
            if let Some(direction) = point.runway_direction {
                properties["runwayDirection"] = json!(direction);
            }
            if let Some(arrival) = entry.arrival {
                let margin = arrival.margin.as_meters();
                let status = if margin >= 0. {
                    "reachable"
                } else if margin + snapshot.arrival_reserve.meters() > 0. {
                    "belowReserve"
                } else {
                    "unreachable"
                };
                properties["arrivalMarginMeters"] = json!(margin);
                properties["arrivalStale"] = json!(arrival.stale);
                properties["arrivalStatus"] = json!(status);
            }
            features.push(json!({
                "type": "Feature", "id": id, "properties": properties,
                "geometry": {"type": "Point", "coordinates": point.position.to_geojson_coordinate()},
            }));
        }
        let geojson = json!({"type": "FeatureCollection", "features": features});
        Ok(Self {
            generation: arrivals.generation,
            body: serde_json::to_vec(&geojson)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use claims::{assert_none, assert_ok, assert_some};
    use serde_json::Value;
    use std::{collections::BTreeMap, sync::Arc};
    use updraft_core::{
        Core, Fix, GetGlideSnapshot, InternalGps, SettingsSnapshot, Timestamp, WaypointCatalog,
        WaypointLoadError,
    };
    use updraft_geo::LatLon;
    use updraft_units::{Angle, EllipsoidAltitude, Length};
    use updraft_waypoint::WaypointDataset;

    #[test]
    fn resource_preserves_feature_identity_and_exact_arrival_boundaries() {
        let mut core = Core::new(SettingsSnapshot::default());
        let at = Timestamp::from_millis(0);
        let fix = Fix {
            position: LatLon::from_degrees(0., 0.),
            altitude_ellipsoid: Some(EllipsoidAltitude::new(Length::from_meters(1000.))),
            track: None,
            ground_speed: None,
            fix_time: None,
        };
        core.apply(InternalGps::new(fix), at);
        let mut snapshot = core.apply(GetGlideSnapshot, at).response;
        let cup =
            b"name,code,country,lat,lon,elev,style,rwdir\nField,,,0000.000N,00000.000E,100m,2,90\n";
        let dataset = Arc::new(assert_ok!(WaypointDataset::from_cup(cup)));
        snapshot.waypoints.generation = 7;
        snapshot.waypoints.catalog = Arc::new(WaypointCatalog {
            sources: BTreeMap::from([
                ("a.cup".into(), Err(WaypointLoadError::ReadFailed)),
                ("b.cup".into(), Ok(dataset)),
            ]),
        });
        let bounds = BoundingBox::new(Angle::ZERO, Angle::ZERO, Angle::ZERO, Angle::ZERO);
        for (altitude, status) in [
            (300., "reachable"),
            (299.9, "belowReserve"),
            (100.1, "belowReserve"),
            (100., "unreachable"),
            (99.9, "unreachable"),
        ] {
            let derived = assert_some!(snapshot.instruments.derived.as_mut());
            assert_some!(derived.altitude.as_mut()).altitude_msl_meters = altitude;
            let resource = assert_ok!(ArrivalResource::calculate(&snapshot, bounds));
            assert_eq!(resource.generation, 7);
            let geojson: Value = assert_ok!(serde_json::from_slice(&resource.body));
            let properties = &geojson["features"][0]["properties"];
            assert_eq!(properties["catalogGeneration"], 7);
            assert_eq!(properties["arrivalStatus"], status);
            assert_eq!(properties["arrivalMarginMeters"], altitude - 300.);
            if altitude == 300. {
                insta::assert_json_snapshot!(geojson);
            }
        }
        assert_some!(snapshot.instruments.gps.as_mut()).stale = true;
        let stale = assert_ok!(ArrivalResource::calculate(&snapshot, bounds));
        let stale: Value = assert_ok!(serde_json::from_slice(&stale.body));
        assert_eq!(stale["features"][0]["properties"]["arrivalStale"], true);
        snapshot.instruments.gps = None;
        let unavailable = assert_ok!(ArrivalResource::calculate(&snapshot, bounds));
        let unavailable: Value = assert_ok!(serde_json::from_slice(&unavailable.body));
        let properties = &unavailable["features"][0]["properties"];
        assert_eq!(unavailable["features"][0]["id"], "7:1:0");
        assert_none!(properties.get("id"));
        assert_none!(properties.get("arrivalMarginMeters"));
        assert_none!(properties.get("arrivalStatus"));
        assert_none!(properties.get("arrivalStale"));
    }
}
