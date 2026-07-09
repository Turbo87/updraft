use std::time::Duration;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tempfile::TempDir;
use tower::ServiceExt;
use updraft_core::{
    App, CoreRuntime, CoreRuntimeHandle, FlightInput, Input, MonotonicTime, ObservationSource,
    OwnshipPosition, PositionObservation, Snapshot, StateMessage,
};
use updraft_geo::LatLon;
use updraft_units::Angle;

const INDEX_HTML: &str = "<!doctype html><title>updraft fixture</title>";

/// Creates a runtime and a throwaway static directory containing a known
/// `index.html`. The returned `TempDir` must stay in scope for the duration of
/// the test, as dropping it deletes the directory.
fn fixture() -> (TempDir, CoreRuntimeHandle) {
    let dir = tempfile::tempdir().expect("create tempdir");
    std::fs::write(dir.path().join("index.html"), INDEX_HTML).expect("write index.html");
    let runtime = CoreRuntime::spawn(App::default());
    (dir, runtime)
}

fn app_with_fixture() -> (TempDir, axum::Router) {
    let (dir, runtime) = fixture();
    let app = updraft_server::router(dir.path(), runtime);
    (dir, app)
}

fn simulation_app_with_fixture() -> (TempDir, CoreRuntimeHandle, axum::Router) {
    let (dir, runtime) = fixture();
    let app = updraft_server::simulation_router(dir.path(), runtime.clone());
    (dir, runtime, app)
}

async fn body_bytes(response: axum::http::Response<Body>) -> Vec<u8> {
    response
        .into_body()
        .collect()
        .await
        .unwrap()
        .to_bytes()
        .to_vec()
}

#[tokio::test]
async fn health_returns_ok_with_empty_body() {
    let (_dir, app) = app_with_fixture();

    let uri = "/api/health";
    let request = Request::builder().uri(uri).body(Body::empty()).unwrap();
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert!(body_bytes(response).await.is_empty());
}

#[tokio::test]
async fn unknown_route_serves_spa_index() {
    let (_dir, app) = app_with_fixture();

    for uri in ["/", "/map"] {
        let request = Request::builder().uri(uri).body(Body::empty()).unwrap();
        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK, "uri = {uri}");
        assert_eq!(
            body_bytes(response).await,
            INDEX_HTML.as_bytes(),
            "uri = {uri}"
        );
    }
}

#[tokio::test]
async fn unknown_api_route_returns_not_found() {
    let (_dir, app) = app_with_fixture();

    let uri = "/api/unknown";
    let request = Request::builder().uri(uri).body(Body::empty()).unwrap();
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert!(body_bytes(response).await.is_empty());
}

#[tokio::test]
async fn state_stream_begins_with_the_current_snapshot() {
    let (dir, runtime) = fixture();
    let app = updraft_server::router(dir.path(), runtime);
    let request = Request::builder()
        .uri("/api/state")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()["content-type"], "text/event-stream");
    let frame = response
        .into_body()
        .frame()
        .await
        .expect("state stream ended before snapshot")
        .unwrap();
    assert_eq!(
        frame.into_data().unwrap(),
        "event: snapshot\ndata: {\"position\":null}\n\n"
    );
}

#[tokio::test]
async fn state_stream_publishes_position_changes() {
    let (dir, runtime) = fixture();
    let app = updraft_server::router(dir.path(), runtime.clone());
    let request = Request::builder()
        .uri("/api/state")
        .body(Body::empty())
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    let mut body = response.into_body();
    body.frame()
        .await
        .expect("state stream ended before snapshot")
        .unwrap();
    let position = PositionObservation::new(
        ObservationSource::Simulation,
        MonotonicTime::from_duration(Duration::from_secs(1)),
        LatLon::from_degrees(50.823, 6.186),
        Some(Angle::from_degrees(45.)),
    )
    .unwrap();

    runtime
        .submit(Input::Flight(FlightInput::PositionObserved(position)))
        .await
        .unwrap();

    let frame = body
        .frame()
        .await
        .expect("state stream ended before position change")
        .unwrap();
    assert_eq!(
        frame.into_data().unwrap(),
        concat!(
            "event: changes\n",
            "data: [{\"flight\":{\"position_changed\":{",
            "\"location\":{\"latitude\":50.823,\"longitude\":6.186},",
            "\"track\":45.0}}}]\n\n"
        )
    );
}

#[tokio::test]
async fn simulation_position_endpoint_submits_an_observation() {
    let (_dir, runtime, app) = simulation_app_with_fixture();
    let request = Request::builder()
        .method("POST")
        .uri("/api/simulation/position")
        .header("content-type", "application/json")
        .body(Body::from(
            r#"{"latitude":50.823,"longitude":6.186,"track":45}"#,
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    let mut stream = runtime.subscribe().await.unwrap();
    assert_eq!(
        stream.recv().await,
        Some(StateMessage::Snapshot(Snapshot {
            position: Some(OwnshipPosition::new(
                LatLon::from_degrees(50.823, 6.186),
                Some(Angle::from_degrees(45.))
            ))
        }))
    );
}

#[tokio::test]
async fn simulation_position_endpoint_rejects_invalid_coordinates() {
    let (_dir, _runtime, app) = simulation_app_with_fixture();
    let request = Request::builder()
        .method("POST")
        .uri("/api/simulation/position")
        .header("content-type", "application/json")
        .body(Body::from(r#"{"latitude":91,"longitude":6.186}"#))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert!(body_bytes(response).await.is_empty());
}

#[tokio::test]
async fn normal_router_does_not_expose_simulation_position_endpoint() {
    let (_dir, app) = app_with_fixture();
    let request = Request::builder()
        .method("POST")
        .uri("/api/simulation/position")
        .header("content-type", "application/json")
        .body(Body::from(
            r#"{"latitude":50.823,"longitude":6.186,"track":45}"#,
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert!(body_bytes(response).await.is_empty());
}
