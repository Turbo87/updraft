use std::convert::Infallible;
use std::path::Path;
use std::time::{Duration, Instant};

use axum::Router;
use axum::extract::{Json, State};
use axum::http::StatusCode;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::routing::{get, post};
use futures_util::Stream;
use futures_util::stream;
use serde::Deserialize;
use tower_http::services::{ServeDir, ServeFile};
use updraft_core::{FlightInput, Input, MonotonicTime, ObservationSource, PositionObservation};
use updraft_geo::LatLon;
use updraft_runtime::{RuntimeHandle, StateMessage};
use updraft_units::Angle;

#[derive(Clone)]
struct ServerState {
    runtime: RuntimeHandle,
    /// The process-wide monotonic epoch that adapters stamp inputs against.
    epoch: Instant,
}

/// Builds the HTTP application.
///
/// Backend routes live under `/api`, and unknown `/api` paths return `404`.
/// Everything else is served from `static_dir`, falling back to `index.html`
/// so the client-side-routed frontend can handle deep links.
pub fn router(static_dir: impl AsRef<Path>, runtime: RuntimeHandle) -> Router {
    build_router(static_dir, runtime, false)
}

/// Like [`router`], plus the simulation input routes used by the e2e suite
/// and, later, the user-facing simulator (see `docs/design/simulator.md`).
pub fn simulation_router(static_dir: impl AsRef<Path>, runtime: RuntimeHandle) -> Router {
    build_router(static_dir, runtime, true)
}

fn build_router(static_dir: impl AsRef<Path>, runtime: RuntimeHandle, simulation: bool) -> Router {
    let static_dir = static_dir.as_ref();
    let index_html = static_dir.join("index.html");

    let mut api = Router::new()
        .route("/health", get(health))
        .route("/state", get(state_stream));
    if simulation {
        api = api.route("/simulation/position", post(simulation_position));
    }
    let api = api.fallback(not_found).with_state(ServerState {
        runtime,
        epoch: Instant::now(),
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

/// The state stream: one SSE connection carrying a `snapshot` event
/// followed by `changes` events (see `docs/design/server.md`).
///
/// When the runtime drops this subscriber for falling behind, the stream
/// ends; `EventSource` then reconnects on its own and starts over with a
/// fresh snapshot.
async fn state_stream(
    State(state): State<ServerState>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, StatusCode> {
    let stream = state
        .runtime
        .subscribe()
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;

    let events = stream::unfold(stream, |mut stream| async move {
        let message = stream.recv().await?;
        let event = match &message {
            StateMessage::Snapshot(snapshot) => {
                Event::default().event("snapshot").json_data(snapshot)
            }
            StateMessage::Changes(changes) => Event::default().event("changes").json_data(changes),
        };
        let event = event.expect("state messages serialize to JSON");
        Some((Ok(event), stream))
    });

    Ok(Sse::new(events).keep_alive(KeepAlive::default()))
}

#[derive(Deserialize)]
struct SimulationPosition {
    latitude: f64,
    longitude: f64,
    track: Option<f64>,
    /// Milliseconds on the server's monotonic timeline. Tests inject this
    /// so simulated time is controlled, not wall time; live use omits it.
    observed_at_ms: Option<u64>,
}

async fn simulation_position(
    State(state): State<ServerState>,
    Json(body): Json<SimulationPosition>,
) -> Result<StatusCode, StatusCode> {
    let observed_at = MonotonicTime::from_duration(match body.observed_at_ms {
        Some(millis) => Duration::from_millis(millis),
        None => state.epoch.elapsed(),
    });

    let observation = PositionObservation::new(
        ObservationSource::Simulation,
        observed_at,
        LatLon::from_degrees(body.latitude, body.longitude),
        body.track.map(Angle::from_degrees),
    )
    .map_err(|_| StatusCode::BAD_REQUEST)?;

    state
        .runtime
        .submit(Input::Flight(FlightInput::PositionObserved(observation)))
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;

    Ok(StatusCode::NO_CONTENT)
}
