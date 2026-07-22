use crate::clock::Clock;
use crate::metrics::Metrics;
use crate::worker::{CancellationToken, Worker, WorkerResult};
use std::collections::{HashMap, VecDeque};
use std::panic::AssertUnwindSafe;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender, SyncSender, TrySendError};
use std::sync::mpsc::{channel, sync_channel};
use std::thread;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};
use updraft_core::{
    App, AppConfig, Change, ChangeGroup, ComputeCancellation, ComputeFailure, ComputeJob,
    ComputeKind, Effect, Input, Query, Snapshot,
};

/// Everything the core loop accepts from the outside.
///
/// Inputs, queries, and subscriptions share the one bounded FIFO, so a
/// query observes all inputs submitted before it and a subscription's
/// snapshot capture cannot race with change delivery.
enum LoopMsg {
    Input(Input),
    Query(Box<dyn ErasedQuery>),
    Subscribe(ChangeFilter, SyncSender<Subscription>),
    Shutdown,
}

trait ErasedQuery: Send {
    fn execute(self: Box<Self>, app: &App);
}

struct QueryRequest<Q: Query> {
    query: Q,
    reply: SyncSender<Q::Output>,
}

impl<Q> ErasedQuery for QueryRequest<Q>
where
    Q: Query + Send + 'static,
    Q::Output: Send + 'static,
{
    fn execute(self: Box<Self>, app: &App) {
        let QueryRequest { query, reply } = *self;
        let _ = reply.send(app.query(query));
    }
}

struct QueuedMsg {
    enqueued_at: Instant,
    msg: LoopMsg,
}

#[derive(Clone)]
struct QueueSender {
    tx: SyncSender<QueuedMsg>,
    metrics: Arc<Metrics>,
}

struct WorkerRequest {
    job: ComputeJob,
    cancellation: CancellationToken,
}

struct ActiveJob {
    revision: updraft_core::ComputeRevision,
    cancellation: CancellationToken,
}

impl QueueSender {
    fn send(&self, msg: LoopMsg) -> Result<(), ()> {
        self.metrics.record_enqueued();
        let queued = QueuedMsg {
            enqueued_at: Instant::now(),
            msg,
        };
        self.tx.send(queued).map_err(|_| {
            self.metrics.record_send_failure();
        })
    }
}

/// Configures and starts a [`Runtime`].
pub struct RuntimeBuilder {
    app_config: AppConfig,
    input_queue_capacity: usize,
    subscriber_buffer_capacity: usize,
    workers: Vec<(ComputeKind, Box<dyn Worker>)>,
}

impl RuntimeBuilder {
    /// Uses the given application configuration instead of [`AppConfig::default()`].
    #[must_use]
    pub fn with_app_config(mut self, app_config: AppConfig) -> Self {
        self.app_config = app_config;
        self
    }

    /// Registers the worker executing one compute-job kind.
    ///
    /// A job for a kind without a registered worker fails immediately
    /// with an [`Input::ComputeFailed`]. Registering two workers for the
    /// same kind is a configuration error and panics.
    #[must_use]
    pub fn worker(mut self, kind: ComputeKind, worker: impl Worker) -> Self {
        assert!(
            !self
                .workers
                .iter()
                .any(|(registered, _)| *registered == kind),
            "a worker is already registered for {kind:?}"
        );
        self.workers.push((kind, Box::new(worker)));
        self
    }

    /// Capacity of the bounded input FIFO (default 256). A full queue
    /// blocks the producer and never drops an input.
    #[must_use]
    pub fn input_queue_capacity(mut self, capacity: usize) -> Self {
        self.input_queue_capacity = capacity;
        self
    }

    /// How many change batches a subscriber may buffer before it is
    /// dropped (default 64).
    #[must_use]
    pub fn subscriber_buffer_capacity(mut self, capacity: usize) -> Self {
        self.subscriber_buffer_capacity = capacity;
        self
    }

