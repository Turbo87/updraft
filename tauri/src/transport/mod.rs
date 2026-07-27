mod reconnect;
#[cfg(any(target_os = "android", test))]
mod spp;
pub mod tcp;

use crate::driver::DriverHandle;
use updraft_core::{ConnectionId, ConnectionSpec, ConnectionState, Input};

/// Brings up the transport for one connection spec.
///
/// The core names a link and how to reach it. Which socket type that
/// implies, and everything about keeping it alive, stops here.
pub fn open(connection: ConnectionId, spec: ConnectionSpec, handle: DriverHandle) {
    match spec {
        ConnectionSpec::Tcp { host, port } => tcp::run(connection, host, port, handle),
        ConnectionSpec::BluetoothSpp { address } => {
            tracing::warn!(?connection, %address, "Bluetooth SPP transport is not implemented");
            handle.send(Input::connection_changed(
                connection,
                ConnectionState::Disconnected,
            ));
        }
    }
}
