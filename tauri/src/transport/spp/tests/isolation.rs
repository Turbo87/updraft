use super::super::supervisor::spawn_maintained;
use super::support::*;
use std::sync::Arc;
use tokio::time::timeout;
use tracing_test::traced_test;
use updraft_core::{ExternalDeviceId, STANDARD_SPP_SERVICE_UUID};

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