    /// Starts the core loop and worker threads.
    #[must_use = "dropping the returned Runtime immediately shuts it down"]
    pub fn start(self) -> Runtime {
        let clock = Clock::new();
        let app = App::with_config(self.app_config);
        let metrics = Arc::new(Metrics::default());
        let (tx, rx) = sync_channel(self.input_queue_capacity);
        let tx = QueueSender {
            tx,
            metrics: Arc::clone(&metrics),
        };

        let mut worker_channels = HashMap::new();
        let mut worker_threads = Vec::new();
        for (kind, worker) in self.workers {
            // The job channel is unbounded, but the core's job slot only
            // dispatches a kind's next job once the previous result has been
            // handled, so at most one job per kind is ever in flight and the
            // channel never actually grows.
            let (job_tx, job_rx) = channel();
            worker_channels.insert(kind, job_tx);
            let results = tx.clone();
            let metrics = Arc::clone(&metrics);
            let thread = thread::Builder::new()
                .name(format!("updraft-worker-{kind:?}"))
                .spawn(move || worker_loop(kind, worker, &job_rx, &results, &metrics))
                .expect("failed to spawn worker thread");
            worker_threads.push(thread);
        }

        let core_loop = CoreLoop {
            app,
            rx,
            clock: clock.clone(),
            metrics: Arc::clone(&metrics),
            subscribers: Vec::new(),
            subscriber_buffer_capacity: self.subscriber_buffer_capacity,
            workers: worker_channels,
            active_jobs: HashMap::new(),
            next_deadline: None,
        };
        let core_thread = thread::Builder::new()
            .name("updraft-core".into())
            .spawn(move || core_loop.run())
            .expect("failed to spawn core loop thread");

        Runtime {
            handle: Handle { tx, clock, metrics },
            core_thread: Some(core_thread),
            worker_threads,
        }
    }
}

/// The shared runtime: owns one [`App`], the input queue, a monotonic
/// clock, compute workers, and state-stream subscribers.
///
/// Hosts add transport or platform bindings on top of a [`Handle`] and
/// are responsible for detecting when the runtime stops
/// ([`RuntimeStopped`]) and reporting the failure.
pub struct Runtime {
    handle: Handle,
    core_thread: Option<JoinHandle<()>>,
    worker_threads: Vec<JoinHandle<()>>,
}

impl Runtime {
    /// Configures a runtime that constructs a fresh [`App`] when started.
    ///
    /// The application configuration defaults to [`AppConfig::default()`].
    pub fn builder() -> RuntimeBuilder {
        RuntimeBuilder {
            app_config: AppConfig::default(),
            input_queue_capacity: 256,
            subscriber_buffer_capacity: 64,
            workers: Vec::new(),
        }
    }

    pub fn handle(&self) -> Handle {
        self.handle.clone()
    }

    /// Stops the core loop and joins all runtime threads.
    ///
    /// Queued messages ahead of the shutdown message are still handled.
    /// Inputs submitted concurrently may be queued behind it and not handled.
    /// Active worker jobs receive cooperative cancellation before joining.
    /// Dropping the `Runtime` does the same thing. Calling this explicitly
    /// lets the caller block on teardown at a chosen point.
    pub fn shutdown(mut self) {
        self.stop_and_join();
    }

    /// Signals the core loop to stop and joins every runtime thread.
    /// Idempotent: the second call (from `Drop` after `shutdown`) finds no
    /// threads left to join and a disconnected input channel.
    fn stop_and_join(&mut self) {
        let _ = self.handle.tx.send(LoopMsg::Shutdown);
        if let Some(core_thread) = self.core_thread.take() {
            // A panicked core loop stops the runtime just like a clean
            // shutdown, so surface it here instead of letting the join
            // swallow the only trace of it.
            if let Err(panic) = core_thread.join() {
                tracing::error!("core loop thread panicked: {}", panic_message(&*panic));
            }
        }
        // The core loop owned the job senders, so the workers' queues are
        // now disconnected and the threads wind down. A worker thread that
        // panicked past its own unwind boundary surfaces here.
        for thread in self.worker_threads.drain(..) {
            if let Err(panic) = thread.join() {
                tracing::error!("worker thread panicked: {}", panic_message(&*panic));
            }
        }
    }
}

impl Drop for Runtime {
    fn drop(&mut self) {
        self.stop_and_join();
    }
}

/// A cloneable handle for submitting inputs, querying state, and
/// subscribing to the state stream.
#[derive(Clone)]
pub struct Handle {
    tx: QueueSender,
    clock: Clock,
    metrics: Arc<Metrics>,
}

impl Handle {
    /// The runtime clock used by adapters to timestamp observations.
    pub fn clock(&self) -> &Clock {
        &self.clock
    }

    /// Current time on the runtime clock.
    pub fn clock_time(&self) -> Duration {
        self.clock.clock_time()
    }

    pub fn metrics(&self) -> &Metrics {
        &self.metrics
    }

