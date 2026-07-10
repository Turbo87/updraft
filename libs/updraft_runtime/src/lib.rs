//! The shared host runtime (see `docs/design/core.md`).
//!
//! Owns the [`App`], the bounded input queue, the clock, effect
//! execution (compute workers on their own threads), and the
//! state-stream fan-out. Both the axum server and the Tauri shell embed
//! this same runtime; hosts contribute only transport bindings on top.

use std::panic::AssertUnwindSafe;
use std::sync::mpsc as std_mpsc;
use std::time::Duration;

use tokio::sync::mpsc::error::TrySendError;
use tokio::sync::{mpsc, oneshot};
use tokio::time::Instant;
use updraft_core::workers::TrackDistanceWorker;
use updraft_core::{App, Change, ComputeJob, Effect, Input, JobOutcome, MonotonicTime, Snapshot};

/// Inputs queue here; a full queue blocks the producer, it never drops.
const INPUT_CAPACITY: usize = 256;

/// Per-subscriber buffer. A subscriber that stays this far behind is
/// dropped and recovers by resubscribing.
const SUBSCRIBER_CAPACITY: usize = 64;

/// Handler-duration watchdog. The end-to-end warning budget is 100 ms
/// (see `docs/design/core.md`), so a single input spending 10 ms inside
/// `handle()` deserves a warning long before it becomes dangerous.
const SLOW_HANDLER_THRESHOLD: Duration = Duration::from_millis(10);

/// One frame of the client state stream: a full snapshot on subscribe,
/// then batches of changes in input order.
#[derive(Clone, Debug, PartialEq)]
pub enum StateMessage {
    Snapshot(Snapshot),
    Changes(Vec<Change>),
}

enum Request {
    Submit(Input),
    Subscribe(oneshot::Sender<StateStream>),
}

/// Starts the runtime loop on the ambient tokio runtime.
///
/// The moment of spawning defines the process-wide monotonic epoch: the
/// runtime stamps its own clock inputs against it, and adapters obtain
/// matching timestamps from [`RuntimeHandle::now`].
///
/// A panic inside the loop aborts the process: a dead runtime must fail
/// fast instead of leaving every client frozen on a stale stream.
pub fn spawn(app: App) -> RuntimeHandle {
    let epoch = Instant::now();
    let (requests, receiver) = mpsc::channel(INPUT_CAPACITY);
    let jobs = spawn_track_distance_worker(requests.downgrade());

    let task = tokio::spawn(run(app, receiver, epoch, jobs));
    tokio::spawn(async move {
        if let Err(error) = task.await
            && error.is_panic()
        {
            tracing::error!(%error, "core runtime panicked, aborting");
            std::process::abort();
        }
    });

    RuntimeHandle { requests, epoch }
}

async fn run(
    mut app: App,
    mut requests: mpsc::Receiver<Request>,
    epoch: Instant,
    jobs: std_mpsc::Sender<ComputeJob>,
) {
    let mut subscribers: Vec<mpsc::Sender<StateMessage>> = Vec::new();
    let mut next_deadline: Option<Instant> = None;

    loop {
        let input = tokio::select! {
            biased;

            request = requests.recv() => match request {
                None => break,
                // Registering the subscriber and capturing the snapshot
                // happen in the same loop iteration, so no change can fall
                // between them: a late subscriber's snapshot already
                // contains everything submitted before it.
                Some(Request::Subscribe(reply)) => {
                    let (sender, receiver) = mpsc::channel(SUBSCRIBER_CAPACITY);
                    sender
                        .try_send(StateMessage::Snapshot(app.snapshot()))
                        .expect("fresh subscriber queue has capacity");
                    subscribers.push(sender);
                    let _ = reply.send(StateStream { messages: receiver });
                    continue;
                }
                Some(Request::Submit(input)) => input,
            },

            // The single host timer, armed from the previous update's
            // earliest deadline; elapsing turns into a clock input.
            () = tokio::time::sleep_until(next_deadline.unwrap_or_else(Instant::now)),
                if next_deadline.is_some() =>
            {
                Input::Clock(MonotonicTime::from_duration(Instant::now() - epoch))
            }
        };

        let started = Instant::now();
        let update = app.handle(input);
        let elapsed = started.elapsed();
        if elapsed >= SLOW_HANDLER_THRESHOLD {
            tracing::warn!(?elapsed, queue_depth = requests.len(), "slow input handler");
        }

        for effect in update.effects {
            match effect {
                // The worker channel only disconnects when this loop is
                // shutting down, so a failed send is safe to ignore.
                Effect::Compute(job) => drop(jobs.send(job)),
            }
        }

        if !update.changes.is_empty() {
            publish(&mut subscribers, StateMessage::Changes(update.changes));
        }

        next_deadline = update
            .next_deadline
            .map(|deadline| epoch + deadline.as_duration());
    }
}

