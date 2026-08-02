use super::super::attempt::{AttemptResult, SppPlatform, run_attempt};
use crate::driver::{Driver, DriverHandle};
use claims::assert_some;
use std::assert_matches;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicUsize, Ordering},
};
use std::time::Duration;
use tauri::ipc::{Channel, InvokeResponseBody};
use tauri_plugin_updraft::SppConnectionId;
use tokio::sync::{mpsc, oneshot};
use tokio::time::timeout;
use updraft_core::{
    AirspaceState, ConnectionSpec, ExternalDeviceConfig, ExternalDeviceId,
    STANDARD_SPP_SERVICE_UUID, SettingsSnapshot, Topic,
};
use uuid::{Uuid, uuid};

pub const ADDRESS: &str = "00:11:22:33:44:55";
pub const SECOND_ADDRESS: &str = "00:11:22:33:44:66";
pub const CUSTOM_UUID: Uuid = uuid!("e56617bf-f548-4f7c-9cef-4a26eec19b04");
pub const DEVICE_ID: ExternalDeviceId = ExternalDeviceId(1);
pub const PATIENCE: Duration = Duration::from_secs(5);
pub const RMC_EVENT: &str = r#"{"type":"bytes","data":"JEdQUk1DLDEyMDAwMC4wMCxBLDUwNDkuMzgsTiwwMDYxMS4xNixFLDQ1LjAsMjcwLjAsMDEwMTI2LCwsQQ0K"}"#;
pub const SECOND_RMC_EVENT: &str = r#"{"type":"bytes","data":"JEdQUk1DLDEyMDAwMC4wMCxBLDUxNDkuMzgsTiwwMDYxMS4xNixFLDQ1LjAsMjcwLjAsMDEwMTI2LCwsQQ0K"}"#;

pub struct FakePlatform {
    events: Vec<&'static str>,
    start_error: Option<&'static str>,
    cancel_error: Option<&'static str>,
    attempts: AtomicUsize,
    connection_ids: Mutex<Vec<SppConnectionId>>,
    cancelled_ids: Mutex<Vec<SppConnectionId>>,
    channels: Mutex<Vec<Channel>>,
    service_uuids: Mutex<Vec<Uuid>>,
}

impl FakePlatform {
    pub fn with_events(events: Vec<&'static str>) -> Self {
        Self {
            events,
            start_error: None,
            cancel_error: None,
            attempts: AtomicUsize::new(0),
            connection_ids: Mutex::new(Vec::new()),
            cancelled_ids: Mutex::new(Vec::new()),
            channels: Mutex::new(Vec::new()),
            service_uuids: Mutex::new(Vec::new()),
        }
    }

    pub fn failing_with(reason: &'static str) -> Self {
        Self {
            events: Vec::new(),
            start_error: Some(reason),
            cancel_error: None,
            attempts: AtomicUsize::new(0),
            connection_ids: Mutex::new(Vec::new()),
            cancelled_ids: Mutex::new(Vec::new()),
            channels: Mutex::new(Vec::new()),
            service_uuids: Mutex::new(Vec::new()),
        }
    }

    pub fn with_cancel_error(reason: &'static str) -> Self {
        Self {
            cancel_error: Some(reason),
            ..Self::with_events(Vec::new())
        }
    }

    pub fn attempts(&self) -> usize {
        self.attempts.load(Ordering::SeqCst)
    }

    pub fn cancellations(&self) -> usize {
        self.cancelled_ids.lock().expect("cancelled IDs lock").len()
    }

    pub fn connection_ids(&self) -> Vec<SppConnectionId> {
        self.connection_ids
            .lock()
            .expect("connection IDs lock")
            .clone()
    }

    pub fn cancelled_ids(&self) -> Vec<SppConnectionId> {
        self.cancelled_ids
            .lock()
            .expect("cancelled IDs lock")
            .clone()
    }

    pub fn service_uuids(&self) -> Vec<Uuid> {
        self.service_uuids
            .lock()
            .expect("service UUIDs lock")
            .clone()
    }

    pub fn send(&self, payload: &str) {
        let index = self
            .channels
            .lock()
            .expect("channels lock")
            .len()
            .checked_sub(1)
            .expect("an active attempt channel");
        self.send_on(index, payload);
    }

    pub fn send_on(&self, index: usize, payload: &str) {
        let channel = self
            .channels
            .lock()
            .expect("channels lock")
            .get(index)
            .expect("an attempt channel at the selected index")
            .clone();
        channel
            .send(InvokeResponseBody::Json(payload.to_owned()))
            .expect("fake event reaches the channel");
    }
}

