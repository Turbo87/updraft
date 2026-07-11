//! The shared Updraft runtime.
//!
//! The runtime is the layer around the deterministic [`updraft_core`]. It
//! owns one [`App`] on a single loop thread, a bounded FIFO input queue,
//! the process clock, one compute worker per kind, and the snapshot-first
//! state stream. Callers submit typed inputs and subscribe to a stream of
//! [`Change`] batches; hosts (the axum server, the Tauri shell) add only
//! transport bindings on top.
//!
//! It is built on `std` threads and channels — no async runtime — which
//! keeps the worker path simple and deterministic to test. The one
//! illustrative worker here reduces a batch of samples; see the
//! `async_compute` example for an end-to-end run.

mod timer;
mod worker;

use std::sync::Arc;
use std::sync::mpsc::{self, Receiver, SyncSender, sync_channel};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use updraft_core::{App, Change, ComputeJob, ComputeResult, Effect, Input, Sample, Snapshot};

pub use updraft_core::{MonotonicTime, Reduction};

use timer::TimerMsg;
use worker::WorkerMsg;

/// The strategy the worker uses to execute a job.
///
/// Defaults to [`ComputeJob::run`]; tests substitute a function that panics
/// to exercise the failure path. This is the seam where a real system would
/// plug in a heavier or stateful worker.
pub type ComputeFn = Arc<dyn Fn(&ComputeJob) -> ComputeResult + Send + Sync>;

/// Configuration for a [`Runtime`].
#[derive(Clone)]
pub struct RuntimeConfig {
    /// Capacity of the bounded input queue. A full queue blocks producers;
    /// inputs are never dropped.
    pub input_capacity: usize,
    /// Per-subscriber change-buffer capacity. A subscriber whose buffer
    /// fills is a slow client and is dropped.
    pub subscriber_capacity: usize,
    /// The simulated cost of a worker job.
    pub worker_delay: Duration,
    /// The demo domain's re-evaluation cadence, in milliseconds.
    pub interval_millis: u64,
    /// How the worker executes a job.
    pub compute: ComputeFn,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            input_capacity: 256,
            subscriber_capacity: 64,
            worker_delay: Duration::from_millis(5),
            interval_millis: 1000,
            compute: Arc::new(|job: &ComputeJob| job.run()),
        }
    }
}

/// A live subscription to the state stream.
///
/// Per the core contract, the `snapshot` is captured in the same loop turn
/// the subscription is registered, so no change can fall between them.
/// Subsequent changes arrive on `changes` as FIFO-ordered batches.
pub struct Subscription {
    /// The shared state at the moment of subscription.
    pub snapshot: Snapshot,
    /// FIFO-ordered batches of changes produced after the snapshot.
    pub changes: Receiver<Vec<Change>>,
}

/// A cloneable handle for submitting inputs to the loop, used by the worker
/// and timer threads.
#[derive(Clone)]
pub(crate) struct LoopSender(SyncSender<LoopMsg>);

impl LoopSender {
    /// Submits an input, returning an error if the loop has stopped.
    pub(crate) fn send_input(&self, input: Input) -> Result<(), mpsc::SendError<LoopMsg>> {
        self.0.send(LoopMsg::Input(input))
    }
}

/// A message to the loop thread. Inputs share the queue with the two
/// control messages so that subscription registration is serialized with
/// input handling.
pub(crate) enum LoopMsg {
    /// Handle a core input.
    Input(Input),
    /// Register a subscriber and reply with the captured snapshot.
    Subscribe(SyncSender<Vec<Change>>, mpsc::Sender<Snapshot>),
    /// Stop the loop; the runtime is shutting down.
    Shutdown,
}

struct Threads {
    loop_thread: JoinHandle<()>,
    worker_thread: JoinHandle<()>,
    timer_thread: JoinHandle<()>,
}

/// The running runtime: submit inputs, subscribe to changes, and shut down.
pub struct Runtime {
    tx: SyncSender<LoopMsg>,
    origin: Instant,
    subscriber_capacity: usize,
    threads: Option<Threads>,
}

impl Runtime {
    /// Starts a runtime with the default configuration.
    pub fn new() -> Self {
        Self::with_config(RuntimeConfig::default())
    }

