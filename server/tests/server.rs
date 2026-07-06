use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tempfile::TempDir;
use tower::ServiceExt;

const INDEX_HTML: &str = "<!doctype html><title>updraft fixture</title>";

/// Builds the app backed by a throwaway static directory containing a known
/// `index.html`. The returned `TempDir` must stay in scope for the duration of
/// the test, as dropping it deletes the directory.
fn app_with_fixture() -> (TempDir, axum::Router) {
    let dir = tempfile::tempdir().expect("create tempdir");
    std::fs::write(dir.path().join("index.html"), INDEX_HTML).expect("write index.html");
    let app = updraft_server::router(dir.path());
    (dir, app)
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
