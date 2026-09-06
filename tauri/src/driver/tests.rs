use super::*;
use approx::assert_abs_diff_eq;
use claims::{assert_some, assert_some_eq};
use std::{
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};
use tokio::sync::{mpsc, oneshot};
use tokio::time::timeout;
use updraft_airspace::AirspaceDataset;
use updraft_core::{
    ActivateAirspaceDataset, AddExternalDevice, AirspaceLoadError, AirspaceState, AirspaceStatus,
    Bytes, ConnectionSpec, DeleteExternalDevice, EditExternalDevice, ExternalDeviceConfig,
    ExternalDeviceId, LatLon, PublishedExternalDevice, SetExternalDeviceEnabled, SetLocale,
    SettingsSnapshot, Topic, TrafficUpdate,
};

const RMC: &[u8] = b"$GPRMC,120000.00,A,5049.38,N,00611.16,E,45.0,270.0,010126,,,A\r\n";
const PATIENCE: Duration = Duration::from_secs(5);

pub struct TestDriver {
    pub handle: DriverHandle,
    task: tokio::task::JoinHandle<()>,
}

impl TestDriver {
    pub async fn terminate(self) {
        self.task.abort();
        let _ = self.task.await;
    }
}

pub fn spawn(
    snapshot: SettingsSnapshot,
    open: OpenFn,
    persist: PersistFn,
    tick_interval: Duration,
) -> TestDriver {
    let (handle, task) = Driver::spawn_task(
        snapshot,
        AirspaceState::none_at_startup(),
        open,
        persist,
        tick_interval,
    );
    TestDriver { handle, task }
}

fn no_airspace() -> AirspaceState {
    AirspaceState::none_at_startup()
}

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
async fn next_position(receiver: &mut mpsc::UnboundedReceiver<Topic>) -> LatLon {
    loop {
        let received = timeout(PATIENCE, receiver.recv())
            .await
            .expect("a topic within the timeout");
        let Topic::Instruments(instruments) = assert_some!(received) else {
            continue;
        };
        if let Some(gps) = instruments.gps {
            return gps.position;
        }
    }
}

