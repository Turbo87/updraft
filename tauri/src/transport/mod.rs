mod reconnect;
#[cfg(any(target_os = "android", test))]
mod spp;
pub mod tcp;

use crate::driver::DriverHandle;
use tauri::{AppHandle, Runtime};
use updraft_core::{ConnectionId, ConnectionSpec};
#[cfg(not(target_os = "android"))]
use updraft_core::{ConnectionState, Input};

/// Brings up the transport for one connection spec.
///
/// The core names a link and how to reach it. Which socket type that
/// implies, and everything about keeping it alive, stops here.
pub fn open<R: Runtime>(
    connection: ConnectionId,
    spec: ConnectionSpec,
    handle: DriverHandle,
    app: AppHandle<R>,
) {
    match spec {
        ConnectionSpec::Tcp { host, port } => tcp::run(connection, host, port, handle),
        ConnectionSpec::BluetoothSpp { address } => {
            #[cfg(target_os = "android")]
            spp::run(connection, address, handle, app);
            #[cfg(not(target_os = "android"))]
            {
                let _ = app;
                tracing::warn!(?connection, %address, "Bluetooth SPP transport is unsupported");
                handle.send(Input::connection_changed(
                    connection,
                    ConnectionState::Disconnected,
                ));
            }
        }
    }
}
