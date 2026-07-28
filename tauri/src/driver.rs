use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use updraft_core::{
    ConnectionId, ConnectionSpec, Core, CoreConfig, Effect, Input, Timestamp, Topic,
};

/// Receives every emitted topic. Returns `false` once its consumer is
/// gone, which is how the driver prunes dead subscribers.
pub type Sink = Box<dyn Fn(&Topic) -> bool + Send>;

/// Brings up the transport for a connection the core asked for.
///
/// Injected rather than called directly so the driver carries no
/// dependency on the transport layer and can be tested with a stub.
pub type OpenFn = Box<dyn Fn(ConnectionId, ConnectionSpec, DriverHandle) + Send>;

enum Message {
    Input(Input),
    Subscribe(Sink),
}

#[derive(Clone)]
pub struct DriverHandle {
    messages: mpsc::UnboundedSender<Message>,
}

impl DriverHandle {
    /// Queues an input. A dropped driver makes this a no-op rather than an
    /// error, because shutdown races are expected during teardown.
    pub fn send(&self, input: Input) {
        let _ = self.messages.send(Message::Input(input));
    }

    /// Registers a sink. It immediately receives the current value of
    /// every topic, so a client that reloads mid-flight resyncs without
    /// needing a distinct snapshot message.
    pub fn subscribe(&self, sink: Sink) {
        let _ = self.messages.send(Message::Subscribe(sink));
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
    pub fn spawn(config: CoreConfig, open: OpenFn, tick_interval: Duration) -> DriverHandle {
        let (sender, mut receiver) = mpsc::unbounded_channel();
        let handle = DriverHandle { messages: sender };
        let driver_handle = handle.clone();

        tokio::spawn(async move {
            let started = Instant::now();
            let mut core = Core::new(config);
            let mut sinks: Vec<Sink> = Vec::new();
            let mut ticker = tokio::time::interval(tick_interval);
            ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

            loop {
                let message = tokio::select! {
                    _ = ticker.tick() => Message::Input(Input::Tick),
                    received = receiver.recv() => match received {
                        Some(message) => message,
                        // Unreachable while the driver holds its own
                        // handle. Exiting is still the right response if
                        // that ever changes.
                        None => break,
                    },
                };

                let input = match message {
                    Message::Subscribe(sink) => {
                        if core.topics().iter().all(&sink) {
                            sinks.push(sink);
                        }
                        continue;
                    }
                    Message::Input(input) => input,
                };

                let at = Timestamp::from_millis(started.elapsed().as_millis() as u64);
                for effect in core.apply(input, at) {
                    match effect {
                        Effect::Emit(topic) => sinks.retain(|sink| sink(&topic)),
                        Effect::OpenConnection { connection, spec } => {
                            open(connection, spec, driver_handle.clone());
                        }
                        Effect::CloseConnection { connection } => {
                            tracing::warn!(
                                ?connection,
                                "close requested, but transports run for the process lifetime in this milestone"
                            );
                        }
                    }
                }
            }
        });

        handle
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_abs_diff_eq;
    use claims::{assert_some, assert_some_eq};
    use tokio::time::timeout;

    const RMC: &[u8] = b"$GPRMC,120000.00,A,5049.38,N,00611.16,E,45.0,270.0,010126,,,A\r\n";
    const LINK: ConnectionId = ConnectionId(1);
    const PATIENCE: Duration = Duration::from_secs(5);

    fn config() -> CoreConfig {
        CoreConfig {
            connections: vec![(LINK, ConnectionSpec::tcp("127.0.0.1", 4353))],
            ..CoreConfig::default()
        }
    }

    /// Subscribes and returns a receiver of every topic the driver emits.
    fn topic_stream(handle: &DriverHandle) -> mpsc::UnboundedReceiver<Topic> {
        let (sender, receiver) = mpsc::unbounded_channel();
        handle.subscribe(Box::new(move |topic: &Topic| {
            sender.send(topic.clone()).is_ok()
        }));
        receiver
    }

    /// Awaits topics until one carries a position, so the onboarding
    /// emission of empty state does not have to be counted.
    async fn next_position(receiver: &mut mpsc::UnboundedReceiver<Topic>) -> updraft_core::LatLon {
        loop {
            let received = timeout(PATIENCE, receiver.recv())
                .await
                .expect("a topic within the timeout");
            let Topic::Instruments(instruments) = assert_some!(received) else {
                continue;
            };
            if let Some(position) = instruments.position {
                return position;
            }
        }
    }

    #[tokio::test]
    async fn subscribing_delivers_current_state_immediately() {
        let handle = Driver::spawn(config(), Box::new(|_, _, _| {}), Duration::from_millis(100));
        let mut topics = topic_stream(&handle);

        let received = timeout(PATIENCE, topics.recv())
            .await
            .expect("onboarding topic within the timeout");

        assert_some_eq!(
            received,
            Topic::Instruments(updraft_core::Instruments::default())
        );
    }

    #[tokio::test]
    async fn decoded_fixes_reach_subscribers() {
        let handle = Driver::spawn(config(), Box::new(|_, _, _| {}), Duration::from_millis(100));
        let mut topics = topic_stream(&handle);

        handle.send(Input::bytes(LINK, RMC));

        let position = next_position(&mut topics).await;
        assert_abs_diff_eq!(position.latitude_degrees, 50.823, epsilon = 1e-3);
    }

    #[tokio::test]
    async fn start_asks_for_a_transport_per_configured_connection() {
        let (sender, mut receiver) = mpsc::unbounded_channel();
        let handle = Driver::spawn(
            config(),
            Box::new(move |connection, _spec, _handle| {
                let _ = sender.send(connection);
            }),
            Duration::from_millis(100),
        );

        handle.send(Input::Start);

        let requested = timeout(PATIENCE, receiver.recv())
            .await
            .expect("an open request within the timeout");
        assert_some_eq!(requested, LINK);
    }
}
