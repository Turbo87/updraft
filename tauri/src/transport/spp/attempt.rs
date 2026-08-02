use crate::driver::DriverHandle;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use std::pin::Pin;
use tauri::ipc::{Channel, InvokeResponseBody};
#[cfg(target_os = "android")]
use tauri::{AppHandle, Runtime};
#[cfg(target_os = "android")]
use tauri_plugin_updraft::UpdraftMobileExt;
use tauri_plugin_updraft::{SppConnectionId, SppEvent};
use tokio::sync::{mpsc, oneshot};
use updraft_core::{Bytes, ConnectionChanged, ConnectionState, ExternalDeviceId};
use uuid::Uuid;

pub trait SppPlatform: Send + Sync + 'static {
    fn start_attempt(
        &self,
        address: &str,
        service_uuid: Uuid,
        events: Channel,
    ) -> Result<(), String>;
    fn cancel_attempt(&self, connection_id: SppConnectionId) -> Result<(), String>;
}

#[cfg(target_os = "android")]
pub struct AndroidSppPlatform<R: Runtime>(pub AppHandle<R>);

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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AttemptResult {
    Completed { delivered_bytes: bool },
    EventStreamClosed,
    Stopped,
    DriverStopped,
}

#[expect(clippy::too_many_arguments)]
pub async fn run_attempt(
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
