mod reconnect;
#[cfg(any(target_os = "android", test))]
mod spp;
pub mod tcp;

use crate::driver::{DriverHandle, StopFn};
use tauri::{AppHandle, Runtime};
use updraft_core::{ConnectionSpec, ExternalDeviceId};
#[cfg(not(target_os = "android"))]
use updraft_core::{ConnectionState, Input};

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
        ConnectionSpec::BluetoothSpp { address } => {
            #[cfg(target_os = "android")]
            {
                spp::run(device_id, address, handle, app)
            }
            #[cfg(not(target_os = "android"))]
            {
                let _ = app;
                tracing::warn!(?device_id, %address, "Bluetooth SPP transport is unsupported");
                handle.send(Input::connection_changed(
                    device_id,
                    ConnectionState::Disconnected,
                ));
                Box::new(|| {})
            }
        }
    }
}
