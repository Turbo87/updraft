use super::reconnect::ReconnectBackoff;
use crate::driver::{DriverHandle, StopFn};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use std::{pin::Pin, sync::Arc};
use tauri::ipc::{Channel, InvokeResponseBody};
#[cfg(target_os = "android")]
use tauri::{AppHandle, Runtime};
#[cfg(target_os = "android")]
use tauri_plugin_updraft::UpdraftMobileExt;
use tauri_plugin_updraft::{SppConnectionId, SppEvent};
use tokio::sync::{mpsc, oneshot};
use updraft_core::{Bytes, ConnectionChanged, ConnectionState, ExternalDeviceId};
use uuid::Uuid;

trait SppPlatform: Send + Sync + 'static {
    fn start_attempt(
        &self,
        address: &str,
        service_uuid: Uuid,
        events: Channel,
    ) -> Result<(), String>;
    fn cancel_attempt(&self, connection_id: SppConnectionId) -> Result<(), String>;
}

#[cfg(target_os = "android")]
struct AndroidSppPlatform<R: Runtime>(AppHandle<R>);

#[cfg(target_os = "android")]
impl<R: Runtime> SppPlatform for AndroidSppPlatform<R> {
    fn start_attempt(
        &self,
        address: &str,
        service_uuid: Uuid,
        events: Channel,
    ) -> Result<(), String> {
        self.0
            .updraft_mobile()
            .start_spp_attempt(address, service_uuid, events)
            .map_err(|error| error.to_string())
    }

    fn cancel_attempt(&self, connection_id: SppConnectionId) -> Result<(), String> {
        self.0
            .updraft_mobile()
            .cancel_spp_attempt(connection_id)
            .map_err(|error| error.to_string())
    }
}

