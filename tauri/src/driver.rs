use std::{
    collections::BTreeMap,
    time::{Duration, Instant},
};
use tokio::sync::mpsc;
use updraft_core::{
    ConnectionSpec, Core, Effect, ExternalDeviceId, Input, SettingsSnapshot, Timestamp, Topic,
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
    pub fn spawn(
        snapshot: SettingsSnapshot,
        open: OpenFn,
        persist: PersistFn,
        tick_interval: Duration,
    ) -> DriverHandle {
        let (sender, mut receiver) = mpsc::unbounded_channel();
        let handle = DriverHandle { messages: sender };
        let driver_handle = handle.clone();

        tokio::spawn(async move {
            let started = Instant::now();
            let mut core = Core::new(snapshot);
            let mut sinks: Vec<Sink> = Vec::new();
            let mut transports = ActiveTransports::default();
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
                        Effect::PersistSettings(snapshot) => persist(snapshot),
                        Effect::OpenConnection { device_id, spec } => {
                            transports.open(device_id, spec, driver_handle.clone(), &open);
                        }
                        Effect::CloseConnection { device_id } => {
                            transports.close(device_id);
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
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    use tokio::time::timeout;
    use updraft_core::ExternalDeviceConfig;

    const RMC: &[u8] = b"$GPRMC,120000.00,A,5049.38,N,00611.16,E,45.0,270.0,010126,,,A\r\n";
    const PATIENCE: Duration = Duration::from_secs(5);

    fn snapshot() -> SettingsSnapshot {
        SettingsSnapshot {
            settings: Default::default(),
            external_devices: vec![ExternalDeviceConfig {
                enabled: true,
                spec: ConnectionSpec::tcp("127.0.0.1", 4353),
            }],
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

    async fn next_device_id(receiver: &mut mpsc::UnboundedReceiver<Topic>) -> ExternalDeviceId {
        loop {
            let received = timeout(PATIENCE, receiver.recv())
                .await
                .expect("a topic within the timeout");
            let Topic::ExternalDevices(devices) = assert_some!(received) else {
                continue;
            };
            return devices[0].device_id;
        }
    }

    async fn next_external_devices(
        receiver: &mut mpsc::UnboundedReceiver<Topic>,
        matches: impl Fn(&[updraft_core::PublishedExternalDevice]) -> bool,
    ) -> Vec<updraft_core::PublishedExternalDevice> {
        loop {
            let received = timeout(PATIENCE, receiver.recv())
                .await
                .expect("an external devices topic within the timeout");
            let Topic::ExternalDevices(devices) = assert_some!(received) else {
                continue;
            };
            if matches(&devices) {
                return devices;
            }
        }
    }

    async fn next_persisted_snapshot(
        receiver: &mut mpsc::UnboundedReceiver<SettingsSnapshot>,
    ) -> SettingsSnapshot {
        timeout(PATIENCE, receiver.recv())
            .await
            .expect("a persisted snapshot within the timeout")
            .expect("the driver remains active")
    }

    fn inactive_driver_handle() -> DriverHandle {
        let (messages, _) = mpsc::unbounded_channel();
        DriverHandle { messages }
    }

    #[tokio::test]
    async fn subscribing_delivers_current_state_immediately() {
        let handle = Driver::spawn(
            snapshot(),
            Box::new(|_, _, _| Box::new(|| {})),
            Box::new(|_| {}),
            Duration::from_millis(100),
        );
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
    async fn locale_changes_reach_subscribers_and_persistence() {
        let (persisted_tx, mut persisted_rx) = mpsc::unbounded_channel();
        let handle = Driver::spawn(
            snapshot(),
            Box::new(|_, _, _| Box::new(|| {})),
            Box::new(move |snapshot| {
                let _ = persisted_tx.send(snapshot);
            }),
            Duration::from_millis(100),
        );
        let mut topics = topic_stream(&handle);

        handle.send(Input::SetLocale(updraft_core::Locale::De));

        let settings = loop {
            let topic = timeout(PATIENCE, topics.recv())
                .await
                .expect("a settings topic within the timeout")
                .expect("the driver remains active");
            if let Topic::Settings(settings) = topic
                && settings.locale == Some(updraft_core::Locale::De)
            {
                break settings;
            }
        };

        assert_eq!(
            timeout(PATIENCE, persisted_rx.recv())
                .await
                .expect("a persisted snapshot within the timeout"),
            Some(SettingsSnapshot {
                settings,
                external_devices: snapshot().external_devices,
            })
        );
    }

    #[tokio::test]
    async fn decoded_fixes_reach_subscribers() {
        let handle = Driver::spawn(
            snapshot(),
            Box::new(|_, _, _| Box::new(|| {})),
            Box::new(|_| {}),
            Duration::from_millis(100),
        );
        let mut topics = topic_stream(&handle);
        let device_id = next_device_id(&mut topics).await;

        handle.send(Input::bytes(device_id, RMC));

        let position = next_position(&mut topics).await;
        assert_abs_diff_eq!(position.latitude_degrees, 50.823, epsilon = 1e-3);
    }

    #[tokio::test]
    async fn start_asks_for_a_transport_per_configured_connection() {
        let (sender, mut receiver) = mpsc::unbounded_channel();
        let handle = Driver::spawn(
            snapshot(),
            Box::new(move |device_id, spec, _handle| {
                let _ = sender.send((device_id, spec));
                Box::new(|| {})
            }),
            Box::new(|_| {}),
            Duration::from_millis(100),
        );
        let mut topics = topic_stream(&handle);
        let expected_device_id = next_device_id(&mut topics).await;

        handle.send(Input::Start);

        let requested = timeout(PATIENCE, receiver.recv())
            .await
            .expect("an open request within the timeout");
        assert_some_eq!(
            requested,
            (expected_device_id, ConnectionSpec::tcp("127.0.0.1", 4353))
        );
    }

    #[tokio::test]
    async fn external_device_mutations_drive_one_worker_and_complete_snapshots() {
        let opens = Arc::new(AtomicUsize::new(0));
        let stops = Arc::new(AtomicUsize::new(0));
        let (opened_tx, mut opened_rx) = mpsc::unbounded_channel();
        let (persisted_tx, mut persisted_rx) = mpsc::unbounded_channel();
        let open_count = opens.clone();
        let stop_count = stops.clone();
        let handle = Driver::spawn(
            SettingsSnapshot::default(),
            Box::new(move |device_id, spec, _handle| {
                open_count.fetch_add(1, Ordering::SeqCst);
                let _ = opened_tx.send((device_id, spec));
                let stops = stop_count.clone();
                Box::new(move || {
                    stops.fetch_add(1, Ordering::SeqCst);
                })
            }),
            Box::new(move |snapshot| {
                let _ = persisted_tx.send(snapshot);
            }),
            Duration::from_millis(100),
        );
        let mut topics = topic_stream(&handle);

        let first_spec = ConnectionSpec::tcp("127.0.0.1", 4353);
        handle.send(Input::AddExternalDevice {
            spec: first_spec.clone(),
        });
        let (device_id, opened_spec) = timeout(PATIENCE, opened_rx.recv())
            .await
            .expect("an add open within the timeout")
            .expect("the driver remains active");
        assert_eq!(opened_spec, first_spec);
        let published = next_external_devices(&mut topics, |devices| {
            devices.len() == 1 && devices[0].config.spec == first_spec
        })
        .await;
        assert_eq!(published[0].device_id, device_id);
        assert_eq!(
            next_persisted_snapshot(&mut persisted_rx).await,
            SettingsSnapshot {
                settings: Default::default(),
                external_devices: vec![published[0].config.clone()],
            }
        );
        assert_eq!(opens.load(Ordering::SeqCst), 1);
        assert_eq!(stops.load(Ordering::SeqCst), 0);

        let edited_spec = ConnectionSpec::bluetooth_spp("00:11:22:33:44:55");
        handle.send(Input::EditExternalDevice {
            device_id,
            spec: edited_spec.clone(),
        });
        let (edited_device_id, opened_spec) = timeout(PATIENCE, opened_rx.recv())
            .await
            .expect("an edit open within the timeout")
            .expect("the driver remains active");
        assert_eq!(edited_device_id, device_id);
        assert_eq!(opened_spec, edited_spec);
        let published = next_external_devices(&mut topics, |devices| {
            devices.len() == 1 && devices[0].config.spec == edited_spec
        })
        .await;
        assert_eq!(published[0].device_id, device_id);
        assert_eq!(
            next_persisted_snapshot(&mut persisted_rx).await,
            SettingsSnapshot {
                settings: Default::default(),
                external_devices: vec![published[0].config.clone()],
            }
        );
        assert_eq!(opens.load(Ordering::SeqCst), 2);
        assert_eq!(stops.load(Ordering::SeqCst), 1);

        handle.send(Input::SetExternalDeviceEnabled {
            device_id,
            enabled: false,
        });
        let published = next_external_devices(&mut topics, |devices| {
            devices.len() == 1 && !devices[0].config.enabled
        })
        .await;
        assert_eq!(published[0].device_id, device_id);
        assert_eq!(
            next_persisted_snapshot(&mut persisted_rx).await,
            SettingsSnapshot {
                settings: Default::default(),
                external_devices: vec![published[0].config.clone()],
            }
        );
        assert_eq!(opens.load(Ordering::SeqCst), 2);
        assert_eq!(stops.load(Ordering::SeqCst), 2);

        handle.send(Input::SetExternalDeviceEnabled {
            device_id,
            enabled: true,
        });
        let (enabled_device_id, opened_spec) = timeout(PATIENCE, opened_rx.recv())
            .await
            .expect("an enable open within the timeout")
            .expect("the driver remains active");
        assert_eq!(enabled_device_id, device_id);
        assert_eq!(opened_spec, edited_spec);
        let published = next_external_devices(&mut topics, |devices| {
            devices.len() == 1 && devices[0].config.enabled
        })
        .await;
        assert_eq!(published[0].device_id, device_id);
        assert_eq!(
            next_persisted_snapshot(&mut persisted_rx).await,
            SettingsSnapshot {
                settings: Default::default(),
                external_devices: vec![published[0].config.clone()],
            }
        );
        assert_eq!(opens.load(Ordering::SeqCst), 3);
        assert_eq!(stops.load(Ordering::SeqCst), 2);

        handle.send(Input::DeleteExternalDevice(device_id));
        let published = next_external_devices(&mut topics, |devices| devices.is_empty()).await;
        assert!(published.is_empty());
        assert_eq!(
            next_persisted_snapshot(&mut persisted_rx).await,
            SettingsSnapshot::default()
        );
        assert_eq!(opens.load(Ordering::SeqCst), 3);
        assert_eq!(stops.load(Ordering::SeqCst), 3);
    }

    #[test]
    fn active_transports_opening_the_same_id_stops_and_replaces_its_worker() {
        let first_stops = Arc::new(AtomicUsize::new(0));
        let second_stops = Arc::new(AtomicUsize::new(0));
        let next_stops = Arc::new(std::sync::Mutex::new(vec![
            first_stops.clone(),
            second_stops.clone(),
        ]));
        let open: OpenFn = Box::new(move |_, _, _| {
            let stops = next_stops.lock().expect("stop counters lock").remove(0);
            Box::new(move || {
                stops.fetch_add(1, Ordering::SeqCst);
            })
        });
        let mut transports = ActiveTransports::default();
        let device_id = ExternalDeviceId(1);
        let spec = ConnectionSpec::tcp("127.0.0.1", 4353);
        let handle = inactive_driver_handle();

        transports.open(device_id, spec.clone(), handle.clone(), &open);
        transports.open(device_id, spec, handle, &open);

        assert_eq!(first_stops.load(Ordering::SeqCst), 1);
        assert_eq!(second_stops.load(Ordering::SeqCst), 0);

        transports.close(device_id);

        assert_eq!(first_stops.load(Ordering::SeqCst), 1);
        assert_eq!(second_stops.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn active_transports_closing_an_active_id_stops_and_removes_its_worker() {
        let stops = Arc::new(AtomicUsize::new(0));
        let open_stops = stops.clone();
        let open: OpenFn = Box::new(move |_, _, _| {
            let stops = open_stops.clone();
            Box::new(move || {
                stops.fetch_add(1, Ordering::SeqCst);
            })
        });
        let mut transports = ActiveTransports::default();
        let device_id = ExternalDeviceId(1);
        let handle = inactive_driver_handle();

        transports.open(
            device_id,
            ConnectionSpec::tcp("127.0.0.1", 4353),
            handle,
            &open,
        );
        transports.close(device_id);
        transports.close(device_id);

        assert_eq!(stops.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn active_transports_closing_an_unknown_id_is_a_no_op() {
        let stops = Arc::new(AtomicUsize::new(0));
        let open_stops = stops.clone();
        let open: OpenFn = Box::new(move |_, _, _| {
            let stops = open_stops.clone();
            Box::new(move || {
                stops.fetch_add(1, Ordering::SeqCst);
            })
        });
        let mut transports = ActiveTransports::default();
        let device_id = ExternalDeviceId(1);

        transports.open(
            device_id,
            ConnectionSpec::tcp("127.0.0.1", 4353),
            inactive_driver_handle(),
            &open,
        );
        transports.close(ExternalDeviceId(99));
        assert_eq!(stops.load(Ordering::SeqCst), 0);

        transports.close(device_id);
        assert_eq!(stops.load(Ordering::SeqCst), 1);
    }
}