    /// Starts a runtime with the given configuration, spawning the loop,
    /// worker, and timer threads.
    pub fn with_config(config: RuntimeConfig) -> Self {
        let RuntimeConfig {
            input_capacity,
            subscriber_capacity,
            worker_delay,
            interval_millis,
            compute,
        } = config;

        let origin = Instant::now();
        let (tx, rx) = sync_channel::<LoopMsg>(input_capacity);
        let (worker_tx, worker_rx) = sync_channel::<WorkerMsg>(1);
        let (timer_tx, timer_rx) = mpsc::channel::<TimerMsg>();
        let loop_sender = LoopSender(tx.clone());

        let worker_sender = loop_sender.clone();
        let worker_thread =
            thread::spawn(move || worker::run(&worker_rx, &worker_sender, worker_delay, compute));

        let timer_sender = loop_sender;
        let timer_thread = thread::spawn(move || timer::run(&timer_rx, &timer_sender, origin));

        let loop_thread = thread::spawn(move || {
            run_loop(
                &rx,
                &mut App::with_interval_millis(interval_millis),
                &worker_tx,
                &timer_tx,
            );
        });

        Self {
            tx,
            origin,
            subscriber_capacity,
            threads: Some(Threads {
                loop_thread,
                worker_thread,
                timer_thread,
            }),
        }
    }

    /// The current time on the runtime's monotonic timeline.
    pub fn now(&self) -> MonotonicTime {
        MonotonicTime::from_nanos(self.origin.elapsed().as_nanos() as u64)
    }

    /// Submits a sample observation, stamped with the current time. Blocks
    /// if the input queue is full (backpressure); does nothing once the
    /// runtime has stopped.
    pub fn submit_observe(&self, value: u32) {
        let sample = Sample {
            value,
            observed_at: self.now(),
        };
        let _ = self.tx.send(LoopMsg::Input(Input::Observe(sample)));
    }

    /// Subscribes to the state stream, returning the current snapshot and a
    /// receiver of change batches.
    pub fn subscribe(&self) -> Subscription {
        let (sub_tx, sub_rx) = sync_channel::<Vec<Change>>(self.subscriber_capacity);
        let (reply_tx, reply_rx) = mpsc::channel::<Snapshot>();

        let snapshot = if self.tx.send(LoopMsg::Subscribe(sub_tx, reply_tx)).is_ok() {
            reply_rx.recv().unwrap_or_default()
        } else {
            Snapshot::default()
        };

        Subscription {
            snapshot,
            changes: sub_rx,
        }
    }

    /// Stops the runtime and waits for its threads to finish.
    pub fn shutdown(mut self) {
        self.teardown();
    }

    fn teardown(&mut self) {
        let Some(threads) = self.threads.take() else {
            return;
        };
        let _ = self.tx.send(LoopMsg::Shutdown);
        let _ = threads.loop_thread.join();
        let _ = threads.worker_thread.join();
        let _ = threads.timer_thread.join();
    }
}

impl Default for Runtime {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for Runtime {
    fn drop(&mut self) {
        self.teardown();
    }
}

/// The loop thread body: handle inputs, broadcast changes, dispatch
/// effects, and arm the timer, until told to stop.
fn run_loop(
    rx: &Receiver<LoopMsg>,
    app: &mut App,
    worker_tx: &SyncSender<WorkerMsg>,
    timer_tx: &mpsc::Sender<TimerMsg>,
) {
    let mut subscribers: Vec<SyncSender<Vec<Change>>> = Vec::new();

    while let Ok(msg) = rx.recv() {
        match msg {
            LoopMsg::Input(input) => {
                let update = app.handle(input);

                if !update.changes.is_empty() {
                    // A subscriber whose bounded buffer is full is a slow
                    // client: drop it silently rather than block the loop.
                    subscribers.retain(|sub| sub.try_send(update.changes.clone()).is_ok());
                }

                for effect in update.effects {
                    match effect {
                        Effect::Compute(job) => {
                            // The core keeps one job outstanding per kind,
                            // so this bounded send never actually blocks.
                            let _ = worker_tx.send(WorkerMsg::Run(job));
                        }
                    }
                }

                let _ = timer_tx.send(TimerMsg::Arm(update.next_deadline));
            }
            LoopMsg::Subscribe(sub, reply) => {
                // Capture the snapshot and register in one turn, so no
                // change can slip between them.
                let snapshot = app.snapshot();
                subscribers.push(sub);
                let _ = reply.send(snapshot);
            }
            LoopMsg::Shutdown => break,
        }
    }

    let _ = worker_tx.send(WorkerMsg::Stop);
    let _ = timer_tx.send(TimerMsg::Stop);
}
