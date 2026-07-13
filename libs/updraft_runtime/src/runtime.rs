use std::collections::VecDeque;
use std::sync::mpsc::{Receiver, RecvTimeoutError, SyncSender, sync_channel};
use std::thread::JoinHandle;
use std::time::Duration;
use std::{fmt, thread};

use updraft_core::{App, ComputeFailure, Effect, Input, Query};

use crate::Clock;

/// Everything the core loop accepts from the outside.
enum LoopMsg {
    Input(Input),
    Query(Box<dyn ErasedQuery>),
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

/// Configures and starts a [`Runtime`].
pub struct RuntimeBuilder {
    app: App,
    input_queue_capacity: usize,
}

impl RuntimeBuilder {
    /// Capacity of the bounded input FIFO (default 256). A full queue
    /// blocks the producer and never drops an input.
    #[must_use]
    pub fn input_queue_capacity(mut self, capacity: usize) -> Self {
        self.input_queue_capacity = capacity;
        self
    }

    #[must_use = "dropping the returned Runtime immediately shuts it down"]
    pub fn start(self) -> Runtime {
        let clock = Clock::new();
        let (tx, rx) = sync_channel(self.input_queue_capacity);
        let core_loop = CoreLoop {
            app: self.app,
            rx,
            clock: clock.clone(),
            next_deadline: None,
        };
        let core_thread = thread::Builder::new()
            .name("updraft-core".into())
            .spawn(move || core_loop.run())
            .expect("failed to spawn core loop thread");

        Runtime {
            handle: Handle { tx, clock },
            core_thread: Some(core_thread),
        }
    }
}

/// The shared runtime that owns one [`App`] and its input loop.
pub struct Runtime {
    handle: Handle,
    core_thread: Option<JoinHandle<()>>,
}

impl Runtime {
    pub fn builder(app: App) -> RuntimeBuilder {
        RuntimeBuilder {
            app,
            input_queue_capacity: 256,
        }
    }

    pub fn handle(&self) -> Handle {
        self.handle.clone()
    }

    /// Stops the core loop and joins the runtime thread.
    ///
    /// Queued messages ahead of the shutdown message are still handled.
    /// Inputs submitted concurrently may be queued behind it and not handled.
    /// Dropping the `Runtime` does the same thing.
    pub fn shutdown(mut self) {
        self.stop_and_join();
    }

    fn stop_and_join(&mut self) {
        let _ = self.handle.tx.send(LoopMsg::Shutdown);
        if let Some(core_thread) = self.core_thread.take()
            && let Err(panic) = core_thread.join()
        {
            tracing::error!("core loop thread panicked: {}", panic_message(&*panic));
        }
    }
}

impl Drop for Runtime {
    fn drop(&mut self) {
        self.stop_and_join();
    }
}

/// A cloneable handle for submitting inputs and querying state.
#[derive(Clone)]
pub struct Handle {
    tx: SyncSender<LoopMsg>,
    clock: Clock,
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

struct CoreLoop {
    app: App,
    rx: Receiver<LoopMsg>,
    clock: Clock,
    next_deadline: Option<Duration>,
}

impl CoreLoop {
    fn run(mut self) {
        loop {
            let msg = if let Some(deadline) = self.next_deadline {
                let clock_time = self.clock.clock_time();
                if clock_time >= deadline {
                    self.process(Input::Clock { clock_time });
                    continue;
                }
                match self.rx.recv_timeout(deadline.saturating_sub(clock_time)) {
                    Ok(msg) => msg,
                    Err(RecvTimeoutError::Timeout) => {
                        self.process(Input::Clock {
                            clock_time: self.clock.clock_time(),
                        });
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
                LoopMsg::Query(query) => query.execute(&self.app),
                LoopMsg::Shutdown => return,
            }
        }
    }

    fn process(&mut self, input: Input) {
        let mut inputs = VecDeque::from([input]);
        while let Some(input) = inputs.pop_front() {
            let update = self.app.handle(input);
            self.next_deadline = update.next_deadline;
            for effect in update.effects {
                let Effect::Compute(job) = effect;
                let kind = job.kind();
                let revision = job.revision();
                tracing::error!("no worker available for {kind:?} compute jobs");
                inputs.push_back(Input::ComputeFailed(ComputeFailure {
                    kind,
                    revision,
                    message: format!("no worker available for {kind:?}"),
                }));
            }
        }
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

    #[test]
    fn describes_non_string_panic_payload() {
        let payload: Box<dyn std::any::Any + Send> = Box::new(42_u8);

        assert_eq!(panic_message(&*payload), "non-string panic payload");
    }
}
