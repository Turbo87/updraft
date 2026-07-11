use std::path::Path;

use axum::Router;
use axum::http::StatusCode;
use axum::routing::get;
use tower_http::services::{ServeDir, ServeFile};
use tower_http::trace::TraceLayer;

/// Builds the HTTP application.
///
/// Backend routes live under `/api`, and unknown `/api` paths return `404`.
/// Everything else is served from `static_dir`, falling back to `index.html`
/// so the client-side-routed frontend can handle deep links.
pub fn router(static_dir: impl AsRef<Path>) -> Router {
    let static_dir = static_dir.as_ref();
    let index_html = static_dir.join("index.html");

    let api = Router::new()
        .route("/health", get(health))
        .fallback(not_found);

    let static_service = ServeDir::new(static_dir).fallback(ServeFile::new(index_html));

    Router::new()
        .nest("/api", api)
        .fallback_service(static_service)
        .layer(TraceLayer::new_for_http())
}

async fn health() -> StatusCode {
    StatusCode::OK
}

async fn not_found() -> StatusCode {
    StatusCode::NOT_FOUND
}
