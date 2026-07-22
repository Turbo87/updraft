use axum::body::Body;
use axum::http::{Request, StatusCode};
use claims::{assert_none, assert_ok, assert_ok_eq, assert_some};
use futures_util::FutureExt;
use http_body_util::BodyExt;
use std::time::Duration;
use std::{sync::mpsc, thread};
use tempfile::TempDir;
use tower::ServiceExt;
use updraft_core::device::DeviceId;
use updraft_core::flight::{FlightInput, GnssUpdate, Observation, Sourced};
use updraft_core::{App, Input};
use updraft_geo::LatLon;
use updraft_runtime::Runtime;
use updraft_units::{Angle, Length, MslAltitude, PressureAltitude, Speed};

const INDEX_HTML: &str = "<!doctype html><title>updraft fixture</title>";
const SIMULATED_POSITION: &str = r#"{"observedAtMs":2500,"latitudeDegrees":50.824,"longitudeDegrees":6.187,"altitudeMeters":410.5,"trackDegrees":90,"groundSpeedMetersPerSecond":31}"#;

/// Builds the app backed by a throwaway static directory containing a known
/// `index.html`. The returned `TempDir` must stay in scope for the duration of
/// the test, as dropping it deletes the directory.
fn app_with_fixture() -> (TempDir, Runtime, axum::Router) {
    let dir = tempfile::tempdir().expect("failed to create temporary directory");
    std::fs::write(dir.path().join("index.html"), INDEX_HTML)
        .expect("failed to write test index.html");
    let runtime = updraft_server::start_runtime();
    let app = updraft_server::router(
        updraft_server::ServerState {
            runtime: runtime.handle(),
        },
        updraft_server::RouterOptions {
            static_dir: Some(dir.path().to_owned()),
            ..Default::default()
        },
    );
    (dir, runtime, app)
}

fn simulation_app_with_fixture() -> (TempDir, Runtime, axum::Router) {
    let dir = tempfile::tempdir().expect("failed to create temporary directory");
    std::fs::write(dir.path().join("index.html"), INDEX_HTML)
        .expect("failed to write test index.html");
    let runtime = updraft_server::start_runtime();
    let app = updraft_server::router(
        updraft_server::ServerState {
            runtime: runtime.handle(),
        },
        updraft_server::RouterOptions {
            static_dir: Some(dir.path().to_owned()),
            simulation: true,
        },
    );
    (dir, runtime, app)
}

fn position_input() -> Input {
    Input::Flight(FlightInput::Gnss(Sourced::simulator(Observation::new(
        Duration::from_millis(1_250),
        GnssUpdate {
            position: LatLon::from_degrees(50.823, 6.186),
            altitude: Some(MslAltitude::new(Length::from_meters(400.5))),
            track: Some(Angle::from_degrees(45.)),
            ground_speed: Some(Speed::from_meters_per_second(30.)),
        },
    ))))
}

fn pressure_altitude_input() -> Input {
    Input::Flight(FlightInput::PressureAltitude(Sourced::external(
        DeviceId::new(7),
        Observation::new(
            Duration::from_millis(1_500),
            PressureAltitude::new(Length::from_meters(975.)),
        ),
    )))
}

struct BlockingQuery {
    entered: mpsc::SyncSender<()>,
    release: mpsc::Receiver<()>,
}

impl updraft_core::Query for BlockingQuery {
    type Output = ();

    fn execute(self, _app: &App) -> Self::Output {
        assert_ok_eq!(self.entered.send(()), ());
        assert_ok_eq!(self.release.recv(), ());
    }
}

async fn body_bytes(response: axum::http::Response<Body>) -> Vec<u8> {
    assert_ok!(response.into_body().collect().await)
        .to_bytes()
        .to_vec()
}

async fn next_data(body: &mut Body) -> axum::body::Bytes {
    let frame = assert_some!(body.frame().await);
    let frame = assert_ok!(frame);
    assert_ok!(frame.into_data())
}

fn simulation_position_request(position: &'static str) -> Request<Body> {
    assert_ok!(
        Request::builder()
            .method("POST")
            .uri("/api/simulation/position")
            .header("content-type", "application/json")
            .body(Body::from(position))
    )
}

