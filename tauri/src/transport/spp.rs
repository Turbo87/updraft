use super::reconnect::ReconnectBackoff;
use crate::driver::DriverHandle;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use std::sync::Arc;
use tauri::ipc::{Channel, InvokeResponseBody};
#[cfg(target_os = "android")]
use tauri::{AppHandle, Runtime};
use tauri_plugin_updraft::SppEvent;
#[cfg(target_os = "android")]
use tauri_plugin_updraft::UpdraftMobileExt;
use tokio::sync::mpsc;
use updraft_core::{ConnectionId, ConnectionState, Input};

trait SppPlatform: Send + Sync + 'static {
    fn start_attempt(&self, address: &str, events: Channel) -> Result<(), String>;
    fn cancel_attempt(&self) -> Result<(), String>;
}

#[cfg(target_os = "android")]
struct AndroidSppPlatform<R: Runtime>(AppHandle<R>);

#[cfg(target_os = "android")]
impl<R: Runtime> SppPlatform for AndroidSppPlatform<R> {
    fn start_attempt(&self, address: &str, events: Channel) -> Result<(), String> {
        self.0
            .updraft_mobile()
            .start_spp_attempt(address, events)
            .map_err(|error| error.to_string())
    }

    fn cancel_attempt(&self) -> Result<(), String> {
        self.0
            .updraft_mobile()
            .cancel_spp_attempt()
            .map_err(|error| error.to_string())
    }
}

