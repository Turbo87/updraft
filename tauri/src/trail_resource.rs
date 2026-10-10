use crate::flight_recording::{FlightRecordingPath, trail_samples};
use tauri::http::{Response, StatusCode, header};
use tauri::{AppHandle, Manager};

/// Serves the flight recording samples with a UTC at or after the `since`
/// query value in Unix milliseconds.
pub async fn trail_resource_response<R: tauri::Runtime>(
    app: AppHandle<R>,
    query: Option<String>,
) -> Response<Vec<u8>> {
    let Some(since) = query.as_deref().and_then(since) else {
        return empty_response(StatusCode::BAD_REQUEST);
    };
    let path = app.state::<FlightRecordingPath>().0.clone();
    let samples = tauri::async_runtime::spawn_blocking(move || trail_samples(&path, since)).await;
    match samples {
        Ok(Ok(samples)) => Response::builder()
            .header(header::CONTENT_TYPE, "application/json")
            .header(header::CACHE_CONTROL, "no-store")
            .body(serde_json::to_vec(&samples).expect("trail samples should serialize as JSON"))
            .expect("the fixed JSON response should be valid"),
        Ok(Err(error)) => {
            tracing::warn!(%error, "Could not read the flight recording for the trail resource");
            empty_response(StatusCode::SERVICE_UNAVAILABLE)
        }
        Err(error) => {
            tracing::warn!(%error, "Trail resource worker failed");
            empty_response(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

fn since(query: &str) -> Option<i64> {
    query
        .split('&')
        .find_map(|pair| pair.strip_prefix("since="))?
        .parse()
        .ok()
}

fn empty_response(status: StatusCode) -> Response<Vec<u8>> {
    Response::builder()
        .status(status)
        .body(Vec::new())
        .expect("the fixed empty response should be valid")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flight_recording::FlightRecording;
    use crate::flight_recording::RecordingWrite::{RecordSample, StartRecording};
    use crate::flight_recording::tests::{COVERED, SYSTEM_UTC, sample, terrain};
    use claims::assert_ok;
    use serde_json::Value;
    use tauri::test::mock_app;
    use tracing_test::traced_test;

    async fn request(path: &std::path::Path, query: Option<&str>) -> Response<Vec<u8>> {
        let app = mock_app();
        app.manage(FlightRecordingPath(path.to_owned()));
        trail_resource_response(app.handle().clone(), query.map(str::to_owned)).await
    }

    #[tokio::test]
    async fn serves_the_samples_since_the_requested_utc() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("state.sqlite");
        let mut recording = assert_ok!(FlightRecording::open(&path, terrain(directory.path())));
        let uncovered = (50.0, 20.0);
        let mut without_altitude = sample(4_000, COVERED);
        without_altitude.altitude_msl = None;
        without_altitude.relative_vario = None;
        assert_ok!(recording.write(&StartRecording(sample(1_000, COVERED)), SYSTEM_UTC));
        assert_ok!(recording.write(&RecordSample(sample(2_000, COVERED)), SYSTEM_UTC));
        assert_ok!(recording.write(&RecordSample(sample(3_000, uncovered)), SYSTEM_UTC));
        assert_ok!(recording.write(&RecordSample(without_altitude), SYSTEM_UTC));

        let response = request(&path, Some("since=2000")).await;

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers()[header::CONTENT_TYPE], "application/json");
        assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
        let body: Value = assert_ok!(serde_json::from_slice(response.body()));
        insta::assert_json_snapshot!(body, @r#"
        [
          {
            "altitudeAglMeters": 1100.0,
            "altitudeMslMeters": 1200.0,
            "nettoMetersPerSecond": 2.0,
            "position": {
              "latitudeDegrees": 46.0,
              "longitudeDegrees": 2.0
            },
            "relativeVarioMetersPerSecond": 1.0,
            "unixMilliseconds": 2000,
            "varioMetersPerSecond": 1.5
          },
          {
            "altitudeAglMeters": 1200.0,
            "altitudeMslMeters": 1200.0,
            "nettoMetersPerSecond": 2.0,
            "position": {
              "latitudeDegrees": 50.0,
              "longitudeDegrees": 20.0
            },
            "relativeVarioMetersPerSecond": 1.0,
            "unixMilliseconds": 3000,
            "varioMetersPerSecond": 1.5
          },
          {
            "altitudeAglMeters": null,
            "altitudeMslMeters": null,
            "nettoMetersPerSecond": 2.0,
            "position": {
              "latitudeDegrees": 46.0,
              "longitudeDegrees": 2.0
            },
            "relativeVarioMetersPerSecond": null,
            "unixMilliseconds": 4000,
            "varioMetersPerSecond": 1.5
          }
        ]
        "#);
    }

    #[tokio::test]
    async fn rejects_a_request_without_a_valid_since_value() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("state.sqlite");
        assert_ok!(FlightRecording::open(&path, terrain(directory.path())));

        for query in [
            None,
            Some(""),
            Some("since="),
            Some("since=1.5"),
            Some("from=0"),
        ] {
            let response = request(&path, query).await;
            assert_eq!(response.status(), StatusCode::BAD_REQUEST, "{query:?}");
        }
    }

    #[tokio::test]
    #[traced_test]
    async fn missing_flight_recording_returns_service_unavailable() {
        let directory = tempfile::tempdir().unwrap();

        let response = request(&directory.path().join("state.sqlite"), Some("since=0")).await;

        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
        assert!(logs_contain(
            "Could not read the flight recording for the trail resource"
        ));
    }
}