#[tokio::test]
async fn health_returns_ok_with_empty_body() {
    let (_dir, _runtime, app) = app_with_fixture();

    let uri = "/api/health";
    let request = Request::builder().uri(uri).body(Body::empty()).unwrap();
    let response = assert_ok!(app.oneshot(request).await);
    assert_eq!(response.status(), StatusCode::OK);
    assert!(body_bytes(response).await.is_empty());
}

#[tokio::test]
async fn unknown_route_serves_spa_index() {
    let (_dir, _runtime, app) = app_with_fixture();

    for uri in ["/", "/map"] {
        let request = Request::builder().uri(uri).body(Body::empty()).unwrap();
        let response = assert_ok!(app.clone().oneshot(request).await);
        assert_eq!(response.status(), StatusCode::OK, "uri = {uri}");
        assert_eq!(
            body_bytes(response).await,
            INDEX_HTML.as_bytes(),
            "uri = {uri}"
        );
    }
}

#[tokio::test]
async fn server_without_static_dir_only_serves_api() {
    let runtime = updraft_server::start_runtime();
    let app = updraft_server::router(
        updraft_server::ServerState {
            runtime: runtime.handle(),
        },
        updraft_server::RouterOptions::default(),
    );
    let health_request = assert_ok!(Request::builder().uri("/api/health").body(Body::empty()));
    let health_response = assert_ok!(app.clone().oneshot(health_request).await);
    assert_eq!(health_response.status(), StatusCode::OK);
    assert!(body_bytes(health_response).await.is_empty());

    let frontend_request = assert_ok!(Request::builder().uri("/").body(Body::empty()));
    let frontend_response = assert_ok!(app.oneshot(frontend_request).await);

    assert_eq!(frontend_response.status(), StatusCode::NOT_FOUND);
    assert!(body_bytes(frontend_response).await.is_empty());
}

#[tokio::test]
async fn unknown_api_route_returns_not_found() {
    let (_dir, _runtime, app) = app_with_fixture();

    let uri = "/api/unknown";
    let request = Request::builder().uri(uri).body(Body::empty()).unwrap();
    let response = assert_ok!(app.oneshot(request).await);
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert!(body_bytes(response).await.is_empty());
}

#[tokio::test]
async fn state_stream_starts_with_snapshot() {
    let (_dir, _runtime, app) = app_with_fixture();

    let request = Request::builder()
        .uri("/api/state")
        .body(Body::empty())
        .unwrap();
    let response = assert_ok!(app.oneshot(request).await);
    assert_eq!(response.status(), StatusCode::OK);

    let mut body = response.into_body();
    assert_eq!(
        next_data(&mut body).await,
        "event: snapshot\ndata: {\"flight\":{\"gnss\":null,\"pressureAltitudeMeters\":null,\"traceStats\":null}}\n\n"
    );
}

#[tokio::test]
async fn state_stream_snapshot_includes_current_state() {
    let (_dir, runtime, app) = app_with_fixture();
    let handle = runtime.handle();
    assert_ok_eq!(handle.submit(position_input()), ());
    assert_ok_eq!(handle.submit(pressure_altitude_input()), ());
    assert_ok_eq!(handle.submit(Input::Flight(FlightInput::ClearTrace)), ());

    let request = Request::builder()
        .uri("/api/state")
        .body(Body::empty())
        .unwrap();
    let response = assert_ok!(app.oneshot(request).await);
    let mut body = response.into_body();

    assert_eq!(
        next_data(&mut body).await,
        "event: snapshot\ndata: {\"flight\":{\"gnss\":{\"position\":{\"latitudeDegrees\":50.823,\"longitudeDegrees\":6.186},\"altitudeMeters\":400.5,\"trackDegrees\":45.0,\"groundSpeedMetersPerSecond\":30.0},\"pressureAltitudeMeters\":975.0,\"traceStats\":null}}\n\n"
    );
}

#[tokio::test]
async fn state_stream_reports_a_stopped_runtime() {
    let (_dir, runtime, app) = app_with_fixture();
    runtime.shutdown();
    let request = Request::builder()
        .uri("/api/state")
        .body(Body::empty())
        .unwrap();

    let response = assert_ok!(app.oneshot(request).await);

    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    assert!(body_bytes(response).await.is_empty());
}

