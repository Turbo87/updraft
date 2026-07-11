use std::collections::{HashMap, VecDeque};
use std::panic::AssertUnwindSafe;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender, SyncSender, TrySendError};
use std::sync::mpsc::{channel, sync_channel};
use std::thread::JoinHandle;
use std::{fmt, thread};

use updraft_core::{
    App, Change, ComputeFailure, ComputeJob, ComputeKind, Effect, Input, MonotonicTime, Query,
    QueryResult, Snapshot,
};

use crate::clock::Clock;
use crate::metrics::Metrics;
use crate::worker::Worker;

/// Everything the core loop accepts from the outside.
///
/// Inputs, queries, and subscriptions share the one bounded FIFO, so a
/// query observes all inputs submitted before it and a subscription's
/// snapshot capture cannot race with change delivery.
enum LoopMsg {
    Input(Input),
    Query(Query, SyncSender<QueryResult>),
    Subscribe(SyncSender<Subscription>),
    Shutdown,
}

/// Configures and starts a [`Runtime`].
pub struct RuntimeBuilder {
    app: App,
    input_queue_len: usize,
    subscriber_buffer_len: usize,
    workers: Vec<(ComputeKind, Box<dyn Worker>)>,
}

impl RuntimeBuilder {
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
    pub fn input_queue_len(mut self, len: usize) -> Self {
        self.input_queue_len = len;
        self
    }

    /// How many change batches a subscriber may buffer before it is
    /// dropped (default 64).
    #[must_use]
    pub fn subscriber_buffer_len(mut self, len: usize) -> Self {
        self.subscriber_buffer_len = len;
        self
    }

    /// Starts the core loop and worker threads.
    #[must_use = "dropping the returned Runtime immediately shuts it down"]
    pub fn start(self) -> Runtime {
        let clock = Clock::new();
        let metrics = Arc::new(Metrics::default());
        let (tx, rx) = sync_channel(self.input_queue_len);

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
            app: self.app,
            rx,
            clock: clock.clone(),
            metrics: Arc::clone(&metrics),
            subscribers: Vec::new(),
            subscriber_buffer_len: self.subscriber_buffer_len,
            workers: worker_channels,
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

/// The shared runtime: owns one [`App`], the input queue, the process
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
    pub fn builder(app: App) -> RuntimeBuilder {
        RuntimeBuilder {
            app,
            input_queue_len: 256,
            subscriber_buffer_len: 64,
            workers: Vec::new(),
        }
    }

    pub fn handle(&self) -> Handle {
        self.handle.clone()
    }

    /// Stops the core loop and joins all runtime threads.
    ///
    /// Queued inputs ahead of the shutdown message are still handled.
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
                log::error!("core loop thread panicked: {}", panic_message(&*panic));
            }
        }
        // The core loop owned the job senders, so the workers' queues are
        // now disconnected and the threads wind down. A worker thread that
        // panicked past its own unwind boundary surfaces here.
        for thread in self.worker_threads.drain(..) {
            if let Err(panic) = thread.join() {
                log::error!("worker thread panicked: {}", panic_message(&*panic));
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
    tx: SyncSender<LoopMsg>,
    clock: Clock,
    metrics: Arc<Metrics>,
}

impl Handle {
    /// The runtime's process-wide clock, for adapters that stamp their
    /// observations.
    pub fn clock(&self) -> &Clock {
        &self.clock
    }

    /// The current time on the process-wide monotonic timeline.
    pub fn now(&self) -> MonotonicTime {
        self.clock.now()
    }

    pub fn metrics(&self) -> &Metrics {
        &self.metrics
    }

    /// Submits one input to the core.
    ///
    /// Blocks while the bounded queue is full (backpressure). An input is
    /// never dropped.
    pub fn submit(&self, input: Input) -> Result<(), RuntimeStopped> {
        self.tx
            .send(LoopMsg::Input(input))
            .map_err(|_| RuntimeStopped)
    }

    /// Answers a read-only query against current state.
    pub fn query(&self, query: Query) -> Result<QueryResult, RuntimeStopped> {
        let (tx, rx) = sync_channel(1);
        self.tx
            .send(LoopMsg::Query(query, tx))
            .map_err(|_| RuntimeStopped)?;
        rx.recv().map_err(|_| RuntimeStopped)
    }

    /// Opens a state-stream subscription: a snapshot first, then
    /// FIFO-ordered change batches.
    ///
    /// Registration and snapshot capture happen in one core-loop turn, so
    /// no change can fall between them.
    pub fn subscribe(&self) -> Result<Subscription, RuntimeStopped> {
        let (tx, rx) = sync_channel(1);
        self.tx
            .send(LoopMsg::Subscribe(tx))
            .map_err(|_| RuntimeStopped)?;
        rx.recv().map_err(|_| RuntimeStopped)
    }
}

/// The runtime's core loop has stopped and no longer accepts requests.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RuntimeStopped;

impl fmt::Display for RuntimeStopped {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("the runtime has stopped")
    }
}

impl std::error::Error for RuntimeStopped {}

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
    rx: Receiver<LoopMsg>,
    clock: Clock,
    metrics: Arc<Metrics>,
    subscribers: Vec<SyncSender<Vec<Change>>>,
    subscriber_buffer_len: usize,
    workers: HashMap<ComputeKind, Sender<ComputeJob>>,
    next_deadline: Option<MonotonicTime>,
}

