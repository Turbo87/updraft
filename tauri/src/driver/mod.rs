use std::{
    collections::BTreeMap,
    time::{Duration, Instant},
};
use tokio::sync::{mpsc, oneshot};
use updraft_core::{
    AirspaceState, ConnectionSpec, Core, Effect, ExternalDeviceId, Input, SettingsSnapshot, Tick,
    Timestamp, Topic, Update,
};

/// Receives every emitted topic. Returns `false` once its consumer is
/// gone, which is how the driver prunes dead subscribers.
pub type Sink = Box<dyn Fn(&Topic) -> bool + Send>;

pub type StopFn = Box<dyn FnOnce() + Send>;

/// Brings up the transport for a connection the core asked for.
///
/// Injected rather than called directly so the driver carries no
/// dependency on the transport layer and can be tested with a stub.
pub type OpenFn = Box<dyn Fn(ExternalDeviceId, ConnectionSpec, DriverHandle) -> StopFn + Send>;

pub type PersistFn = Box<dyn Fn(SettingsSnapshot) + Send>;

#[derive(Default)]
struct ActiveTransports {
    workers: BTreeMap<ExternalDeviceId, StopFn>,
}

impl ActiveTransports {
    fn open(
        &mut self,
        device_id: ExternalDeviceId,
        spec: ConnectionSpec,
        handle: DriverHandle,
        open: &OpenFn,
    ) {
        self.close(device_id);
        self.workers
            .insert(device_id, open(device_id, spec, handle));
    }

    fn close(&mut self, device_id: ExternalDeviceId) {
        if let Some(stop) = self.workers.remove(&device_id) {
            stop();
        }
    }
}

struct Request<I: Input> {
    input: I,
    reply: oneshot::Sender<I::Response>,
}

trait ErasedInput: Send {
    fn run(self: Box<Self>, driver: &mut DriverState, at: Timestamp);
}

enum Message {
    Input(Box<dyn ErasedInput>),
    Subscribe(Sink),
}

#[derive(Clone, Copy, Debug, thiserror::Error, PartialEq, Eq)]
#[error("driver stopped")]
pub struct DriverStopped;

#[derive(Clone)]
pub struct DriverHandle {
    messages: mpsc::UnboundedSender<Message>,
}

impl DriverHandle {
    /// Queues an input and waits until its effects are dispatched.
    pub async fn send<I: Input>(&self, input: I) -> Result<I::Response, DriverStopped> {
        let (reply, response) = oneshot::channel();
        self.messages
            .send(Message::Input(Box::new(Request { input, reply })))
            .map_err(|_| DriverStopped)?;
        response.await.map_err(|_| DriverStopped)
    }

    /// Registers a sink. It immediately receives the current onboarding
    /// value for each topic. Traffic uses a `TrafficUpdate::Snapshot` update.
    pub fn subscribe(&self, sink: Sink) {
        let _ = self.messages.send(Message::Subscribe(sink));
    }
}

impl<I: Input> ErasedInput for Request<I> {
    fn run(self: Box<Self>, driver: &mut DriverState, at: Timestamp) {
        let Request { input, reply } = *self;
        let response = driver.apply(input, at);
        let _ = reply.send(response);
    }
}

struct DriverState {
    core: Core,
    sinks: Vec<Sink>,
    transports: ActiveTransports,
    open: OpenFn,
    persist: PersistFn,
    handle: DriverHandle,
}

impl DriverState {
    fn apply<I: Input>(&mut self, input: I, at: Timestamp) -> I::Response {
        let Update { effects, response } = self.core.apply(input, at);
        for effect in effects {
            self.dispatch(effect);
        }
        response
    }

    fn subscribe(&mut self, sink: Sink) {
        if self.core.topics().iter().all(&sink) {
            self.sinks.push(sink);
        }
    }

    fn dispatch(&mut self, effect: Effect) {
        match effect {
            Effect::Emit(topic) => self.sinks.retain(|sink| sink(&topic)),
            Effect::PersistSettings(snapshot) => (self.persist)(snapshot),
            Effect::OpenConnection { device_id, spec } => {
                self.transports
                    .open(device_id, spec, self.handle.clone(), &self.open);
            }
            Effect::CloseConnection { device_id } => self.transports.close(device_id),
        }
    }
}

/// Owns the single mutable [`Core`] and the subscriber list, and drives
/// both.
///
/// Keeping subscribers inside the task means there is no shared state and
/// therefore no lock, and the current-topics reply needed on subscribe is
/// just a local call.
///
/// The driver runs for the lifetime of the process. Holding no handles is
/// an ordinary state, not a reason to stop: a webview between reloads has
/// no subscriber, and a profile with no devices has no transport. If
/// stopping ever needs to be possible, it belongs as an explicit
/// [`Message`] variant rather than as a consequence of dropping a handle.
pub struct Driver;

impl Driver {
    pub fn spawn(
        snapshot: SettingsSnapshot,
        airspace: AirspaceState,
        open: OpenFn,
        persist: PersistFn,
        tick_interval: Duration,
    ) -> DriverHandle {
        Self::spawn_task(snapshot, airspace, open, persist, tick_interval).0
    }

    fn spawn_task(
        snapshot: SettingsSnapshot,
        airspace: AirspaceState,
        open: OpenFn,
        persist: PersistFn,
        tick_interval: Duration,
    ) -> (DriverHandle, tokio::task::JoinHandle<()>) {
        let (sender, mut receiver) = mpsc::unbounded_channel();
        let handle = DriverHandle { messages: sender };
        let driver_handle = handle.clone();

        let task = tokio::spawn(async move {
            let started = Instant::now();
            let mut state = DriverState {
                core: Core::with_airspace(snapshot, airspace),
                sinks: Vec::new(),
                transports: ActiveTransports::default(),
                open,
                persist,
                handle: driver_handle,
            };
            let mut ticker = tokio::time::interval(tick_interval);
            ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

            state.apply(updraft_core::Start, Timestamp::from_millis(0));

            loop {
                let message = tokio::select! {
                    _ = ticker.tick() => {
                        let at = Timestamp::from_millis(started.elapsed().as_millis() as u64);
                        state.apply(Tick, at);
                        continue;
                    }
                    received = receiver.recv() => match received {
                        Some(message) => message,
                        // Unreachable while the driver holds its own
                        // handle. Exiting is still the right response if
                        // that ever changes.
                        None => break,
                    },
                };

                match message {
                    Message::Subscribe(sink) => state.subscribe(sink),
                    Message::Input(input) => {
                        let at = Timestamp::from_millis(started.elapsed().as_millis() as u64);
                        input.run(&mut state, at);
                    }
                }
            }
        });

        (handle, task)
    }
}

#[cfg(test)]
pub mod tests;