#[tokio::test(flavor = "current_thread")]
async fn waiting_for_state_subscription_keeps_executor_responsive() {
    let (_dir, runtime, app) = app_with_fixture();
    let (entered_tx, entered_rx) = mpsc::sync_channel(1);
    let (release_tx, release_rx) = mpsc::sync_channel(1);
    let handle = runtime.handle();
    let query_thread = thread::spawn(move || {
        assert_ok_eq!(
            handle.query(BlockingQuery {
                entered: entered_tx,
                release: release_rx,
            }),
            ()
        );
    });
    assert_ok_eq!(entered_rx.recv(), ());

    let (state_started_tx, state_started_rx) = tokio::sync::oneshot::channel();
    let (watchdog_started_tx, watchdog_started_rx) = mpsc::sync_channel(1);
    let (health_done_tx, health_done_rx) = mpsc::sync_channel(1);
    // The watchdog releases the blocked core after the deadline so a blocking
    // state handler fails the assertion instead of hanging the test process.
    let watchdog_thread = thread::spawn(move || {
        assert_ok_eq!(watchdog_started_rx.recv(), ());
        let health_responded = health_done_rx.recv_timeout(Duration::from_secs(2)).is_ok();
        assert_ok_eq!(release_tx.send(()), ());
        if !health_responded {
            assert_ok_eq!(
                health_done_rx.recv(),
                (),
                "health request did not complete after releasing the core"
            );
        }
        health_responded
    });

    let state_request = Request::builder()
        .uri("/api/state")
        .body(Body::empty())
        .unwrap();
    let state_app = app.clone();
    let state_task = tokio::spawn(async move {
        assert_ok_eq!(state_started_tx.send(()), ());
        assert_ok_eq!(watchdog_started_tx.send(()), ());
        state_app.oneshot(state_request).await
    });

    // The state task sends this immediately before polling the handler. On a
    // single-threaded executor, a direct blocking subscribe would prevent this
    // test task and the health handler below from being polled.
    assert_ok_eq!(state_started_rx.await, ());

    let health_request = Request::builder()
        .uri("/api/health")
        .body(Body::empty())
        .unwrap();
    let health_response = assert_ok!(app.oneshot(health_request).await);
    assert_eq!(health_response.status(), StatusCode::OK);
    assert_ok_eq!(health_done_tx.send(()), ());

    let health_responded_while_core_blocked = assert_ok!(watchdog_thread.join());
    let state_response = assert_ok!(state_task.await);
    let state_response = assert_ok!(state_response);
    assert_eq!(state_response.status(), StatusCode::OK);
    assert_ok_eq!(query_thread.join(), ());
    assert!(
        health_responded_while_core_blocked,
        "state subscription blocked the async executor"
    );
}

#[tokio::test]
async fn state_stream_sends_change_batches_after_snapshot() {
    let (_dir, runtime, app) = app_with_fixture();
    let handle = runtime.handle();
    let request = Request::builder()
        .uri("/api/state")
        .body(Body::empty())
        .unwrap();
    let response = assert_ok!(app.oneshot(request).await);
    let mut body = response.into_body();
    let _snapshot = next_data(&mut body).await;

    assert_ok_eq!(handle.submit(position_input()), ());

    assert_eq!(
        next_data(&mut body).await,
        "event: changes\ndata: [{\"group\":\"flight\",\"type\":\"gnss\",\"value\":{\"position\":{\"latitudeDegrees\":50.823,\"longitudeDegrees\":6.186},\"altitudeMeters\":400.5,\"trackDegrees\":45.0,\"groundSpeedMetersPerSecond\":30.0}}]\n\n"
    );
}

#[tokio::test]
async fn state_stream_sends_pressure_altitude_meter_changes() {
    let (_dir, runtime, app) = app_with_fixture();
    let handle = runtime.handle();
    let request = Request::builder()
        .uri("/api/state")
        .body(Body::empty())
        .unwrap();
    let response = assert_ok!(app.oneshot(request).await);
    let mut body = response.into_body();
    let _snapshot = next_data(&mut body).await;

    assert_ok_eq!(handle.submit(pressure_altitude_input()), ());

    assert_eq!(
        next_data(&mut body).await,
        "event: changes\ndata: [{\"group\":\"flight\",\"type\":\"pressureAltitudeMeters\",\"value\":975.0}]\n\n"
    );
}

