//! The shared host runtime (see `docs/design/core.md`).
//!
//! Owns the [`App`], the bounded input queue, effect execution, and the
//! state-stream fan-out. Both the axum server and the Tauri shell embed
//! this same runtime; hosts contribute only transport bindings on top.

use std::time::{Duration, Instant};

use tokio::sync::mpsc::error::TrySendError;
use tokio::sync::{mpsc, oneshot};
use updraft_core::{App, Change, Effect, Input, Snapshot};

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
/// A panic inside the loop aborts the process: a dead runtime must fail
/// fast instead of leaving every client frozen on a stale stream.
pub fn spawn(app: App) -> RuntimeHandle {
    let (requests, receiver) = mpsc::channel(INPUT_CAPACITY);

    let task = tokio::spawn(run(app, receiver));
    tokio::spawn(async move {
        if let Err(error) = task.await
            && error.is_panic()
        {
            tracing::error!(%error, "core runtime panicked, aborting");
            std::process::abort();
        }
    });

    RuntimeHandle { requests }
}

async fn run(mut app: App, mut requests: mpsc::Receiver<Request>) {
    let mut subscribers: Vec<mpsc::Sender<StateMessage>> = Vec::new();

    while let Some(request) = requests.recv().await {
        match request {
            Request::Submit(input) => {
                let started = Instant::now();
                let update = app.handle(input);
                let elapsed = started.elapsed();
                if elapsed >= SLOW_HANDLER_THRESHOLD {
                    tracing::warn!(?elapsed, "slow input handler");
                }

                update.effects.into_iter().for_each(execute_effect);

                if !update.changes.is_empty() {
                    publish(&mut subscribers, StateMessage::Changes(update.changes));
                }
            }
            // Registering the subscriber and capturing the snapshot happen
            // in the same loop iteration, so no change can fall between
            // them: a late subscriber's snapshot already contains
            // everything submitted before it.
            Request::Subscribe(reply) => {
                let (sender, receiver) = mpsc::channel(SUBSCRIBER_CAPACITY);
                sender
                    .try_send(StateMessage::Snapshot(app.snapshot()))
                    .expect("fresh subscriber queue has capacity");
                subscribers.push(sender);
                let _ = reply.send(StateStream { messages: receiver });
            }
        }
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

fn execute_effect(effect: Effect) {
    match effect {}
}

/// Cloneable handle for feeding inputs and opening state streams.
#[derive(Clone)]
pub struct RuntimeHandle {
    requests: mpsc::Sender<Request>,
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