#[cfg(target_os = "android")]
pub fn run<R: Runtime>(
    device_id: ExternalDeviceId,
    address: String,
    service_uuid: Uuid,
    handle: DriverHandle,
    app: AppHandle<R>,
) -> StopFn {
    let Maintained { stop, task: _task } = spawn_maintained(
        device_id,
        address,
        service_uuid,
        handle,
        Arc::new(AndroidSppPlatform(app)),
    );
    stop
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AttemptResult {
    Completed { delivered_bytes: bool },
    EventStreamClosed,
    Stopped,
    DriverStopped,
}

#[expect(clippy::too_many_arguments)]
async fn run_attempt(
    device_id: ExternalDeviceId,
    address: &str,
    service_uuid: Uuid,
    handle: &DriverHandle,
    platform: &dyn SppPlatform,
    events: &Channel,
    receiver: &mut mpsc::UnboundedReceiver<InvokeResponseBody>,
    mut stop_receiver: Pin<&mut oneshot::Receiver<()>>,
) -> AttemptResult {
    let connection_id = SppConnectionId::from_channel(events);
    match stop_receiver.as_mut().get_mut().try_recv() {
        Ok(()) | Err(oneshot::error::TryRecvError::Closed) => return AttemptResult::Stopped,
        Err(oneshot::error::TryRecvError::Empty) => {}
    }

    let input = ConnectionChanged::new(device_id, ConnectionState::Connecting);
    if handle.send(input).await.is_err() {
        return AttemptResult::DriverStopped;
    }

    let result = match platform.start_attempt(address, service_uuid, events.clone()) {
        Err(reason) => {
            tracing::warn!(
                ?device_id,
                %address,
                %reason,
                "SPP attempt failed to start"
            );
            AttemptResult::Completed {
                delivered_bytes: false,
            }
        }
        Ok(()) => {
            let mut delivered_bytes = false;
            let mut cancelling = false;
            let mut stopping = false;
            loop {
                let body = if stopping {
                    receiver.recv().await
                } else {
                    tokio::select! {
                        biased;
                        _ = stop_receiver.as_mut() => {
                            if !cancelling {
                                cancel_attempt(device_id, address, connection_id, platform);
                                cancelling = true;
                            }
                            stopping = true;
                            continue;
                        }
                        body = receiver.recv() => body,
                    }
                };
                let Some(body) = body else {
                    tracing::warn!(
                        ?device_id,
                        %address,
                        reason = "channel closed",
                        "SPP event channel closed"
                    );
                    break AttemptResult::EventStreamClosed;
                };
                let event = match body.deserialize::<SppEvent>() {
                    Ok(event) => event,
                    Err(_) if cancelling => continue,
                    Err(_) => {
                        tracing::warn!(
                            ?device_id,
                            %address,
                            reason = "malformed channel data",
                            "Malformed SPP event"
                        );
                        cancel_attempt(device_id, address, connection_id, platform);
                        cancelling = true;
                        continue;
                    }
                };

                if cancelling {
                    if let SppEvent::Disconnected { error } = event {
                        if let Some(reason) = error
                            && !stopping
                        {
                            tracing::warn!(
                                ?device_id,
                                %address,
                                %reason,
                                "SPP attempt disconnected"
                            );
                        }
                        break if stopping {
                            AttemptResult::Stopped
                        } else {
                            AttemptResult::Completed { delivered_bytes }
                        };
                    }
                    continue;
                }

                match event {
                    SppEvent::Connected => {
                        let input = ConnectionChanged::new(device_id, ConnectionState::Connected);
                        if handle.send(input).await.is_err() {
                            cancel_attempt(device_id, address, connection_id, platform);
                            return AttemptResult::DriverStopped;
                        }
                    }
                    SppEvent::Bytes { data } => match STANDARD.decode(data) {
                        Ok(bytes) => {
                            delivered_bytes |= !bytes.is_empty();
                            let input = Bytes::new(device_id, bytes);
                            if handle.send(input).await.is_err() {
                                cancel_attempt(device_id, address, connection_id, platform);
                                return AttemptResult::DriverStopped;
                            }
                        }
                        Err(_) => {
                            tracing::warn!(
                                ?device_id,
                                %address,
                                reason = "invalid Base64 data",
                                "Invalid Base64 SPP bytes"
                            );
                            cancel_attempt(device_id, address, connection_id, platform);
                            cancelling = true;
                        }
                    },
                    SppEvent::Disconnected { error } => {
                        if let Some(reason) = error {
                            tracing::warn!(
                                ?device_id,
                                %address,
                                %reason,
                                "SPP attempt disconnected"
                            );
                        }
                        break AttemptResult::Completed { delivered_bytes };
                    }
                }
            }
        }
    };

    let input = ConnectionChanged::new(device_id, ConnectionState::Disconnected);
    if handle.send(input).await.is_err() {
        return AttemptResult::DriverStopped;
    }
    result
}

fn cancel_attempt(
    device_id: ExternalDeviceId,
    address: &str,
    connection_id: SppConnectionId,
    platform: &dyn SppPlatform,
) {
    if let Err(reason) = platform.cancel_attempt(connection_id) {
        tracing::warn!(
            ?device_id,
            %address,
            %reason,
            "SPP attempt cancellation failed"
        );
    }
}

async fn maintain(
    device_id: ExternalDeviceId,
    address: String,
    service_uuid: Uuid,
    handle: DriverHandle,
    platform: Arc<dyn SppPlatform>,
    stop_receiver: oneshot::Receiver<()>,
) {
    let (sender, receiver) = mpsc::unbounded_channel::<InvokeResponseBody>();
    let events = Channel::new(move |body| {
        let _ = sender.send(body);
        Ok(())
    });
    tokio::pin!(stop_receiver);
    maintain_on_channel(
        device_id,
        address,
        service_uuid,
        handle,
        platform,
        events,
        receiver,
        stop_receiver.as_mut(),
    )
    .await;
}

#[expect(clippy::too_many_arguments)]
async fn maintain_on_channel(
    device_id: ExternalDeviceId,
    address: String,
    service_uuid: Uuid,
    handle: DriverHandle,
    platform: Arc<dyn SppPlatform>,
    events: Channel,
    mut receiver: mpsc::UnboundedReceiver<InvokeResponseBody>,
    mut stop_receiver: Pin<&mut oneshot::Receiver<()>>,
) {
    let mut backoff = ReconnectBackoff::default();

    loop {
        match run_attempt(
            device_id,
            &address,
            service_uuid,
            &handle,
            platform.as_ref(),
            &events,
            &mut receiver,
            stop_receiver.as_mut(),
        )
        .await
        {
            AttemptResult::Completed { delivered_bytes } => {
                tokio::select! {
                    biased;
                    _ = stop_receiver.as_mut() => return,
                    _ = tokio::time::sleep(backoff.after_attempt(delivered_bytes)) => {}
                }
            }
            AttemptResult::EventStreamClosed
            | AttemptResult::Stopped
            | AttemptResult::DriverStopped => return,
        }
    }
}

struct Maintained {
    stop: StopFn,
    task: tokio::task::JoinHandle<()>,
}

fn spawn_maintained(
    device_id: ExternalDeviceId,
    address: String,
    service_uuid: Uuid,
    handle: DriverHandle,
    platform: Arc<dyn SppPlatform>,
) -> Maintained {
    let (stop_sender, stop_receiver) = oneshot::channel();
    let task = tokio::spawn(maintain(
        device_id,
        address,
        service_uuid,
        handle,
        platform,
        stop_receiver,
    ));
    Maintained {
        stop: Box::new(move || {
            let _ = stop_sender.send(());
        }),
        task,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        AttemptResult, SppConnectionId, SppPlatform, maintain, maintain_on_channel, run_attempt,
        spawn_maintained,
    };
    use crate::driver::{Driver, DriverHandle, test_support};
    use claims::assert_some;
    use std::assert_matches;
    use std::sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    };
    use std::time::Duration;
    use tauri::ipc::{Channel, InvokeResponseBody};
    use tokio::sync::{mpsc, oneshot};
    use tokio::time::timeout;
    use tracing_test::traced_test;
    use updraft_core::{
        ConnectionSpec, ExternalDeviceConfig, ExternalDeviceId, STANDARD_SPP_SERVICE_UUID,
        SettingsSnapshot, Topic,
    };
    use uuid::{Uuid, uuid};

    const ADDRESS: &str = "00:11:22:33:44:55";
    const SECOND_ADDRESS: &str = "00:11:22:33:44:66";
    const CUSTOM_UUID: Uuid = uuid!("e56617bf-f548-4f7c-9cef-4a26eec19b04");
    const DEVICE_ID: ExternalDeviceId = ExternalDeviceId(1);
    const PATIENCE: Duration = Duration::from_secs(5);
    const RMC_EVENT: &str = r#"{"type":"bytes","data":"JEdQUk1DLDEyMDAwMC4wMCxBLDUwNDkuMzgsTiwwMDYxMS4xNixFLDQ1LjAsMjcwLjAsMDEwMTI2LCwsQQ0K"}"#;
    const SECOND_RMC_EVENT: &str = r#"{"type":"bytes","data":"JEdQUk1DLDEyMDAwMC4wMCxBLDUxNDkuMzgsTiwwMDYxMS4xNixFLDQ1LjAsMjcwLjAsMDEwMTI2LCwsQQ0K"}"#;

    struct FakePlatform {
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
        fn with_events(events: Vec<&'static str>) -> Self {
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

        fn failing_with(reason: &'static str) -> Self {
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

        fn with_cancel_error(reason: &'static str) -> Self {
            Self {
                cancel_error: Some(reason),
                ..Self::with_events(Vec::new())
            }
        }

        fn attempts(&self) -> usize {
            self.attempts.load(Ordering::SeqCst)
        }

        fn cancellations(&self) -> usize {
            self.cancelled_ids.lock().expect("cancelled IDs lock").len()
        }

        fn connection_ids(&self) -> Vec<SppConnectionId> {
            self.connection_ids
                .lock()
                .expect("connection IDs lock")
                .clone()
        }

        fn cancelled_ids(&self) -> Vec<SppConnectionId> {
            self.cancelled_ids
                .lock()
                .expect("cancelled IDs lock")
                .clone()
        }

        fn service_uuids(&self) -> Vec<Uuid> {
            self.service_uuids
                .lock()
                .expect("service UUIDs lock")
                .clone()
        }

        fn send(&self, payload: &str) {
            let index = self
                .channels
                .lock()
                .expect("channels lock")
                .len()
                .checked_sub(1)
                .expect("an active attempt channel");
            self.send_on(index, payload);
        }

        fn send_on(&self, index: usize, payload: &str) {
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

    fn event_stream() -> (Channel, mpsc::UnboundedReceiver<InvokeResponseBody>) {
        let (sender, receiver) = mpsc::unbounded_channel();
        let events = Channel::new(move |body| {
            let _ = sender.send(body);
            Ok(())
        });
        (events, receiver)
    }

    fn spawn_attempt(
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

    fn driver() -> DriverHandle {
        driver_with_spp_addresses(&[ADDRESS])
    }

    fn driver_with_spp_addresses(addresses: &[&str]) -> DriverHandle {
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
            Box::new(|_, _, _| Box::new(|| {})),
            Box::new(|_| {}),
            Duration::from_secs(60),
        )
    }

    fn topic_stream(handle: &DriverHandle) -> mpsc::UnboundedReceiver<Topic> {
        let (sender, receiver) = mpsc::unbounded_channel();
        handle.subscribe(Box::new(move |topic: &Topic| {
            sender.send(topic.clone()).is_ok()
        }));
        receiver
    }

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

    async fn current_instruments(handle: &DriverHandle) -> updraft_core::Instruments {
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

    fn warning_context(lines: &[&str], message: &str, reason: &str) -> Result<(), String> {
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

    #[tokio::test]
    async fn connected_bytes_reach_the_existing_nmea_path() {
        let platform = FakePlatform::with_events(vec![
            r#"{"type":"connected"}"#,
            RMC_EVENT,
            r#"{"type":"disconnected"}"#,
        ]);
        let handle = driver();
        let mut topics = topic_stream(&handle);
        let (events, mut receiver) = event_stream();
        let (_stop_sender, stop_receiver) = oneshot::channel();
        tokio::pin!(stop_receiver);

        let device_id = loop {
            let received = timeout(PATIENCE, topics.recv())
                .await
                .expect("a topic within the timeout");
            let Topic::ExternalDevices(devices) = assert_some!(received) else {
                continue;
            };
            break devices[0].device_id;
        };

        let result = run_attempt(
            device_id,
            ADDRESS,
            STANDARD_SPP_SERVICE_UUID,
            &handle,
            &platform,
            &events,
            &mut receiver,
            stop_receiver.as_mut(),
        )
        .await;

        assert_eq!(
            result,
            AttemptResult::Completed {
                delivered_bytes: true
            }
        );
        next_position(&mut topics).await;
    }

    #[tokio::test]
    async fn attempt_passes_the_service_uuid_to_the_platform() {
        let platform = FakePlatform::with_events(vec![r#"{"type":"disconnected"}"#]);
        let (events, mut receiver) = event_stream();
        let (_stop_sender, stop_receiver) = oneshot::channel();
        tokio::pin!(stop_receiver);

        run_attempt(
            DEVICE_ID,
            ADDRESS,
            CUSTOM_UUID,
            &driver(),
            &platform,
            &events,
            &mut receiver,
            stop_receiver.as_mut(),
        )
        .await;

        assert_eq!(platform.service_uuids(), vec![CUSTOM_UUID]);
    }

    #[tokio::test]
    #[traced_test]
    async fn malformed_event_cancels_and_waits_for_the_terminal_event() {
        let platform = Arc::new(FakePlatform::with_events(Vec::new()));
        let handle = driver();
        let attempt = spawn_attempt(platform.clone(), handle.clone());
        tokio::task::yield_now().await;

        platform.send(r#"{"type":"secret-payload","data":"do-not-log"}"#);
        tokio::task::yield_now().await;

        assert_eq!(platform.cancellations(), 1);
        assert!(!attempt.is_finished());

        platform.send(r#"{"type":"connected"}"#);
        platform.send(RMC_EVENT);
        tokio::task::yield_now().await;

        assert_eq!(platform.cancellations(), 1);
        assert!(!attempt.is_finished());

        platform.send(r#"{"type":"disconnected"}"#);
        let result = timeout(PATIENCE, attempt)
            .await
            .expect("attempt completes after its terminal event")
            .expect("attempt task succeeds");

        assert_eq!(
            result,
            AttemptResult::Completed {
                delivered_bytes: false
            }
        );
        assert!(current_instruments(&handle).await.position.is_none());
        logs_assert(|lines| {
            warning_context(lines, "Malformed SPP event", "malformed channel data")
        });
        assert!(!logs_contain("do-not-log"));
        assert!(!logs_contain("Connected"));
    }

    #[tokio::test]
    #[traced_test]
    async fn invalid_base64_cancels_and_waits_for_the_terminal_event() {
        let platform = Arc::new(FakePlatform::with_events(Vec::new()));
        let handle = driver();
        let attempt = spawn_attempt(platform.clone(), handle.clone());
        tokio::task::yield_now().await;

        platform.send(r#"{"type":"bytes","data":"do-not-log!"}"#);
        tokio::task::yield_now().await;

        assert_eq!(platform.cancellations(), 1);
        assert!(!attempt.is_finished());

        platform.send(RMC_EVENT);
        tokio::task::yield_now().await;

        assert_eq!(platform.cancellations(), 1);
        assert!(!attempt.is_finished());

        platform.send(r#"{"type":"disconnected"}"#);
        let result = timeout(PATIENCE, attempt)
            .await
            .expect("attempt completes after its terminal event")
            .expect("attempt task succeeds");

        assert_eq!(
            result,
            AttemptResult::Completed {
                delivered_bytes: false
            }
        );
        assert!(current_instruments(&handle).await.position.is_none());
        logs_assert(|lines| {
            warning_context(lines, "Invalid Base64 SPP bytes", "invalid Base64 data")
        });
        assert!(!logs_contain("do-not-log"));
    }

    #[tokio::test(start_paused = true)]
    async fn terminal_event_reconnects_after_the_current_delay_on_the_same_channel() {
        let platform = Arc::new(FakePlatform::with_events(vec![
            r#"{"type":"connected"}"#,
            r#"{"type":"disconnected"}"#,
        ]));
        let (_stop_sender, stop_receiver) = oneshot::channel();
        let task = tokio::spawn(maintain(
            DEVICE_ID,
            ADDRESS.to_owned(),
            STANDARD_SPP_SERVICE_UUID,
            driver(),
            platform.clone(),
            stop_receiver,
        ));
        tokio::task::yield_now().await;
        assert_eq!(platform.attempts(), 1);

        tokio::time::advance(Duration::from_millis(249)).await;
        tokio::task::yield_now().await;
        assert_eq!(platform.attempts(), 1);

        tokio::time::advance(Duration::from_millis(1)).await;
        tokio::task::yield_now().await;
        assert_eq!(platform.attempts(), 2);

        tokio::time::advance(Duration::from_millis(499)).await;
        tokio::task::yield_now().await;
        assert_eq!(platform.attempts(), 2);

        tokio::time::advance(Duration::from_millis(1)).await;
        tokio::task::yield_now().await;
        assert_eq!(platform.attempts(), 3);
        let connection_ids = platform.connection_ids();
        assert_eq!(connection_ids, vec![connection_ids[0]; 3]);
        assert_eq!(platform.service_uuids(), vec![STANDARD_SPP_SERVICE_UUID; 3]);

        task.abort();
    }

    #[tokio::test]
    #[traced_test]
    async fn synchronous_start_failure_logs_connection_address_and_reason() {
        let platform = FakePlatform::failing_with("Nearby Devices unavailable");
        let (events, mut receiver) = event_stream();
        let (_stop_sender, stop_receiver) = oneshot::channel();
        tokio::pin!(stop_receiver);

        let result = run_attempt(
            DEVICE_ID,
            ADDRESS,
            STANDARD_SPP_SERVICE_UUID,
            &driver(),
            &platform,
            &events,
            &mut receiver,
            stop_receiver.as_mut(),
        )
        .await;

        assert_eq!(
            result,
            AttemptResult::Completed {
                delivered_bytes: false
            }
        );
        logs_assert(|lines| {
            warning_context(
                lines,
                "SPP attempt failed to start",
                "Nearby Devices unavailable",
            )
        });
    }

    #[tokio::test]
    #[traced_test]
    async fn disconnect_error_logs_connection_address_and_reason() {
        let platform =
            FakePlatform::with_events(vec![r#"{"type":"disconnected","error":"socket closed"}"#]);
        let (events, mut receiver) = event_stream();
        let (_stop_sender, stop_receiver) = oneshot::channel();
        tokio::pin!(stop_receiver);

        let result = run_attempt(
            DEVICE_ID,
            ADDRESS,
            STANDARD_SPP_SERVICE_UUID,
            &driver(),
            &platform,
            &events,
            &mut receiver,
            stop_receiver.as_mut(),
        )
        .await;

        assert_eq!(
            result,
            AttemptResult::Completed {
                delivered_bytes: false
            }
        );
        logs_assert(|lines| warning_context(lines, "SPP attempt disconnected", "socket closed"));
    }

    #[tokio::test(start_paused = true)]
    #[traced_test]
    async fn failed_cancellation_does_not_start_another_attempt() {
        let platform = Arc::new(FakePlatform::with_cancel_error("cancel command failed"));
        let (_stop_sender, stop_receiver) = oneshot::channel();
        let task = tokio::spawn(maintain(
            DEVICE_ID,
            ADDRESS.to_owned(),
            STANDARD_SPP_SERVICE_UUID,
            driver(),
            platform.clone(),
            stop_receiver,
        ));
        tokio::task::yield_now().await;
        assert_eq!(platform.attempts(), 1);

        platform.send(r#"{"type":"secret-payload","data":"do-not-log"}"#);
        tokio::task::yield_now().await;
        assert_eq!(platform.cancellations(), 1);
        assert!(!task.is_finished());

        tokio::time::advance(Duration::from_secs(11)).await;
        tokio::task::yield_now().await;

        assert_eq!(platform.attempts(), 1);
        assert!(!task.is_finished());
        logs_assert(|lines| {
            warning_context(
                lines,
                "SPP attempt cancellation failed",
                "cancel command failed",
            )
        });
        assert!(!logs_contain("do-not-log"));

        task.abort();
    }

    #[tokio::test]
    #[traced_test]
    async fn receiver_closure_stops_the_supervisor() {
        let platform = Arc::new(FakePlatform::with_events(Vec::new()));
        let events = Channel::new(|_| Ok(()));
        let (sender, receiver) = mpsc::unbounded_channel::<InvokeResponseBody>();
        drop(sender);
        let (_stop_sender, stop_receiver) = oneshot::channel();
        tokio::pin!(stop_receiver);

        timeout(
            PATIENCE,
            maintain_on_channel(
                DEVICE_ID,
                ADDRESS.to_owned(),
                STANDARD_SPP_SERVICE_UUID,
                driver(),
                platform.clone(),
                events,
                receiver,
                stop_receiver.as_mut(),
            ),
        )
        .await
        .expect("supervisor stops after its maintained receiver closes");

        assert_eq!(platform.attempts(), 1);
        logs_assert(|lines| warning_context(lines, "SPP event channel closed", "channel closed"));
    }

    #[tokio::test(start_paused = true)]
    async fn stopping_before_the_task_starts_does_not_acquire_the_platform() {
        let platform = Arc::new(FakePlatform::with_events(vec![
            r#"{"type":"disconnected"}"#,
        ]));
        let maintained = spawn_maintained(
            DEVICE_ID,
            ADDRESS.to_owned(),
            STANDARD_SPP_SERVICE_UUID,
            driver(),
            platform.clone(),
        );

        (maintained.stop)();
        timeout(PATIENCE, maintained.task)
            .await
            .expect("supervisor stops before starting an attempt")
            .expect("supervisor task succeeds");

        assert_eq!(platform.attempts(), 0);
        assert_eq!(platform.cancellations(), 0);
    }

    #[tokio::test(start_paused = true)]
    async fn stopping_an_active_spp_attempt_cancels_and_waits_for_disconnection() {
        let platform = Arc::new(FakePlatform::with_events(Vec::new()));
        let maintained = spawn_maintained(
            DEVICE_ID,
            ADDRESS.to_owned(),
            STANDARD_SPP_SERVICE_UUID,
            driver(),
            platform.clone(),
        );
        tokio::task::yield_now().await;
        assert_eq!(platform.attempts(), 1);

        (maintained.stop)();
        platform.send(r#"{"type":"connected"}"#);
        tokio::task::yield_now().await;

        assert_eq!(platform.cancellations(), 1);
        assert!(!maintained.task.is_finished());

        platform.send(r#"{"type":"disconnected"}"#);
        timeout(PATIENCE, maintained.task)
            .await
            .expect("supervisor finishes after disconnection")
            .expect("supervisor task succeeds");
    }

    #[tokio::test]
    async fn two_active_spp_connections_deliver_bytes_to_their_own_device_ids() {
        let platform = Arc::new(FakePlatform::with_events(Vec::new()));
        let handle = driver_with_spp_addresses(&[ADDRESS, SECOND_ADDRESS]);
        let mut topics = topic_stream(&handle);
        let first = spawn_maintained(
            ExternalDeviceId(1),
            ADDRESS.to_owned(),
            STANDARD_SPP_SERVICE_UUID,
            handle.clone(),
            platform.clone(),
        );
        tokio::task::yield_now().await;
        let second = spawn_maintained(
            ExternalDeviceId(2),
            SECOND_ADDRESS.to_owned(),
            STANDARD_SPP_SERVICE_UUID,
            handle,
            platform.clone(),
        );
        tokio::task::yield_now().await;

        platform.send_on(0, r#"{"type":"connected"}"#);
        platform.send_on(0, RMC_EVENT);
        assert_eq!(next_position(&mut topics).await.latitude_degrees, 50.823);

        platform.send_on(1, r#"{"type":"connected"}"#);
        platform.send_on(1, SECOND_RMC_EVENT);
        assert_eq!(next_position(&mut topics).await.latitude_degrees, 51.823);

        (first.stop)();
        (second.stop)();
        tokio::task::yield_now().await;
        platform.send_on(0, r#"{"type":"disconnected"}"#);
        platform.send_on(1, r#"{"type":"disconnected"}"#);
        timeout(PATIENCE, first.task)
            .await
            .expect("first supervisor finishes")
            .expect("first supervisor task succeeds");
        timeout(PATIENCE, second.task)
            .await
            .expect("second supervisor finishes")
            .expect("second supervisor task succeeds");
    }

    #[tokio::test]
    async fn stopping_one_of_two_active_spp_connections_cancels_only_its_channel() {
        let platform = Arc::new(FakePlatform::with_events(Vec::new()));
        let first = spawn_maintained(
            ExternalDeviceId(1),
            ADDRESS.to_owned(),
            STANDARD_SPP_SERVICE_UUID,
            driver(),
            platform.clone(),
        );
        tokio::task::yield_now().await;
        assert_eq!(platform.attempts(), 1);

        let second = spawn_maintained(
            ExternalDeviceId(2),
            SECOND_ADDRESS.to_owned(),
            STANDARD_SPP_SERVICE_UUID,
            driver(),
            platform.clone(),
        );
        tokio::task::yield_now().await;
        assert_eq!(platform.attempts(), 2);

        let connection_ids = platform.connection_ids();
        assert_ne!(connection_ids[0], connection_ids[1]);

        (first.stop)();
        tokio::task::yield_now().await;
        assert_eq!(platform.cancelled_ids(), vec![connection_ids[0]]);
        assert!(!second.task.is_finished());

        platform.send_on(0, r#"{"type":"disconnected"}"#);
        timeout(PATIENCE, first.task)
            .await
            .expect("first supervisor finishes")
            .expect("first supervisor task succeeds");

        (second.stop)();
        tokio::task::yield_now().await;
        assert_eq!(
            platform.cancelled_ids(),
            vec![connection_ids[0], connection_ids[1]]
        );
        platform.send_on(1, r#"{"type":"disconnected"}"#);
        timeout(PATIENCE, second.task)
            .await
            .expect("second supervisor finishes")
            .expect("second supervisor task succeeds");
    }

    #[tokio::test]
    #[traced_test]
    async fn malformed_event_on_one_spp_connection_does_not_cancel_the_other() {
        let platform = Arc::new(FakePlatform::with_events(Vec::new()));
        let first = spawn_maintained(
            ExternalDeviceId(1),
            ADDRESS.to_owned(),
            STANDARD_SPP_SERVICE_UUID,
            driver(),
            platform.clone(),
        );
        tokio::task::yield_now().await;
        let second = spawn_maintained(
            ExternalDeviceId(2),
            SECOND_ADDRESS.to_owned(),
            STANDARD_SPP_SERVICE_UUID,
            driver(),
            platform.clone(),
        );
        tokio::task::yield_now().await;
        let connection_ids = platform.connection_ids();

        platform.send_on(0, r#"{"type":"secret-payload","data":"do-not-log"}"#);
        tokio::task::yield_now().await;

        assert_eq!(platform.cancelled_ids(), vec![connection_ids[0]]);
        assert!(!second.task.is_finished());
        assert!(!logs_contain("do-not-log"));

        (first.stop)();
        tokio::task::yield_now().await;
        platform.send_on(0, r#"{"type":"disconnected"}"#);
        timeout(PATIENCE, first.task)
            .await
            .expect("first supervisor finishes")
            .expect("first supervisor task succeeds");

        (second.stop)();
        tokio::task::yield_now().await;
        platform.send_on(1, r#"{"type":"disconnected"}"#);
        timeout(PATIENCE, second.task)
            .await
            .expect("second supervisor finishes")
            .expect("second supervisor task succeeds");
    }

    #[tokio::test(start_paused = true)]
    async fn driver_termination_cancels_an_active_attempt_without_reconnecting() {
        for event in [r#"{"type":"connected"}"#, RMC_EVENT] {
            let platform = Arc::new(FakePlatform::with_events(Vec::new()));
            let driver = test_support::spawn(
                SettingsSnapshot {
                    settings: Default::default(),
                    external_devices: vec![ExternalDeviceConfig {
                        enabled: true,
                        spec: ConnectionSpec::bluetooth_spp(ADDRESS),
                    }],
                },
                Box::new(|_, _, _| Box::new(|| {})),
                Box::new(|_| {}),
                Duration::from_secs(60),
            );
            let maintained = spawn_maintained(
                DEVICE_ID,
                ADDRESS.to_owned(),
                STANDARD_SPP_SERVICE_UUID,
                driver.handle.clone(),
                platform.clone(),
            );
            tokio::task::yield_now().await;
            assert_eq!(platform.attempts(), 1);

            driver.terminate().await;
            platform.send(event);
            timeout(PATIENCE, maintained.task)
                .await
                .expect("supervisor finishes after driver termination")
                .expect("supervisor task succeeds");

            tokio::time::advance(Duration::from_secs(10)).await;
            tokio::task::yield_now().await;
            assert_eq!(platform.cancellations(), 1);
            assert_eq!(platform.attempts(), 1);
        }
    }

    #[tokio::test]
    #[traced_test]
    async fn intentional_stop_suppresses_terminal_disconnect_warning() {
        let platform = Arc::new(FakePlatform::with_cancel_error("cancel command failed"));
        let maintained = spawn_maintained(
            DEVICE_ID,
            ADDRESS.to_owned(),
            STANDARD_SPP_SERVICE_UUID,
            driver(),
            platform.clone(),
        );
        tokio::task::yield_now().await;

        (maintained.stop)();
        platform.send(r#"{"type":"disconnected","error":"socket closed"}"#);
        timeout(PATIENCE, maintained.task)
            .await
            .expect("supervisor finishes after disconnection")
            .expect("supervisor task succeeds");

        assert_eq!(platform.cancellations(), 1);
        logs_assert(|lines| {
            warning_context(
                lines,
                "SPP attempt cancellation failed",
                "cancel command failed",
            )
        });
        assert!(!logs_contain("SPP attempt disconnected"));
    }

    #[tokio::test(start_paused = true)]
    async fn stopping_after_spp_start_rejection_does_not_cancel_another_attempt() {
        let platform = Arc::new(FakePlatform::failing_with("already active"));
        let maintained = spawn_maintained(
            DEVICE_ID,
            ADDRESS.to_owned(),
            STANDARD_SPP_SERVICE_UUID,
            driver(),
            platform.clone(),
        );
        tokio::task::yield_now().await;
        assert_eq!(platform.attempts(), 1);

        (maintained.stop)();
        tokio::time::advance(Duration::from_millis(250)).await;
        tokio::task::yield_now().await;
        timeout(PATIENCE, maintained.task)
            .await
            .expect("supervisor stops during backoff")
            .expect("supervisor task succeeds");

        assert_eq!(platform.attempts(), 1);
        assert_eq!(platform.cancellations(), 0);
    }

    #[tokio::test]
    async fn stopping_wins_when_a_terminal_spp_event_is_already_ready() {
        let platform = Arc::new(FakePlatform::with_events(Vec::new()));
        let maintained = spawn_maintained(
            DEVICE_ID,
            ADDRESS.to_owned(),
            STANDARD_SPP_SERVICE_UUID,
            driver(),
            platform.clone(),
        );
        tokio::task::yield_now().await;

        (maintained.stop)();
        platform.send(r#"{"type":"disconnected"}"#);
        timeout(PATIENCE, maintained.task)
            .await
            .expect("supervisor finishes")
            .expect("supervisor task succeeds");

        assert_eq!(platform.cancellations(), 1);
    }
}
