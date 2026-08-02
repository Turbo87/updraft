use super::super::reconnect::ReconnectBackoff;
#[cfg(target_os = "android")]
use super::attempt::AndroidSppPlatform;
use super::attempt::{AttemptResult, SppPlatform, run_attempt};
use crate::driver::{DriverHandle, StopFn};
use std::{pin::Pin, sync::Arc};
use tauri::ipc::{Channel, InvokeResponseBody};
#[cfg(target_os = "android")]
use tauri::{AppHandle, Runtime};
use tokio::sync::{mpsc, oneshot};
use updraft_core::ExternalDeviceId;
use uuid::Uuid;

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

pub async fn maintain(
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
pub async fn maintain_on_channel(
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

pub struct Maintained {
    pub stop: StopFn,
    pub task: tokio::task::JoinHandle<()>,
}

pub fn spawn_maintained(
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