#[tokio::test]
async fn simulated_position_is_published_to_the_state_stream() {
    let (_dir, _runtime, app) = simulation_app_with_fixture();
    let stream_request = assert_ok!(Request::builder().uri("/api/state").body(Body::empty()));
    let stream_response = assert_ok!(app.clone().oneshot(stream_request).await);
    let mut stream = stream_response.into_body();
    let _snapshot = next_data(&mut stream).await;

    let position_request = simulation_position_request(SIMULATED_POSITION);

    let response = assert_ok!(app.oneshot(position_request).await);

    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    assert!(body_bytes(response).await.is_empty());
    assert_eq!(
        next_data(&mut stream).await,
        "event: changes\ndata: [{\"group\":\"flight\",\"type\":\"gnss\",\"value\":{\"position\":{\"latitudeDegrees\":50.824,\"longitudeDegrees\":6.187},\"altitudeMeters\":410.5,\"trackDegrees\":90.0,\"groundSpeedMetersPerSecond\":31.0}}]\n\n"
    );
}

#[tokio::test]
async fn simulated_position_rejects_invalid_domain_values() {
    let (_dir, _runtime, app) = simulation_app_with_fixture();
    let invalid_positions = [
        r#"{"observedAtMs":-1,"latitudeDegrees":50,"longitudeDegrees":6}"#,
        r#"{"observedAtMs":0,"latitudeDegrees":91,"longitudeDegrees":6}"#,
        r#"{"observedAtMs":0,"latitudeDegrees":50,"longitudeDegrees":181}"#,
        r#"{"observedAtMs":0,"latitudeDegrees":50,"longitudeDegrees":6,"trackDegrees":360}"#,
        r#"{"observedAtMs":0,"latitudeDegrees":50,"longitudeDegrees":6,"groundSpeedMetersPerSecond":-1}"#,
    ];

    for position in invalid_positions {
        let request = simulation_position_request(position);

        let response = assert_ok!(app.clone().oneshot(request).await);

        assert_eq!(response.status(), StatusCode::BAD_REQUEST, "{position}");
        assert!(body_bytes(response).await.is_empty(), "{position}");
    }
}

#[tokio::test]
async fn simulated_position_reports_a_stopped_runtime() {
    let (_dir, runtime, app) = simulation_app_with_fixture();
    runtime.shutdown();

    let response = assert_ok!(
        app.oneshot(simulation_position_request(SIMULATED_POSITION))
            .await
    );

    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    assert!(body_bytes(response).await.is_empty());
}

