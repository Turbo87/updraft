use std::time::Duration;

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use serde_json::json;
use tempfile::TempDir;
use tokio::time::timeout;
use tower::ServiceExt;
use updraft_core::App;

const INDEX_HTML: &str = "<!doctype html><title>updraft fixture</title>";

/// Builds the app backed by a throwaway static directory containing a known
/// `index.html`. The returned `TempDir` must stay in scope for the duration of
/// the test, as dropping it deletes the directory.
fn app_with_fixture() -> (TempDir, axum::Router) {
    let dir = static_fixture();
    let app = updraft_server::router(dir.path(), updraft_runtime::spawn(App::default()));
    (dir, app)
}

/// Like [`app_with_fixture`], with the simulation input routes enabled.
fn simulation_app_with_fixture() -> (TempDir, axum::Router) {
    let dir = static_fixture();
    let app = updraft_server::simulation_router(dir.path(), updraft_runtime::spawn(App::default()));
    (dir, app)
}

fn static_fixture() -> TempDir {
    let dir = tempfile::tempdir().expect("create tempdir");
    std::fs::write(dir.path().join("index.html"), INDEX_HTML).expect("write index.html");
    dir
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

fn position_request(body: serde_json::Value) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri("/api/simulation/position")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_string()))
        .unwrap()
}

/// Incrementally parses SSE events off a streaming response body.
struct EventStream {
    body: Body,
    buffer: String,
}

impl EventStream {
    async fn open(app: &axum::Router) -> Self {
        let request = Request::builder()
            .uri("/api/state")
            .body(Body::empty())
            .unwrap();
        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers()[header::CONTENT_TYPE],
            "text/event-stream"
        );
        Self {
            body: response.into_body(),
            buffer: String::new(),
        }
    }

    /// Returns the next `(event, data)` pair, skipping keep-alive comments.
    async fn next_event(&mut self) -> (String, serde_json::Value) {
        loop {
            if let Some(end) = self.buffer.find("\n\n") {
                let block: String = self.buffer.drain(..end + 2).collect();
                let event = block.lines().find_map(|line| line.strip_prefix("event: "));
                let data = block.lines().find_map(|line| line.strip_prefix("data: "));
                if let (Some(event), Some(data)) = (event, data) {
                    return (event.to_owned(), serde_json::from_str(data).unwrap());
                }
                continue;
            }

            let frame = timeout(Duration::from_secs(5), self.body.frame())
                .await
                .expect("state stream stalled")
                .expect("state stream ended")
                .expect("state stream errored");
            if let Some(data) = frame.data_ref() {
                self.buffer.push_str(std::str::from_utf8(data).unwrap());
            }
        }
    }
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
    let (_dir, app) = app_with_fixture();

    let mut stream = EventStream::open(&app).await;
    let (event, data) = stream.next_event().await;

    assert_eq!(event, "snapshot");
    assert_eq!(data, json!({ "position": null, "track_distance": 0.0 }));
}

#[tokio::test]
async fn state_stream_delivers_position_changes() {
    let (_dir, app) = simulation_app_with_fixture();

    let mut stream = EventStream::open(&app).await;
    stream.next_event().await;

    let response = app
        .clone()
        .oneshot(position_request(
            json!({ "latitude": 50.823, "longitude": 6.186, "track": 45.0 }),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NO_CONTENT);

    let (event, data) = stream.next_event().await;
    assert_eq!(event, "changes");
    assert_eq!(
        data,
        json!([{
            "flight": {
                "position_changed": {
                    "location": { "latitude": 50.823, "longitude": 6.186 },
                    "track": 45.0,
                }
            }
        }])
    );
}

#[tokio::test]
async fn late_subscriber_snapshot_contains_earlier_positions() {
    let (_dir, app) = simulation_app_with_fixture();

    let response = app
        .clone()
        .oneshot(position_request(json!({
            "latitude": 50.823,
            "longitude": 6.186,
            "observed_at_ms": 1000,
        })))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NO_CONTENT);

    let mut stream = EventStream::open(&app).await;
    let (event, data) = stream.next_event().await;

    assert_eq!(event, "snapshot");
    assert_eq!(
        data,
        json!({
            "position": {
                "current": {
                    "location": { "latitude": 50.823, "longitude": 6.186 },
                    "track": null,
                }
            },
            "track_distance": 0.0,
        })
    );
}

#[tokio::test]
async fn simulation_position_rejects_invalid_coordinates() {
    let (_dir, app) = simulation_app_with_fixture();

    for body in [
        json!({ "latitude": 90.1, "longitude": 6.186 }),
        json!({ "latitude": 50.823, "longitude": -180.1 }),
    ] {
        let response = app.clone().oneshot(position_request(body)).await.unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }
}

#[tokio::test]
async fn production_router_does_not_expose_simulation_routes() {
    let (_dir, app) = app_with_fixture();

    let response = app
        .oneshot(position_request(
            json!({ "latitude": 50.823, "longitude": 6.186 }),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
