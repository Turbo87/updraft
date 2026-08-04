use crate::driver::DriverHandle;
use serde_json::{Value, json};
use tauri::http::{Response, StatusCode, header};
use updraft_airspace::{Airspace, AirspaceDataset};
use updraft_core::GetAirspaceSnapshot;

/// Builds a `GeoJSON` response from the active airspace dataset.
pub async fn airspace_resource_response(handle: DriverHandle) -> Response<Vec<u8>> {
    let dataset = match handle.send(GetAirspaceSnapshot).await {
        Ok(dataset) => dataset,
        Err(error) => {
            tracing::warn!(%error, "Could not serve airspace GeoJSON because the driver stopped");
            return Response::builder()
                .status(StatusCode::SERVICE_UNAVAILABLE)
                .body(Vec::new())
                .expect("the fixed unavailable response should be valid");
        }
    };
    let body = serde_json::to_vec(&airspace_geojson(dataset.as_deref()))
        .expect("canonical airspace should serialize as GeoJSON");
    Response::builder()
        .header(header::CONTENT_TYPE, "application/geo+json")
        .header(header::CACHE_CONTROL, "no-store")
        .body(body)
        .expect("the fixed GeoJSON response should be valid")
}

fn airspace_geojson(dataset: Option<&AirspaceDataset>) -> Value {
    let features = dataset
        .into_iter()
        .flat_map(AirspaceDataset::airspaces)
        .map(Airspace::to_geojson)
        .collect::<Vec<_>>();

    json!({
        "type": "FeatureCollection",
        "features": features,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::driver::Driver;
    use std::sync::Arc;
    use std::time::Duration;
    use tauri::http::{StatusCode, header};
    use tracing_test::traced_test;
    use updraft_core::{ActivateAirspaceDataset, AirspaceState, SettingsSnapshot};

    const POLYGON: &[u8] = include_bytes!("../../testdata/airspace/polygon.txt");
    const CIRCLE: &[u8] = include_bytes!("../../testdata/airspace/circle.txt");

    fn driver(airspace: AirspaceState) -> DriverHandle {
        Driver::spawn(
            SettingsSnapshot::default(),
            airspace,
            Box::new(|_, _, _| Box::new(|| {})),
            Box::new(|_| {}),
            Duration::from_millis(100),
        )
    }

    #[test]
    fn projects_airspace_as_a_closed_geojson_polygon() {
        let dataset = AirspaceDataset::from_openair(POLYGON).expect("a valid OpenAir fixture");

        let geojson = airspace_geojson(Some(&dataset));

        insta::assert_json_snapshot!(geojson, @r#"
        {
          "features": [
            {
              "geometry": {
                "coordinates": [
                  [
                    [
                      10.0,
                      50.0
                    ],
                    [
                      10.016666666666667,
                      50.0
                    ],
                    [
                      10.016666666666667,
                      50.016666666666666
                    ],
                    [
                      10.0,
                      50.016666666666666
                    ],
                    [
                      10.0,
                      50.0
                    ]
                  ]
                ],
                "type": "Polygon"
              },
              "id": 0,
              "properties": {
                "class": 3,
                "type": 0
              },
              "type": "Feature"
            }
          ],
          "type": "FeatureCollection"
        }
        "#);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn inactive_airspace_returns_an_empty_collection_without_caching() {
        let handle = driver(AirspaceState::none_at_startup());

        let response = airspace_resource_response(handle).await;

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers()[header::CONTENT_TYPE],
            "application/geo+json"
        );
        assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
        let body: Value =
            serde_json::from_slice(response.body()).expect("a valid GeoJSON response");
        insta::assert_json_snapshot!(body, @r#"
        {
          "features": [],
          "type": "FeatureCollection"
        }
        "#);
    }

    #[test]
    fn projects_required_openaip_classification_properties() {
        let legacy_class = b"AC R\nAL GND\nAH FL100\nDP 50:00:00 N 010:00:00 E\nDP 50:00:00 N 010:01:00 E\nDP 50:01:00 N 010:00:00 E\n";
        let dataset =
            AirspaceDataset::from_openair(legacy_class).expect("a valid legacy OpenAir source");

        let properties = &airspace_geojson(Some(&dataset))["features"][0]["properties"];

        assert_eq!(
            properties,
            &json!({
                "class": 8,
                "type": 1,
            })
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn replacement_request_receives_the_latest_dataset_snapshot() {
        let initial = Arc::new(
            AirspaceDataset::from_openair(POLYGON).expect("a valid initial OpenAir fixture"),
        );
        let handle = driver(AirspaceState::active_at_startup(initial, None));
        let initial_response = airspace_resource_response(handle.clone()).await;

        let replacement = Arc::new(
            AirspaceDataset::from_openair(CIRCLE).expect("a valid replacement OpenAir fixture"),
        );
        handle
            .send(ActivateAirspaceDataset::new(replacement, None))
            .await
            .expect("the active driver should accept the replacement");
        let replacement_response = airspace_resource_response(handle).await;

        let initial_body: Value = serde_json::from_slice(initial_response.body())
            .expect("valid initial airspace GeoJSON");
        let replacement_body: Value = serde_json::from_slice(replacement_response.body())
            .expect("valid replacement airspace GeoJSON");
        assert_eq!(initial_body["features"][0]["properties"]["class"], 3);
        assert_eq!(replacement_body["features"][0]["properties"]["class"], 2);
    }

    #[tokio::test]
    #[traced_test]
    async fn stopped_driver_returns_service_unavailable() {
        let response = airspace_resource_response(DriverHandle::stopped()).await;

        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
        assert!(logs_contain(
            "Could not serve airspace GeoJSON because the driver stopped"
        ));
    }
}
