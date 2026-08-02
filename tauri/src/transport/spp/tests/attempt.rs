use super::super::attempt::{AttemptResult, run_attempt};
use super::super::supervisor::{maintain, maintain_on_channel};
use super::support::*;
use claims::assert_some;
use std::sync::Arc;
use std::time::Duration;
use tauri::ipc::{Channel, InvokeResponseBody};
use tokio::sync::{mpsc, oneshot};
use tokio::time::timeout;
use tracing_test::traced_test;
use updraft_core::{STANDARD_SPP_SERVICE_UUID, Topic};

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
    logs_assert(|lines| warning_context(lines, "Malformed SPP event", "malformed channel data"));
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
    logs_assert(|lines| warning_context(lines, "Invalid Base64 SPP bytes", "invalid Base64 data"));
    assert!(!logs_contain("do-not-log"));
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
