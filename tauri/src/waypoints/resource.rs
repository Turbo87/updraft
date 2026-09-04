use crate::driver::DriverHandle;
use serde_json::{Value, json};
use tauri::{
    Manager,
    http::{Response, StatusCode},
};
use updraft_core::{GetWaypointSnapshot, WaypointCatalog};

pub async fn waypoint_resource_response<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
) -> Response<Vec<u8>> {
    let handle = app.state::<DriverHandle>().inner().clone();
    let snapshot = match handle.send(GetWaypointSnapshot).await {
        Ok(snapshot) => snapshot,
        Err(error) => {
            tracing::warn!(%error, "Could not serve waypoint GeoJSON because the driver stopped");
            return Response::builder()
                .status(StatusCode::SERVICE_UNAVAILABLE)
                .body(Vec::new())
                .unwrap();
        }
    };
    let body = waypoint_geojson(snapshot.generation, &snapshot.catalog);
    Response::builder()
        .header("content-type", "application/geo+json")
        .header("cache-control", "no-store")
        .body(serde_json::to_vec(&body).expect("GeoJSON values serialize"))
        .unwrap()
}

pub fn waypoint_geojson(generation: u64, catalog: &WaypointCatalog) -> Value {
    let mut features = Vec::new();
    for (source_index, (name, dataset)) in catalog.sources.iter().enumerate() {
        let Ok(dataset) = dataset else {
            continue;
        };
        for (index, point) in dataset.waypoints().iter().enumerate() {
            let id = format!("{generation}:{source_index}:{index}");
            let mut properties = json!({
                "id": id, "sourceName": name, "name": point.name,
                "kind": point.kind as u8, "elevationMeters": point.elevation.into_inner().as_meters(),
                "frequency": point.frequency, "notes": point.notes,
            });
            if let Some(direction) = point.runway_direction {
                properties["runwayDirection"] = json!(direction);
            }
            if let Some(length) = point.runway_length {
                properties["runwayLengthMeters"] = json!(length.as_meters());
            }
            if let Some(width) = point.runway_width {
                properties["runwayWidthMeters"] = json!(width.as_meters());
            }
            features.push(json!({"type":"Feature", "id":id, "properties":properties,
                "geometry":{"type":"Point", "coordinates":point.position.to_geojson_coordinate()}}));
        }
    }
    json!({"type":"FeatureCollection", "features":features})
}

#[cfg(test)]
mod tests {
    use super::*;
    use claims::assert_ok;
    use std::collections::BTreeMap;
    use std::sync::Arc;
    use updraft_waypoint::WaypointDataset;

    #[test]
    fn projection_keeps_source_identity_and_waypoint_details() {
        let bytes = b"name,code,country,lat,lon,elev,style,rwdir,rwlen,freq,desc\nField,,,5000.000N,00600.000E,100m,2,90,800m,123.500,Notes\n";
        let dataset = Arc::new(assert_ok!(WaypointDataset::from_cup(bytes)));
        let catalog = WaypointCatalog {
            sources: BTreeMap::from([
                ("a.cup".into(), Ok(dataset.clone())),
                ("b.cup".into(), Ok(dataset)),
            ]),
        };
        let value = waypoint_geojson(7, &catalog);
        assert_eq!(value["features"][0]["id"], "7:0:0");
        assert_eq!(value["features"][1]["id"], "7:1:0");
        insta::assert_json_snapshot!(value["features"][0]["properties"]);
        assert_eq!(waypoint_geojson(8, &catalog)["features"][0]["id"], "8:0:0");
    }
    #[tokio::test(flavor = "multi_thread")]
    async fn resource_uses_current_core_generation_and_disables_caching() {
        use crate::driver::Driver;
        use updraft_core::{AirspaceState, ReplaceWaypointCatalog, SettingsSnapshot};
        let handle = Driver::spawn(
            SettingsSnapshot::default(),
            AirspaceState::none_at_startup(),
            Box::new(|_, _, _| Box::new(|| {})),
            Box::new(|_| {}),
            std::time::Duration::from_millis(100),
        );
        let app = tauri::test::mock_app();
        app.manage(handle.clone());
        let response = waypoint_resource_response(app.handle().clone()).await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers()["cache-control"], "no-store");
        assert_eq!(response.headers()["content-type"], "application/geo+json");
        assert_eq!(
            assert_ok!(serde_json::from_slice::<Value>(response.body()))["features"],
            json!([])
        );
        let bytes = b"name,code,country,lat,lon,elev,style\nField,,,5000.000N,00600.000E,100m,2\n";
        let catalog = Arc::new(WaypointCatalog {
            sources: BTreeMap::from([(
                "a.cup".into(),
                Ok(Arc::new(assert_ok!(WaypointDataset::from_cup(bytes)))),
            )]),
        });
        assert_ok!(handle.send(ReplaceWaypointCatalog(catalog)).await);
        let response = waypoint_resource_response(app.handle().clone()).await;
        assert_eq!(
            assert_ok!(serde_json::from_slice::<Value>(response.body()))["features"][0]["id"],
            "1:0:0"
        );
    }

    #[tokio::test]
    #[tracing_test::traced_test]
    async fn stopped_driver_returns_a_resource_error() {
        let app = tauri::test::mock_app();
        app.manage(DriverHandle::stopped());
        assert_eq!(
            waypoint_resource_response(app.handle().clone())
                .await
                .status(),
            StatusCode::SERVICE_UNAVAILABLE
        );
        assert!(logs_contain(
            "Could not serve waypoint GeoJSON because the driver stopped"
        ));
    }
}