    /// Submits one input to the core.
    ///
    /// Blocks while the bounded queue is full. Backpressure does not drop
    /// inputs. An `Ok` result means the input was queued, not that it was
    /// handled.
    pub fn submit(&self, input: Input) -> Result<(), RuntimeStopped> {
        self.tx
            .send(LoopMsg::Input(input))
            .map_err(|_| RuntimeStopped)
    }

    /// Answers a typed read-only query against current state.
    pub fn query<Q>(&self, query: Q) -> Result<Q::Output, RuntimeStopped>
    where
        Q: Query + Send + 'static,
        Q::Output: Send + 'static,
    {
        let (tx, rx) = sync_channel(1);
        self.tx
            .send(LoopMsg::Query(Box::new(QueryRequest { query, reply: tx })))
            .map_err(|_| RuntimeStopped)?;
        rx.recv().map_err(|_| RuntimeStopped)
    }

    /// Opens a state-stream subscription: a snapshot first, then
    /// FIFO-ordered change batches.
    ///
    /// Registration and snapshot capture happen in one core-loop turn, so
    /// no change can fall between them.
    pub fn subscribe(&self, filter: ChangeFilter) -> Result<Subscription, RuntimeStopped> {
        let (tx, rx) = sync_channel(1);
        self.tx
            .send(LoopMsg::Subscribe(filter, tx))
            .map_err(|_| RuntimeStopped)?;
        rx.recv().map_err(|_| RuntimeStopped)
    }
}

/// The runtime's core loop has stopped and no longer accepts requests.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
#[error("the runtime has stopped")]
pub struct RuntimeStopped;

/// Selects which change groups a state-stream subscriber receives.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChangeFilter {
    groups: Option<Vec<ChangeGroup>>,
}

impl ChangeFilter {
    /// Receives every current and future change group.
    pub fn all() -> Self {
        Self { groups: None }
    }

    /// Receives only the listed change groups.
    pub fn only(groups: impl IntoIterator<Item = ChangeGroup>) -> Self {
        Self {
            groups: Some(groups.into_iter().collect()),
        }
    }

    fn includes(&self, group: ChangeGroup) -> bool {
        self.groups
            .as_ref()
            .is_none_or(|groups| groups.contains(&group))
    }
}

/// A state-stream subscription.
///
/// The change buffer is bounded: a subscriber that falls behind is
/// dropped by the runtime and its receiver disconnects. There is no
/// replay buffer: reconnecting means resubscribing for a fresh snapshot.
#[derive(Debug)]
pub struct Subscription {
    /// The shared state at the moment the subscription was registered.
    pub snapshot: Snapshot,
    /// Change batches, in input order.
    pub changes: Receiver<Vec<Change>>,
}

/// The single thread that owns the `App` and feeds it inputs.
struct CoreLoop {
    app: App,
    rx: Receiver<QueuedMsg>,
    clock: Clock,
    metrics: Arc<Metrics>,
    subscribers: Vec<SubscriberSender>,
    subscriber_buffer_capacity: usize,
    workers: HashMap<ComputeKind, Sender<WorkerRequest>>,
    active_jobs: HashMap<ComputeKind, ActiveJob>,
    next_deadline: Option<Duration>,
}

impl CoreLoop {
    fn run(mut self) {
        loop {
            let queued = if let Some(deadline) = self.next_deadline {
                let clock_time = self.clock.clock_time();
                if clock_time >= deadline {
                    self.process(Input::Clock { clock_time }, clock_time);
                    continue;
                }
                match self.rx.recv_timeout(deadline.saturating_sub(clock_time)) {
                    Ok(msg) => msg,
                    Err(RecvTimeoutError::Timeout) => {
                        let clock_time = self.clock.clock_time();
                        self.process(Input::Clock { clock_time }, clock_time);
                        continue;
                    }
                    Err(RecvTimeoutError::Disconnected) => return,
                }
            } else {
                match self.rx.recv() {
                    Ok(msg) => msg,
                    Err(_) => return,
                }
            };

            self.metrics.record_dequeued(queued.enqueued_at.elapsed());
            match queued.msg {
                LoopMsg::Input(input) => {
                    let clock_time = self.clock.clock_time();
                    self.process(input, clock_time);
                }
                LoopMsg::Query(query) => query.execute(&self.app),
                LoopMsg::Subscribe(filter, reply) => self.subscribe(filter, &reply),
                LoopMsg::Shutdown => return,
            }
        }
    }