async fn next_airspace_status(receiver: &mut mpsc::UnboundedReceiver<Topic>) -> AirspaceStatus {
    loop {
        let received = timeout(PATIENCE, receiver.recv())
            .await
            .expect("an airspace topic within the timeout");
        let Topic::Airspace(status) = assert_some!(received) else {
            continue;
        };
        return status;
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
    matches: impl Fn(&[PublishedExternalDevice]) -> bool,
) -> Vec<PublishedExternalDevice> {
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
        no_airspace(),
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
async fn new_subscriber_receives_current_airspace_status() {
    let handle = Driver::spawn(
        snapshot(),
        no_airspace(),
        Box::new(|_, _, _| Box::new(|| {})),
        Box::new(|_| {}),
        Duration::from_millis(100),
    );
    let dataset = Arc::new(AirspaceDataset::default());
    handle
        .send(ActivateAirspaceDataset::new(dataset, None))
        .await
        .expect("driver remains active");
    let mut topics = topic_stream(&handle);

    let status = next_airspace_status(&mut topics).await;

    assert_eq!(
        status,
        AirspaceStatus {
            generation: 1,
            sources: vec![updraft_core::AirspaceSourceStatus::Active {
                source_name: "airspace.txt".into(),
                airspace_count: 0
            }]
        }
    );
}

#[tokio::test]
async fn driver_starts_with_active_airspace_at_generation_zero() {
    let dataset = Arc::new(AirspaceDataset::default());
    let initial_airspace =
        AirspaceState::active_at_startup(dataset, Some("Stored airspace.txt".into()));
    let handle = Driver::spawn(
        snapshot(),
        initial_airspace,
        Box::new(|_, _, _| Box::new(|| {})),
        Box::new(|_| {}),
        Duration::from_millis(100),
    );
    let mut topics = topic_stream(&handle);

    assert_eq!(
        next_airspace_status(&mut topics).await,
        AirspaceStatus {
            generation: 0,
            sources: vec![updraft_core::AirspaceSourceStatus::Active {
                source_name: "Stored airspace.txt".into(),
                airspace_count: 0
            }]
        }
    );
}

#[tokio::test]
async fn driver_starts_with_unavailable_airspace() {
    let initial_airspace = AirspaceState::unavailable_at_startup(
        Some("Broken airspace.txt".into()),
        AirspaceLoadError::ParseFailed,
    );
    let handle = Driver::spawn(
        snapshot(),
        initial_airspace,
        Box::new(|_, _, _| Box::new(|| {})),
        Box::new(|_| {}),
        Duration::from_millis(100),
    );
    let mut topics = topic_stream(&handle);

    assert_eq!(
        next_airspace_status(&mut topics).await,
        AirspaceStatus {
            generation: 0,
            sources: vec![updraft_core::AirspaceSourceStatus::Unavailable {
                source_name: "Broken airspace.txt".into(),
                error: AirspaceLoadError::ParseFailed
            }]
        }
    );
}

#[tokio::test]
async fn subscription_includes_a_traffic_snapshot() {
    let handle = Driver::spawn(
        snapshot(),
        no_airspace(),
        Box::new(|_, _, _| Box::new(|| {})),
        Box::new(|_| {}),
        Duration::from_millis(100),
    );
    let mut topics = topic_stream(&handle);

    loop {
        let received = timeout(PATIENCE, topics.recv())
            .await
            .expect("an onboarding topic within the timeout")
            .expect("the driver remains active");
        if received == Topic::Traffic(TrafficUpdate::Snapshot(Vec::new())) {
            break;
        }
    }
}

#[tokio::test]
async fn locale_changes_reach_subscribers_and_persistence() {
    let (persisted_tx, mut persisted_rx) = mpsc::unbounded_channel();
    let handle = Driver::spawn(
        snapshot(),
        no_airspace(),
        Box::new(|_, _, _| Box::new(|| {})),
        Box::new(move |snapshot| {
            let _ = persisted_tx.send(snapshot);
        }),
        Duration::from_millis(100),
    );
    let mut topics = topic_stream(&handle);

    let input = SetLocale::new(updraft_core::Locale::De);
    assert_eq!(handle.send(input).await, Ok(()));

    let settings = loop {
        let topic = topics
            .try_recv()
            .expect("a topic emitted before the response");
        if let Topic::Settings(settings) = topic
            && settings.locale == Some(updraft_core::Locale::De)
        {
            break settings;
        }
    };

    assert_eq!(
        persisted_rx
            .try_recv()
            .expect("a persisted snapshot emitted before the response"),
        SettingsSnapshot {
            settings,
            external_devices: snapshot().external_devices,
        }
    );
}

#[tokio::test]
async fn decoded_fixes_reach_subscribers() {
    let handle = Driver::spawn(
        snapshot(),
        no_airspace(),
        Box::new(|_, _, _| Box::new(|| {})),
        Box::new(|_| {}),
        Duration::from_millis(100),
    );
    let mut topics = topic_stream(&handle);
    let device_id = next_device_id(&mut topics).await;

    let input = Bytes::new(device_id, RMC);
    handle.send(input).await.expect("driver remains active");

    let position = next_position(&mut topics).await;
    assert_abs_diff_eq!(position.latitude_degrees, 50.823, epsilon = 1e-3);
}

#[tokio::test]
async fn start_asks_for_a_transport_per_configured_connection() {
    let (sender, mut receiver) = mpsc::unbounded_channel();
    let handle = Driver::spawn(
        snapshot(),
        no_airspace(),
        Box::new(move |device_id, spec, _handle| {
            let _ = sender.send((device_id, spec));
            Box::new(|| {})
        }),
        Box::new(|_| {}),
        Duration::from_millis(100),
    );
    let mut topics = topic_stream(&handle);
    let expected_device_id = next_device_id(&mut topics).await;

    let requested = timeout(PATIENCE, receiver.recv())
        .await
        .expect("an open request within the timeout");
    assert_some_eq!(
        requested,
        (expected_device_id, ConnectionSpec::tcp("127.0.0.1", 4353))
    );
}

#[tokio::test]
async fn admitted_input_survives_a_dropped_response_receiver() {
    let (persisted_tx, mut persisted_rx) = mpsc::unbounded_channel();
    let handle = Driver::spawn(
        snapshot(),
        no_airspace(),
        Box::new(|_, _, _| Box::new(|| {})),
        Box::new(move |snapshot| {
            let _ = persisted_tx.send(snapshot);
        }),
        Duration::from_millis(100),
    );
    let (reply, response) = oneshot::channel();
    drop(response);

    handle
        .messages
        .send(Message::Input(Box::new(Request {
            input: SetLocale::new(updraft_core::Locale::De),
            reply,
        })))
        .expect("request is admitted");

    assert_eq!(
        timeout(PATIENCE, persisted_rx.recv())
            .await
            .expect("a persisted snapshot within the timeout"),
        Some(SettingsSnapshot {
            settings: updraft_core::Settings {
                locale: Some(updraft_core::Locale::De),
                ..updraft_core::Settings::default()
            },
            external_devices: snapshot().external_devices,
        })
    );
}

#[tokio::test]
async fn send_to_a_stopped_driver_fails() {
    let input = SetLocale::new(updraft_core::Locale::De);
    assert_eq!(
        inactive_driver_handle().send(input).await,
        Err(DriverStopped)
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
        no_airspace(),
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
    let input = AddExternalDevice::new(first_spec.clone());
    let device_id = handle.send(input).await.expect("driver remains active");
    let (opened_device_id, opened_spec) = timeout(PATIENCE, opened_rx.recv())
        .await
        .expect("an add open within the timeout")
        .expect("the driver remains active");
    assert_eq!(opened_device_id, device_id);
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
    let input = EditExternalDevice::new(device_id, edited_spec.clone());
    handle
        .send(input)
        .await
        .expect("driver remains active")
        .expect("external device edit succeeds");
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

    let input = SetExternalDeviceEnabled::disabled(device_id);
    handle
        .send(input)
        .await
        .expect("driver remains active")
        .expect("external device disable succeeds");
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

    let input = SetExternalDeviceEnabled::enabled(device_id);
    handle
        .send(input)
        .await
        .expect("driver remains active")
        .expect("external device enable succeeds");
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

    let input = DeleteExternalDevice::new(device_id);
    handle
        .send(input)
        .await
        .expect("driver remains active")
        .expect("external device deletion succeeds");
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
