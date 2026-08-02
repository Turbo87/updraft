use super::super::supervisor::{maintain, spawn_maintained};
use super::support::*;
use crate::driver::tests as driver_tests;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::oneshot;
use tokio::time::timeout;
use tracing_test::traced_test;
use updraft_core::{
    ConnectionSpec, ExternalDeviceConfig, STANDARD_SPP_SERVICE_UUID, SettingsSnapshot,
};

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

#[tokio::test(start_paused = true)]
async fn driver_termination_cancels_an_active_attempt_without_reconnecting() {
    for event in [r#"{"type":"connected"}"#, RMC_EVENT] {
        let platform = Arc::new(FakePlatform::with_events(Vec::new()));
        let driver = driver_tests::spawn(
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