    /// Registers a subscriber and captures its snapshot in this loop
    /// turn, so no change can fall between the two.
    fn subscribe(&mut self, filter: ChangeFilter, reply: &SyncSender<Subscription>) {
        let (tx, rx) = sync_channel(self.subscriber_buffer_capacity);
        let subscription = Subscription {
            snapshot: self.app.snapshot(),
            changes: rx,
        };
        if reply.send(subscription).is_ok() {
            self.subscribers.push(SubscriberSender { tx, filter });
        }
    }

    fn process(&mut self, input: Input, clock_time: Duration) {
        // Effects that fail to dispatch synthesize follow-up inputs. The
        // local queue keeps their handling ordered without re-entering
        // the bounded FIFO.
        let mut inputs = VecDeque::from([input]);
        while let Some(input) = inputs.pop_front() {
            if let Some((kind, revision)) = completed_job(&input)
                && self
                    .active_jobs
                    .get(&kind)
                    .is_some_and(|active| active.revision == revision)
            {
                self.active_jobs.remove(&kind);
            }
            let started = Instant::now();
            let update = self.app.handle_at_clock_time(input, clock_time);
            self.metrics.record_handler_time(started.elapsed());
            self.metrics.record_input();
            self.next_deadline = update.next_deadline;
            for effect in update.effects {
                match effect {
                    Effect::Compute(job) => {
                        if let Some(failure) = self.dispatch(job) {
                            inputs.push_back(Input::ComputeFailed(failure));
                        }
                    }
                    Effect::CancelCompute(cancellation) => self.cancel(cancellation),
                }
            }
            if !update.changes.is_empty() {
                self.publish(&update.changes);
            }
        }
    }

    /// Hands a job to its worker. Returns the failure to feed back if
    /// there is no worker for the job's kind.
    fn dispatch(&mut self, job: ComputeJob) -> Option<ComputeFailure> {
        let kind = job.kind();
        let revision = job.revision();
        let cancellation = CancellationToken::default();
        let request = WorkerRequest {
            job,
            cancellation: cancellation.clone(),
        };
        let failed = match self.workers.get(&kind) {
            Some(jobs) => jobs.send(request).is_err(),
            None => true,
        };
        if !failed {
            self.active_jobs.insert(
                kind,
                ActiveJob {
                    revision,
                    cancellation,
                },
            );
        }
        failed.then(|| {
            tracing::error!("no worker available for {kind:?} compute jobs");
            self.metrics.record_worker_failure();
            ComputeFailure {
                kind,
                revision,
                message: format!("no worker available for {kind:?}"),
            }
        })
    }

    fn cancel(&self, cancellation: ComputeCancellation) {
        if let Some(active) = self.active_jobs.get(&cancellation.kind)
            && active.revision == cancellation.revision
        {
            active.cancellation.cancel();
        }
    }

    /// Publishes one change batch to every subscriber, dropping the
    /// subscriptions whose bounded buffer is full.
    fn publish(&mut self, changes: &[Change]) {
        let metrics = &self.metrics;
        self.subscribers.retain(|subscriber| {
            let changes = changes
                .iter()
                .filter(|change| subscriber.filter.includes(change.group()))
                .cloned()
                .collect::<Vec<_>>();
            if changes.is_empty() {
                return true;
            }
            match subscriber.tx.try_send(changes) {
                Ok(()) => true,
                Err(TrySendError::Full(_)) => {
                    tracing::warn!("dropping state-stream subscriber: change buffer full");
                    metrics.record_slow_subscriber_drop();
                    false
                }
                Err(TrySendError::Disconnected(_)) => false,
            }
        });
    }
}

impl Drop for CoreLoop {
    fn drop(&mut self) {
        for active in self.active_jobs.values() {
            active.cancellation.cancel();
        }
    }
}

struct SubscriberSender {
    tx: SyncSender<Vec<Change>>,
    filter: ChangeFilter,
}