#[tokio::test(flavor = "current_thread")]
async fn waiting_to_submit_a_simulated_position_keeps_executor_responsive() {
    let dir = tempfile::tempdir().expect("failed to create temporary directory");
    std::fs::write(dir.path().join("index.html"), INDEX_HTML)
        .expect("failed to write test index.html");
    let runtime = Runtime::builder().input_queue_capacity(1).start();
    let app = updraft_server::router(
        updraft_server::ServerState {
            runtime: runtime.handle(),
        },
        updraft_server::RouterOptions {
            static_dir: Some(dir.path().to_owned()),
            simulation: true,
        },
    );

    let (entered_tx, entered_rx) = mpsc::sync_channel(1);
    let (release_tx, release_rx) = mpsc::sync_channel(1);
    let handle = runtime.handle();
    let query_thread = thread::spawn(move || {
        assert_ok_eq!(
            handle.query(BlockingQuery {
                entered: entered_tx,
                release: release_rx,
            }),
            ()
        );
    });
    assert_ok_eq!(entered_rx.recv(), ());

    // Fill the single queue slot while the core is blocked by the query.
    assert_ok_eq!(runtime.handle().submit(position_input()), ());

    let (position_started_tx, position_started_rx) = tokio::sync::oneshot::channel();
    let (watchdog_started_tx, watchdog_started_rx) = mpsc::sync_channel(1);
    let (health_done_tx, health_done_rx) = mpsc::sync_channel(1);
    let watchdog_thread = thread::spawn(move || {
        assert_ok_eq!(watchdog_started_rx.recv(), ());
        let health_responded = health_done_rx.recv_timeout(Duration::from_secs(2)).is_ok();
        assert_ok_eq!(release_tx.send(()), ());
        if !health_responded {
            assert_ok_eq!(
                health_done_rx.recv(),
                (),
                "health request did not complete after releasing the core"
            );
        }
        health_responded
    });

    let position_app = app.clone();
    let position_task = tokio::spawn(async move {
        assert_ok_eq!(position_started_tx.send(()), ());
        assert_ok_eq!(watchdog_started_tx.send(()), ());
        position_app
            .oneshot(simulation_position_request(SIMULATED_POSITION))
            .await
    });

    assert_ok_eq!(position_started_rx.await, ());

    let health_request = assert_ok!(Request::builder().uri("/api/health").body(Body::empty()));
    let health_response = assert_ok!(app.oneshot(health_request).await);
    assert_eq!(health_response.status(), StatusCode::OK);
    assert_ok_eq!(health_done_tx.send(()), ());

    let health_responded_while_queue_was_full = assert_ok!(watchdog_thread.join());
    let position_response = assert_ok!(position_task.await);
    let position_response = assert_ok!(position_response);
    assert_eq!(position_response.status(), StatusCode::NO_CONTENT);
    assert_ok_eq!(query_thread.join(), ());
    assert!(
        health_responded_while_queue_was_full,
        "simulation input blocked the async executor"
    );
}

#[tokio::test]
async fn standard_server_does_not_expose_simulation_routes() {
    let (_dir, _runtime, app) = app_with_fixture();
    let request = simulation_position_request(
        r#"{"observedAtMs":0,"latitudeDegrees":50,"longitudeDegrees":6}"#,
    );

    let response = assert_ok!(app.oneshot(request).await);

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert!(body_bytes(response).await.is_empty());
}

#[tokio::test]
async fn state_stream_includes_results_from_configured_workers() {
    let (_dir, runtime, app) = app_with_fixture();
    let handle = runtime.handle();
    let request = Request::builder()
        .uri("/api/state")
        .body(Body::empty())
        .unwrap();
    let response = assert_ok!(app.oneshot(request).await);
    let mut body = response.into_body();
    let _snapshot = next_data(&mut body).await;

    assert_ok_eq!(handle.submit(position_input()), ());
    let _position = next_data(&mut body).await;

    let frame = assert_ok!(
        tokio::time::timeout(Duration::from_secs(1), body.frame()).await,
        "timed out waiting for trace statistics"
    );
    let frame = assert_some!(frame);
    let frame = assert_ok!(frame);
    assert_eq!(
        assert_ok!(frame.into_data()),
        "event: changes\ndata: [{\"group\":\"flight\",\"type\":\"traceStats\",\"value\":{\"fixCount\":1,\"distanceMeters\":0.0,\"maxAltitudeMeters\":400.5}}]\n\n"
    );
}

#[tokio::test(start_paused = true)]
async fn state_stream_sends_heartbeats_at_fixed_intervals() {
    let (_dir, _runtime, app) = app_with_fixture();
    let request = Request::builder()
        .uri("/api/state")
        .body(Body::empty())
        .unwrap();
    let response = assert_ok!(app.oneshot(request).await);
    let mut body = response.into_body();
    let _snapshot = next_data(&mut body).await;

    assert_none!(
        body.frame().now_or_never(),
        "heartbeat arrived before its first interval"
    );

    tokio::time::advance(Duration::from_secs(14)).await;
    assert_none!(
        body.frame().now_or_never(),
        "heartbeat arrived before 15 seconds"
    );

    tokio::time::advance(Duration::from_secs(1)).await;

    let frame = assert_some!(
        body.frame().now_or_never(),
        "heartbeat missing after 15 seconds"
    );
    let frame = assert_some!(frame);
    let frame = assert_ok!(frame);
    assert_eq!(
        assert_ok!(frame.into_data()),
        "event: heartbeat\ndata: {}\n\n"
    );
}
