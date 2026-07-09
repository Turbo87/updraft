use std::path::Path;
use std::time::Instant;

use axum::Router;
use axum::extract::{Json, State};
use axum::http::StatusCode;
use axum::response::sse::{Event, Sse};
use axum::routing::{get, post};
use futures_util::stream::{self, Stream};
use serde::Deserialize;
use tower_http::services::{ServeDir, ServeFile};
use updraft_core::{
    CoreRuntimeHandle, FlightInput, Input, MonotonicTime, ObservationSource, PositionObservation,
    StateMessage,
};
use updraft_geo::LatLon;
use updraft_units::Angle;

#[derive(Clone)]
struct ServerState {
    runtime: CoreRuntimeHandle,
    started_at: Instant,
}

/// Builds the HTTP application.
///
/// Backend routes live under `/api`, and unknown `/api` paths return `404`.
/// Everything else is served from `static_dir`, falling back to `index.html`
/// so the client-side-routed frontend can handle deep links.
pub fn router(static_dir: impl AsRef<Path>, runtime: CoreRuntimeHandle) -> Router {
    build_router(static_dir, runtime, false)
}

pub fn simulation_router(static_dir: impl AsRef<Path>, runtime: CoreRuntimeHandle) -> Router {
    build_router(static_dir, runtime, true)
}

fn build_router(
    static_dir: impl AsRef<Path>,
    runtime: CoreRuntimeHandle,
    simulation: bool,
) -> Router {
    let static_dir = static_dir.as_ref();
    let index_html = static_dir.join("index.html");

    let mut api = Router::new()
        .route("/health", get(health))
        .route("/state", get(state_stream))
        .fallback(not_found);
    if simulation {
        api = api.route("/simulation/position", post(simulation_position));
    }
    let api = api.with_state(ServerState {
        runtime,
        started_at: Instant::now(),
    });

    let static_service = ServeDir::new(static_dir).fallback(ServeFile::new(index_html));

    Router::new()
        .nest("/api", api)
        .fallback_service(static_service)
}

async fn health() -> StatusCode {
    StatusCode::OK
}

async fn not_found() -> StatusCode {
    StatusCode::NOT_FOUND
}

async fn state_stream(
    State(state): State<ServerState>,
) -> Result<Sse<impl Stream<Item = Result<Event, axum::Error>>>, StatusCode> {
    let stream = state
        .runtime
        .subscribe()
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    let events = stream::unfold(stream, |mut stream| async move {
        let message = stream.recv().await?;
        let event = match message {
            StateMessage::Snapshot(snapshot) => {
                Event::default().event("snapshot").json_data(snapshot)
            }
            StateMessage::Changes(changes) => Event::default().event("changes").json_data(changes),
        };
        Some((event, stream))
    });

    Ok(Sse::new(events))
}

#[derive(Deserialize)]
struct SimulationPosition {
    latitude: f64,
    longitude: f64,
    track: Option<f64>,
}

async fn simulation_position(
    State(state): State<ServerState>,
    Json(position): Json<SimulationPosition>,
) -> Result<StatusCode, StatusCode> {
    let position = PositionObservation::new(
        ObservationSource::Simulation,
        MonotonicTime::from_duration(state.started_at.elapsed()),
        LatLon::from_degrees(position.latitude, position.longitude),
        position.track.map(Angle::from_degrees),
    )
    .map_err(|_| StatusCode::BAD_REQUEST)?;
    state
        .runtime
        .submit(Input::Flight(FlightInput::PositionObserved(position)))
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    Ok(StatusCode::NO_CONTENT)
}