/// One worker thread: runs jobs for one kind, one at a time, and returns
/// every outcome to the core as an input.
fn worker_loop(
    kind: ComputeKind,
    mut worker: Box<dyn Worker>,
    jobs: &Receiver<WorkerRequest>,
    results: &QueueSender,
    metrics: &Metrics,
) {
    // The revision the worker's cache belongs to. `None` forces a reset
    // before the next run: at startup, and after any failure so a
    // poisoned cache never reaches the next job.
    let mut cache_revision = None;
    while let Ok(request) = jobs.recv() {
        let WorkerRequest { job, cancellation } = request;
        let job_revision = job.revision();
        // A new revision invalidates all earlier work, including the
        // worker's cached state.
        let stale_cache = cache_revision != Some(job_revision);

        // Reset and run share one unwind boundary: a panic in either
        // becomes a typed failure for this job, so the core's job slot
        // is always freed and never waits forever.
        let outcome = std::panic::catch_unwind(AssertUnwindSafe(|| {
            if stale_cache {
                worker.reset();
            }
            worker.run(job, &cancellation)
        }));
        let input = match outcome {
            Ok(WorkerResult::Completed(result)) => {
                if result.kind() == kind && result.revision() == job_revision {
                    cache_revision = Some(job_revision);
                    Input::ComputeResult(result)
                } else {
                    let message = format!(
                        "worker returned {:?} at {:?} for {kind:?} at {job_revision:?}",
                        result.kind(),
                        result.revision()
                    );
                    tracing::error!("{message}");
                    metrics.record_worker_failure();
                    cache_revision = None;
                    Input::ComputeFailed(ComputeFailure {
                        kind,
                        revision: job_revision,
                        message,
                    })
                }
            }
            Ok(WorkerResult::Cancelled) => {
                cache_revision = None;
                Input::ComputeCancelled(ComputeCancellation {
                    kind,
                    revision: job_revision,
                })
            }
            Ok(WorkerResult::Failed(message)) => {
                tracing::error!("{kind:?} worker failed: {message}");
                metrics.record_worker_failure();
                cache_revision = None;
                Input::ComputeFailed(ComputeFailure {
                    kind,
                    revision: job_revision,
                    message,
                })
            }
            Err(panic) => {
                let message = panic_message(&panic);
                tracing::error!("{kind:?} worker panicked: {message}");
                metrics.record_worker_failure();
                cache_revision = None;
                Input::ComputeFailed(ComputeFailure {
                    kind,
                    revision: job_revision,
                    message,
                })
            }
        };
        if results.send(LoopMsg::Input(input)).is_err() {
            return; // the runtime stopped
        }
    }
}

fn completed_job(input: &Input) -> Option<(ComputeKind, updraft_core::ComputeRevision)> {
    match input {
        Input::ComputeResult(result) => Some((result.kind(), result.revision())),
        Input::ComputeFailed(failure) => Some((failure.kind, failure.revision)),
        Input::ComputeCancelled(cancellation) => Some((cancellation.kind, cancellation.revision)),
        Input::Clock { .. } | Input::Flight(_) => None,
    }
}

fn panic_message(panic: &(dyn std::any::Any + Send)) -> String {
    if let Some(message) = panic.downcast_ref::<&str>() {
        (*message).to_string()
    } else if let Some(message) = panic.downcast_ref::<String>() {
        message.clone()
    } else {
        "non-string panic payload".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use claims::{assert_matches, assert_ok};
    use updraft_core::flight::{Availability, FlightInput, Observation, Sourced};
    use updraft_units::{Length, PressureAltitude};

    #[test]
    fn queued_observation_uses_runtime_time_for_freshness() {
        let (tx, rx) = sync_channel(3);
        let clock_time = Duration::from_secs(4);
        let altitude = PressureAltitude::new(Length::from_meters(900.));
        let input = Input::Flight(FlightInput::PressureAltitude(Sourced::simulator(
            Observation::new(Duration::ZERO, altitude),
        )));
        let (subscription_tx, subscription_rx) = sync_channel(1);
        for msg in [
            LoopMsg::Input(input),
            LoopMsg::Subscribe(ChangeFilter::all(), subscription_tx),
            LoopMsg::Shutdown,
        ] {
            assert_ok!(tx.send(QueuedMsg {
                enqueued_at: Instant::now() - clock_time,
                msg,
            }));
        }
        let core_loop = CoreLoop {
            app: App::new(),
            rx,
            clock: Clock::with_elapsed(clock_time),
            metrics: Arc::new(Metrics::default()),
            subscribers: Vec::new(),
            subscriber_buffer_capacity: 1,
            workers: HashMap::new(),
            active_jobs: HashMap::new(),
            next_deadline: None,
        };
        core_loop.run();

        assert_matches!(
            assert_ok!(subscription_rx.recv())
                .snapshot
                .flight
                .pressure_altitude,
            Availability::LastKnown(value) if value == altitude
        );
    }

    #[test]
    fn describes_non_string_panic_payload() {
        let payload: Box<dyn std::any::Any + Send> = Box::new(42_u8);

        assert_eq!(panic_message(&*payload), "non-string panic payload");
    }
}
