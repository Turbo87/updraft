use axum::Router;
use axum::extract::{Json, State};
use axum::http::StatusCode;
use axum::response::sse::{Event, Sse};
use axum::routing::{get, post};
use futures_util::stream::{self, Stream};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio::time::{Instant, Interval, MissedTickBehavior};
use tower_http::services::{ServeDir, ServeFile};
use tower_http::trace::TraceLayer;
use updraft_core::flight::{FlightComputeKind, FlightInput, GnssUpdate, Observation, Sourced};
use updraft_runtime::{ChangeFilter, Handle, PureWorker, Runtime};

pub mod wire;

const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(15);

/// Required dependencies shared by HTTP request handlers.
#[derive(Clone)]
pub struct ServerState {
    pub runtime: Handle,
}

/// Optional HTTP application configuration.
///
/// By default, only the standard `/api` routes are exposed.
#[derive(Default)]
pub struct RouterOptions {
    pub static_dir: Option<PathBuf>,
    pub simulation: bool,
}

/// State carried from one [`stream::unfold`] iteration to the next.
struct StateEvents {
    snapshot: Option<Event>,
    changes: mpsc::Receiver<Vec<updraft_core::Change>>,
    heartbeat: Interval,
    // Retains the cancellation guard for the lifetime of the SSE stream.
    _bridge: ChangeBridge,
}

impl StateEvents {
    /// Returns one SSE event and the state for the next unfold iteration.
    async fn next(mut self) -> Option<(Result<Event, axum::Error>, Self)> {
        if let Some(snapshot) = self.snapshot.take() {
            return Some((Ok(snapshot), self));
        }

        tokio::select! {
            changes = self.changes.recv() => {
                let changes: Vec<wire::Change> = changes?
                    .into_iter()
                    .map(Into::into)
                    .collect();

                let event = Event::default().event("changes").json_data(changes);
                Some((event, self))
            }

            _ = self.heartbeat.tick() => {
                let event = Event::default().event("heartbeat").data("{}");
                Some((Ok(event), self))
            }
        }
    }
}

struct ChangeBridge {
    cancelled: Arc<AtomicBool>,
    task: JoinHandle<()>,
}

impl Drop for ChangeBridge {
    fn drop(&mut self) {
        self.cancelled.store(true, Ordering::Release);
        self.task.abort();
    }
}

/// Starts the shared runtime with the workers required by the current core.
#[must_use = "dropping the returned runtime immediately shuts it down"]
pub fn start_runtime() -> Runtime {
    Runtime::builder()
        .worker(
            updraft_core::ComputeKind::Flight(FlightComputeKind::TraceStats),
            PureWorker,
        )
        .start()
}

/// Builds the HTTP application.
///
/// Backend routes live under `/api`, and unknown `/api` paths return `404`.
/// When `static_dir` is configured, everything outside `/api` is served from
/// that directory and falls back to `index.html` for client-side routing.
/// Without it, routes outside `/api` also return `404`.
pub fn router(state: ServerState, options: RouterOptions) -> Router {
    let mut api = Router::new()
        .route("/health", get(health))
        .route("/state", get(state_stream));
    if options.simulation {
        api = api.route("/simulation/position", post(simulation_position));
    }
    let api = api.fallback(not_found).with_state(state);

    let mut app = Router::new().nest("/api", api);
    if let Some(static_dir) = options.static_dir {
        let index_html = static_dir.join("index.html");
        let static_service = ServeDir::new(static_dir).fallback(ServeFile::new(index_html));
        app = app.fallback_service(static_service);
    }

    app.layer(TraceLayer::new_for_http())
}

async fn health() -> StatusCode {
    StatusCode::OK
}

async fn simulation_position(
    State(state): State<ServerState>,
    Json(position): Json<wire::PositionFix>,
) -> Result<StatusCode, StatusCode> {
    let observation =
        Observation::<GnssUpdate>::try_from(position).map_err(|_| StatusCode::BAD_REQUEST)?;
    let input = updraft_core::Input::Flight(FlightInput::Gnss(Sourced::simulator(observation)));
    let runtime = state.runtime;

    tokio::task::spawn_blocking(move || runtime.submit(input))
        .await
        .inspect_err(|error| tracing::error!("simulation input task failed: {error}"))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .inspect_err(|error| tracing::error!("simulation input failed: {error}"))
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;

    Ok(StatusCode::NO_CONTENT)
}

/// Opens a snapshot-first SSE stream backed by one runtime subscription.
///
/// Subscription registration uses the blocking pool because the runtime API
/// deliberately applies synchronous backpressure. Change batches then cross a
/// capacity-one bridge into the async response stream. The small capacity lets
/// the runtime still detect and drop a client that stops consuming updates.
async fn state_stream(
    State(state): State<ServerState>,
) -> Result<Sse<impl Stream<Item = Result<Event, axum::Error>>>, StatusCode> {
    let runtime = state.runtime;
    let subscription = tokio::task::spawn_blocking(move || runtime.subscribe(ChangeFilter::all()))
        .await
        .inspect_err(|error| tracing::error!("state subscription task failed: {error}"))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .inspect_err(|error| tracing::error!("state subscription task failed: {error}"))
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;

    let snapshot = wire::Snapshot::from(subscription.snapshot);

    let event = Event::default()
        .event("snapshot")
        .json_data(snapshot)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let (changes, bridge) = bridge_changes(subscription.changes);

    let start = Instant::now() + HEARTBEAT_INTERVAL;
    let mut heartbeat = tokio::time::interval_at(start, HEARTBEAT_INTERVAL);
    heartbeat.set_missed_tick_behavior(MissedTickBehavior::Skip);

    let state_events = StateEvents {
        snapshot: Some(event),
        changes,
        heartbeat,
        _bridge: bridge,
    };

    // `unfold` owns this state until the response stream is dropped, which also
    // drops `_bridge` and cancels its blocking receiver.
    let events = stream::unfold(state_events, StateEvents::next);

    Ok(Sse::new(events))
}

/// Bridges a blocking runtime receiver into a bounded Tokio channel.
///
/// The blocking task owns the runtime receiver. Dropping the corresponding
/// [`ChangeBridge`] cancels the task when the HTTP client disconnects. The
/// timeout bounds how long an idle receiver takes to observe that cancellation.
fn bridge_changes(
    changes: Receiver<Vec<updraft_core::Change>>,
) -> (mpsc::Receiver<Vec<updraft_core::Change>>, ChangeBridge) {
    let (tx, rx) = mpsc::channel(1);
    let cancelled = Arc::new(AtomicBool::new(false));
    let bridge_cancelled = Arc::clone(&cancelled);
    let task = tokio::task::spawn_blocking(move || {
        while !bridge_cancelled.load(Ordering::Acquire) {
            match changes.recv_timeout(Duration::from_millis(100)) {
                Ok(changes) => {
                    if tx.blocking_send(changes).is_err() {
                        return;
                    }
                }
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => return,
            }
        }
    });

    (rx, ChangeBridge { cancelled, task })
}

async fn not_found() -> StatusCode {
    StatusCode::NOT_FOUND
}
