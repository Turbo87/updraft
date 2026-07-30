mod reconnect;
#[cfg(any(target_os = "android", test))]
mod spp;
pub mod tcp;

use crate::driver::{DriverHandle, StopFn};
use tauri::{AppHandle, Runtime};
#[cfg(not(target_os = "android"))]
use updraft_core::{ConnectionChanged, ConnectionState};
use updraft_core::{ConnectionSpec, ExternalDeviceId};

/// Brings up the transport for one connection spec.
///
/// The core names a link and how to reach it. Which socket type that
/// implies, and everything about keeping it alive, stops here.
pub fn open<R: Runtime>(
    device_id: ExternalDeviceId,
    spec: ConnectionSpec,
    handle: DriverHandle,
    app: AppHandle<R>,
) -> StopFn {
    match spec {
        ConnectionSpec::Tcp { host, port } => tcp::run(device_id, host, port, handle),
        ConnectionSpec::BluetoothSpp {
            address,
            service_uuid: _,
        } => {
            #[cfg(target_os = "android")]
            {
                spp::run(device_id, address, handle, app)
            }
            #[cfg(not(target_os = "android"))]
            {
                let _ = app;
                tracing::warn!(?device_id, %address, "Bluetooth SPP transport is unsupported");
                tauri::async_runtime::spawn(async move {
                    let input = ConnectionChanged::new(device_id, ConnectionState::Disconnected);
                    let _ = handle.send(input).await;
                });
                Box::new(|| {})
            }
        }
    }
}