fn publish(subscribers: &mut Vec<mpsc::Sender<StateMessage>>, message: StateMessage) {
    subscribers.retain(|subscriber| match subscriber.try_send(message.clone()) {
        Ok(()) => true,
        Err(TrySendError::Full(_)) => {
            tracing::warn!("dropping state subscriber that fell behind");
            false
        }
        Err(TrySendError::Closed(_)) => false,
    });
}

/// One persistent worker thread per job kind: the worker retains state
/// between rounds, one-in-flight scheduling in the core serializes all
/// access to it, and outcomes re-enter the runtime as ordinary inputs.
/// The weak sender keeps the worker from holding the runtime open; the
/// thread exits when the runtime loop drops the job channel.
fn spawn_track_distance_worker(results: mpsc::WeakSender<Request>) -> std_mpsc::Sender<ComputeJob> {
    let (jobs, job_queue) = std_mpsc::channel::<ComputeJob>();

    std::thread::spawn(move || {
        let mut worker = TrackDistanceWorker::default();
        while let Ok(job) = job_queue.recv() {
            let outcome = run_job(&mut worker, &job);
            let Some(results) = results.upgrade() else {
                break;
            };
            if results
                .blocking_send(Request::Submit(Input::Job(outcome)))
                .is_err()
            {
                break;
            }
        }
    });

    jobs
}

/// A worker panic becomes an ordinary [`JobOutcome::Failed`] input —
/// with one-in-flight bookkeeping, a lost completion would otherwise
/// wedge the job kind for the rest of the flight. The worker instance is
/// replaced because its state may be mid-update.
fn run_job(worker: &mut TrackDistanceWorker, job: &ComputeJob) -> JobOutcome {
    match std::panic::catch_unwind(AssertUnwindSafe(|| worker.run(job))) {
        Ok(result) => JobOutcome::Completed {
            epoch: job.epoch(),
            result,
        },
        Err(_) => {
            *worker = TrackDistanceWorker::default();
            tracing::error!(kind = ?job.kind(), "compute worker panicked");
            JobOutcome::Failed {
                kind: job.kind(),
                epoch: job.epoch(),
            }
        }
    }
}

/// Cloneable handle for feeding inputs and opening state streams.
#[derive(Clone)]
pub struct RuntimeHandle {
    requests: mpsc::Sender<Request>,
    epoch: Instant,
}

impl RuntimeHandle {
    /// Queues one input, waiting while the queue is full: inputs are never
    /// dropped (see `docs/design/core.md`, "Inputs").
    pub async fn submit(&self, input: Input) -> Result<(), RuntimeStopped> {
        self.requests
            .send(Request::Submit(input))
            .await
            .map_err(|_| RuntimeStopped)
    }

    /// Opens a state stream that starts with the current [`Snapshot`].
    pub async fn subscribe(&self) -> Result<StateStream, RuntimeStopped> {
        let (reply, response) = oneshot::channel();
        self.requests
            .send(Request::Subscribe(reply))
            .await
            .map_err(|_| RuntimeStopped)?;
        response.await.map_err(|_| RuntimeStopped)
    }

    /// The current time on the runtime's monotonic timeline, for adapters
    /// stamping observations (single process-wide epoch by construction).
    pub fn now(&self) -> MonotonicTime {
        MonotonicTime::from_duration(Instant::now() - self.epoch)
    }
}

/// The runtime loop has stopped and no longer accepts requests.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RuntimeStopped;

impl std::fmt::Display for RuntimeStopped {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("core runtime has stopped")
    }
}

impl std::error::Error for RuntimeStopped {}

/// A subscriber's view of the state stream.
///
/// `None` means the subscription ended (runtime shutdown, or this
/// subscriber was dropped for falling behind) and the client should
/// resubscribe for a fresh snapshot.
pub struct StateStream {
    messages: mpsc::Receiver<StateMessage>,
}

impl StateStream {
    pub async fn recv(&mut self) -> Option<StateMessage> {
        self.messages.recv().await
    }
}