impl SppPlatform for FakePlatform {
    fn start_attempt(
        &self,
        address: &str,
        service_uuid: Uuid,
        events: Channel,
    ) -> Result<(), String> {
        assert_matches!(address, ADDRESS | SECOND_ADDRESS);
        self.service_uuids
            .lock()
            .expect("service UUIDs lock")
            .push(service_uuid);
        self.attempts.fetch_add(1, Ordering::SeqCst);
        self.connection_ids
            .lock()
            .expect("connection IDs lock")
            .push(SppConnectionId::from_channel(&events));
        self.channels
            .lock()
            .expect("channels lock")
            .push(events.clone());

        if let Some(reason) = self.start_error {
            return Err(reason.to_owned());
        }

        for payload in &self.events {
            events
                .send(InvokeResponseBody::Json((*payload).to_owned()))
                .expect("fake event reaches the channel");
        }
        Ok(())
    }

    fn cancel_attempt(&self, connection_id: SppConnectionId) -> Result<(), String> {
        self.cancelled_ids
            .lock()
            .expect("cancelled IDs lock")
            .push(connection_id);
        self.cancel_error
            .map_or(Ok(()), |reason| Err(reason.to_owned()))
    }
}

pub fn event_stream() -> (Channel, mpsc::UnboundedReceiver<InvokeResponseBody>) {
    let (sender, receiver) = mpsc::unbounded_channel();
    let events = Channel::new(move |body| {
        let _ = sender.send(body);
        Ok(())
    });
    (events, receiver)
}

pub fn spawn_attempt(
    platform: Arc<FakePlatform>,
    handle: DriverHandle,
) -> tokio::task::JoinHandle<AttemptResult> {
    let (events, mut receiver) = event_stream();
    let (stop_sender, stop_receiver) = oneshot::channel();
    tokio::spawn(async move {
        let _stop_sender = stop_sender;
        tokio::pin!(stop_receiver);
        run_attempt(
            DEVICE_ID,
            ADDRESS,
            STANDARD_SPP_SERVICE_UUID,
            &handle,
            platform.as_ref(),
            &events,
            &mut receiver,
            stop_receiver.as_mut(),
        )
        .await
    })
}

pub fn driver() -> DriverHandle {
    driver_with_spp_addresses(&[ADDRESS])
}

pub fn driver_with_spp_addresses(addresses: &[&str]) -> DriverHandle {
    Driver::spawn(
        SettingsSnapshot {
            settings: Default::default(),
            external_devices: addresses
                .iter()
                .map(|address| ExternalDeviceConfig {
                    enabled: true,
                    spec: ConnectionSpec::bluetooth_spp(*address),
                })
                .collect(),
        },
        AirspaceState::none_at_startup(),
        Box::new(|_, _, _| Box::new(|| {})),
        Box::new(|_| {}),
        Duration::from_secs(60),
    )
}

pub fn topic_stream(handle: &DriverHandle) -> mpsc::UnboundedReceiver<Topic> {
    let (sender, receiver) = mpsc::unbounded_channel();
    handle.subscribe(Box::new(move |topic: &Topic| {
        sender.send(topic.clone()).is_ok()
    }));
    receiver
}

pub async fn next_position(receiver: &mut mpsc::UnboundedReceiver<Topic>) -> updraft_core::LatLon {
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

pub async fn current_instruments(handle: &DriverHandle) -> updraft_core::Instruments {
    let mut topics = topic_stream(handle);
    loop {
        let received = timeout(PATIENCE, topics.recv())
            .await
            .expect("current instruments within the timeout");
        let Topic::Instruments(instruments) = assert_some!(received) else {
            continue;
        };
        return instruments;
    }
}

pub fn warning_context(lines: &[&str], message: &str, reason: &str) -> Result<(), String> {
    let Some(line) = lines.iter().find(|line| line.contains(message)) else {
        return Err(format!("missing {message:?} warning"));
    };
    let missing: Vec<_> = [
        format!(" device_id={DEVICE_ID:?}"),
        format!(" address={ADDRESS}"),
        " reason=".to_owned(),
        reason.to_owned(),
    ]
    .into_iter()
    .filter(|field| !line.contains(field))
    .collect();

    if missing.is_empty() {
        Ok(())
    } else {
        Err(format!("warning lacks {missing:?}: {line:?}"))
    }
}