impl CoreLoop {
    fn run(mut self) {
        loop {
            let msg = if let Some(deadline) = self.next_deadline {
                let now = self.clock.now();
                if now >= deadline {
                    self.process(Input::Clock { now });
                    continue;
                }
                match self
                    .rx
                    .recv_timeout(deadline.saturating_duration_since(now))
                {
                    Ok(msg) => msg,
                    Err(RecvTimeoutError::Timeout) => {
                        let now = self.clock.now();
                        self.process(Input::Clock { now });
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

            match msg {
                LoopMsg::Input(input) => self.process(input),
                LoopMsg::Query(query, reply) => {
                    let _ = reply.send(self.app.query(query));
                }
                LoopMsg::Subscribe(reply) => self.subscribe(&reply),
                LoopMsg::Shutdown => return,
            }
        }
    }

    /// Registers a subscriber and captures its snapshot in this loop
    /// turn, so no change can fall between the two.
    fn subscribe(&mut self, reply: &SyncSender<Subscription>) {
        let (tx, rx) = sync_channel(self.subscriber_buffer_len);
        let subscription = Subscription {
            snapshot: self.app.snapshot(),
            changes: rx,
        };
        if reply.send(subscription).is_ok() {
            self.subscribers.push(tx);
        }
    }

    fn process(&mut self, input: Input) {
        // Effects that fail to dispatch synthesize follow-up inputs. The
        // local queue keeps their handling ordered without re-entering
        // the bounded FIFO.
        let mut inputs = VecDeque::from([input]);
        while let Some(input) = inputs.pop_front() {
            let update = self.app.handle(input);
            self.metrics.record_input();
            self.next_deadline = update.next_deadline;
            if !update.changes.is_empty() {
                self.publish(&update.changes);
            }
            for effect in update.effects {
                match effect {
                    Effect::Compute(job) => {
                        if let Some(failure) = self.dispatch(job) {
                            inputs.push_back(Input::ComputeFailed(failure));
                        }
                    }
                }
            }
        }
    }

    /// Hands a job to its worker. Returns the failure to feed back if
    /// there is no worker for the job's kind.
    fn dispatch(&mut self, job: ComputeJob) -> Option<ComputeFailure> {
        let kind = job.kind();
        let epoch = job.epoch();
        let failed = match self.workers.get(&kind) {
            Some(jobs) => jobs.send(job).is_err(),
            None => true,
        };
        failed.then(|| {
            log::error!("no worker available for {kind:?} compute jobs");
            self.metrics.record_worker_failure();
            ComputeFailure {
                kind,
                epoch,
                message: format!("no worker available for {kind:?}"),
            }
        })
    }

    /// Publishes one change batch to every subscriber, dropping the
    /// subscriptions whose bounded buffer is full.
    fn publish(&mut self, changes: &[Change]) {
        let metrics = &self.metrics;
        self.subscribers
            .retain(|subscriber| match subscriber.try_send(changes.to_vec()) {
                Ok(()) => true,
                Err(TrySendError::Full(_)) => {
                    log::warn!("dropping state-stream subscriber: change buffer full");
                    metrics.record_slow_subscriber_drop();
                    false
                }
                Err(TrySendError::Disconnected(_)) => false,
            });
    }
}

/// One worker thread: runs jobs for one kind, one at a time, and returns
/// every outcome to the core as an input.
fn worker_loop(
    kind: ComputeKind,
    mut worker: Box<dyn Worker>,
    jobs: &Receiver<ComputeJob>,
    results: &SyncSender<LoopMsg>,
    metrics: &Metrics,
) {
    // The epoch the worker's cache belongs to. `None` forces a reset
    // before the next run: at startup, and after any failure so a
    // poisoned cache never reaches the next job.
    let mut cache_epoch = None;
    while let Ok(job) = jobs.recv() {
        let job_epoch = job.epoch();
        // A new epoch invalidates all earlier work, including the
        // worker's cached state.
        let stale_cache = cache_epoch != Some(job_epoch);

        // Reset and run share one unwind boundary: a panic in either
        // becomes a typed failure for this job, so the core's job slot
        // is always freed and never waits forever.
        let outcome = std::panic::catch_unwind(AssertUnwindSafe(|| {
            if stale_cache {
                worker.reset();
            }
            worker.run(job)
        }));
        let input = match outcome {
            Ok(Ok(result)) => {
                cache_epoch = Some(job_epoch);
                Input::ComputeResult(result)
            }
            Ok(Err(message)) => {
                log::error!("{kind:?} worker failed: {message}");
                metrics.record_worker_failure();
                cache_epoch = None;
                Input::ComputeFailed(ComputeFailure {
                    kind,
                    epoch: job_epoch,
                    message,
                })
            }
            Err(panic) => {
                let message = panic_message(&panic);
                log::error!("{kind:?} worker panicked: {message}");
                metrics.record_worker_failure();
                cache_epoch = None;
                Input::ComputeFailed(ComputeFailure {
                    kind,
                    epoch: job_epoch,
                    message,
                })
            }
        };
        if results.send(LoopMsg::Input(input)).is_err() {
            return; // the runtime stopped
        }
    }
}

fn panic_message(panic: &(dyn std::any::Any + Send)) -> String {
    if let Some(message) = panic.downcast_ref::<&str>() {
        (*message).to_string()
    } else if let Some(message) = panic.downcast_ref::<String>() {
        message.clone()
    } else {
        "worker panicked".to_string()
    }
}