#[cfg(target_os = "android")]
pub fn run<R: Runtime>(
    connection: ConnectionId,
    address: String,
    handle: DriverHandle,
    app: AppHandle<R>,
) {
    tokio::spawn(maintain(
        connection,
        address,
        handle,
        Arc::new(AndroidSppPlatform(app)),
    ));
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AttemptResult {
    Completed { delivered_bytes: bool },
    EventStreamClosed,
}

async fn run_attempt(
    connection: ConnectionId,
    address: &str,
    handle: &DriverHandle,
    platform: &dyn SppPlatform,
    events: &Channel,
    receiver: &mut mpsc::UnboundedReceiver<InvokeResponseBody>,
) -> AttemptResult {
    handle.send(Input::connection_changed(
        connection,
        ConnectionState::Connecting,
    ));

    let result = match platform.start_attempt(address, events.clone()) {
        Err(reason) => {
            tracing::warn!(
                ?connection,
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
            loop {
                let Some(body) = receiver.recv().await else {
                    tracing::warn!(
                        ?connection,
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
                            ?connection,
                            %address,
                            reason = "malformed channel data",
                            "Malformed SPP event"
                        );
                        cancel_attempt(connection, address, platform);
                        cancelling = true;
                        continue;
                    }
                };

                if cancelling {
                    if let SppEvent::Disconnected { error } = event {
                        if let Some(reason) = error {
                            tracing::warn!(
                                ?connection,
                                %address,
                                %reason,
                                "SPP attempt disconnected"
                            );
                        }
                        break AttemptResult::Completed { delivered_bytes };
                    }
                    continue;
                }

                match event {
                    SppEvent::Connected => handle.send(Input::connection_changed(
                        connection,
                        ConnectionState::Connected,
                    )),
                    SppEvent::Bytes { data } => match STANDARD.decode(data) {
                        Ok(bytes) => {
                            delivered_bytes |= !bytes.is_empty();
                            handle.send(Input::bytes(connection, bytes));
                        }
                        Err(_) => {
                            tracing::warn!(
                                ?connection,
                                %address,
                                reason = "invalid Base64 data",
                                "Invalid Base64 SPP bytes"
                            );
                            cancel_attempt(connection, address, platform);
                            cancelling = true;
                        }
                    },
                    SppEvent::Disconnected { error } => {
                        if let Some(reason) = error {
                            tracing::warn!(
                                ?connection,
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

    handle.send(Input::connection_changed(
        connection,
        ConnectionState::Disconnected,
    ));
    result
}

fn cancel_attempt(connection: ConnectionId, address: &str, platform: &dyn SppPlatform) {
    if let Err(reason) = platform.cancel_attempt() {
        tracing::warn!(
            ?connection,
            %address,
            %reason,
            "SPP attempt cancellation failed"
        );
    }
}

async fn maintain(
    connection: ConnectionId,
    address: String,
    handle: DriverHandle,
    platform: Arc<dyn SppPlatform>,
) {
    let (sender, receiver) = mpsc::unbounded_channel::<InvokeResponseBody>();
    let events = Channel::new(move |body| {
        let _ = sender.send(body);
        Ok(())
    });
    maintain_on_channel(connection, address, handle, platform, events, receiver).await;
}

async fn maintain_on_channel(
    connection: ConnectionId,
    address: String,
    handle: DriverHandle,
    platform: Arc<dyn SppPlatform>,
    events: Channel,
    mut receiver: mpsc::UnboundedReceiver<InvokeResponseBody>,
) {
    let mut backoff = ReconnectBackoff::default();

    loop {
        match run_attempt(
            connection,
            &address,
            &handle,
            platform.as_ref(),
            &events,
            &mut receiver,
        )
        .await
        {
            AttemptResult::Completed { delivered_bytes } => {
                tokio::time::sleep(backoff.after_attempt(delivered_bytes)).await;
            }
            AttemptResult::EventStreamClosed => return,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{AttemptResult, SppPlatform, maintain, maintain_on_channel, run_attempt};
    use crate::driver::{Driver, DriverHandle};
    use claims::assert_some;
    use std::sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    };
    use std::time::Duration;
    use tauri::ipc::{Channel, InvokeResponseBody};
    use tokio::sync::mpsc;
    use tokio::time::timeout;
    use tracing_test::traced_test;
    use updraft_core::{ConnectionId, ConnectionSpec, CoreConfig, Topic};

    const ADDRESS: &str = "00:11:22:33:44:55";
    const LINK: ConnectionId = ConnectionId(1);
    const PATIENCE: Duration = Duration::from_secs(5);
    const RMC_EVENT: &str = r#"{"type":"bytes","data":"JEdQUk1DLDEyMDAwMC4wMCxBLDUwNDkuMzgsTiwwMDYxMS4xNixFLDQ1LjAsMjcwLjAsMDEwMTI2LCwsQQ0K"}"#;

    struct FakePlatform {
        events: Vec<&'static str>,
        start_error: Option<&'static str>,
        cancel_error: Option<&'static str>,
        attempts: AtomicUsize,
        cancellations: AtomicUsize,
        channel_ids: Mutex<Vec<u32>>,
        channels: Mutex<Vec<Channel>>,
    }

    impl FakePlatform {
        fn with_events(events: Vec<&'static str>) -> Self {
            Self {
                events,
                start_error: None,
                cancel_error: None,
                attempts: AtomicUsize::new(0),
                cancellations: AtomicUsize::new(0),
                channel_ids: Mutex::new(Vec::new()),
                channels: Mutex::new(Vec::new()),
            }
        }

        fn failing_with(reason: &'static str) -> Self {
            Self {
                events: Vec::new(),
                start_error: Some(reason),
                cancel_error: None,
                attempts: AtomicUsize::new(0),
                cancellations: AtomicUsize::new(0),
                channel_ids: Mutex::new(Vec::new()),
                channels: Mutex::new(Vec::new()),
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
            self.cancellations.load(Ordering::SeqCst)
        }

        fn channel_ids(&self) -> Vec<u32> {
            self.channel_ids.lock().expect("channel IDs lock").clone()
        }

        fn send(&self, payload: &str) {
            let channel = self
                .channels
                .lock()
                .expect("channels lock")
                .last()
                .expect("an active attempt channel")
                .clone();
            channel
                .send(InvokeResponseBody::Json(payload.to_owned()))
                .expect("fake event reaches the channel");
        }
    }

    impl SppPlatform for FakePlatform {
        fn start_attempt(&self, address: &str, events: Channel) -> Result<(), String> {
            assert_eq!(address, ADDRESS);
            self.attempts.fetch_add(1, Ordering::SeqCst);
            self.channel_ids
                .lock()
                .expect("channel IDs lock")
                .push(events.id());
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

        fn cancel_attempt(&self) -> Result<(), String> {
            self.cancellations.fetch_add(1, Ordering::SeqCst);
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
        tokio::spawn(async move {
            run_attempt(
                LINK,
                ADDRESS,
                &handle,
                platform.as_ref(),
                &events,
                &mut receiver,
            )
            .await
        })
    }

    fn driver() -> DriverHandle {
        Driver::spawn(
            CoreConfig {
                connections: vec![(LINK, ConnectionSpec::bluetooth_spp(ADDRESS))],
                ..CoreConfig::default()
            },
            Box::new(|_, _, _| {}),
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

    async fn next_position(receiver: &mut mpsc::UnboundedReceiver<Topic>) {
        loop {
            let received = timeout(PATIENCE, receiver.recv())
                .await
                .expect("a topic within the timeout");
            let Topic::Instruments(instruments) = assert_some!(received) else {
                continue;
            };
            if instruments.position.is_some() {
                return;
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
            format!(" connection={LINK:?}"),
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

        let result = run_attempt(LINK, ADDRESS, &handle, &platform, &events, &mut receiver).await;

        assert_eq!(
            result,
            AttemptResult::Completed {
                delivered_bytes: true
            }
        );
        next_position(&mut topics).await;
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
        let task = tokio::spawn(maintain(
            LINK,
            ADDRESS.to_owned(),
            driver(),
            platform.clone(),
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
        let channel_ids = platform.channel_ids();
        assert!(channel_ids.windows(2).all(|ids| ids[0] == ids[1]));

        task.abort();
    }

    #[tokio::test]
    #[traced_test]
    async fn synchronous_start_failure_logs_connection_address_and_reason() {
        let platform = FakePlatform::failing_with("Nearby Devices unavailable");
        let (events, mut receiver) = event_stream();

        let result = run_attempt(LINK, ADDRESS, &driver(), &platform, &events, &mut receiver).await;

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

        let result = run_attempt(LINK, ADDRESS, &driver(), &platform, &events, &mut receiver).await;

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
        let task = tokio::spawn(maintain(
            LINK,
            ADDRESS.to_owned(),
            driver(),
            platform.clone(),
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

        timeout(
            PATIENCE,
            maintain_on_channel(
                LINK,
                ADDRESS.to_owned(),
                driver(),
                platform.clone(),
                events,
                receiver,
            ),
        )
        .await
        .expect("supervisor stops after its maintained receiver closes");

        assert_eq!(platform.attempts(), 1);
        logs_assert(|lines| warning_context(lines, "SPP event channel closed", "channel closed"));
    }
}
