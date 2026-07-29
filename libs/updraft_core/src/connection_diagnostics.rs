use crate::connection::{ConnectionSpec, ConnectionState, ExternalDeviceId};
use std::collections::BTreeMap;

#[derive(Debug, Default)]
pub struct ConnectionDiagnostics {
    connections: BTreeMap<ExternalDeviceId, Connection>,
}

#[derive(Debug)]
struct Connection {
    endpoint: ConnectionSpec,
    attempt: Attempt,
}

impl Connection {
    pub fn new(endpoint: ConnectionSpec) -> Self {
        Self {
            endpoint,
            attempt: Attempt::default(),
        }
    }
}

#[derive(Debug, Default)]
struct Attempt {
    connected: bool,
    first_bytes_reported: bool,
    delivered_bytes: usize,
}

impl ConnectionDiagnostics {
    pub fn insert(&mut self, device_id: ExternalDeviceId, endpoint: ConnectionSpec) {
        self.connections
            .insert(device_id, Connection::new(endpoint));
    }

    #[cfg_attr(not(test), expect(dead_code))]
    pub fn remove(&mut self, device_id: ExternalDeviceId) {
        self.connections.remove(&device_id);
    }

    pub fn changed(&mut self, device_id: ExternalDeviceId, state: ConnectionState) {
        let Some(entry) = self.connections.get_mut(&device_id) else {
            return;
        };

        match state {
            ConnectionState::Connecting => {
                entry.attempt = Attempt::default();
                tracing::debug!(device_id = ?device_id, endpoint = ?entry.endpoint, "Connecting");
            }
            ConnectionState::Connected => {
                entry.attempt = Attempt {
                    connected: true,
                    ..Attempt::default()
                };
                tracing::info!(device_id = ?device_id, endpoint = ?entry.endpoint, "Connected");
            }
            ConnectionState::Disconnected => {
                if entry.attempt.connected {
                    tracing::info!(
                        device_id = ?device_id,
                        endpoint = ?entry.endpoint,
                        delivered_bytes = entry.attempt.delivered_bytes,
                        "Disconnected"
                    );
                } else {
                    tracing::debug!(device_id = ?device_id, endpoint = ?entry.endpoint, "Disconnected");
                }
                entry.attempt = Attempt::default();
            }
        }
    }

    pub fn bytes(&mut self, device_id: ExternalDeviceId, count: usize) {
        if count == 0 {
            return;
        }

        let Some(entry) = self.connections.get_mut(&device_id) else {
            return;
        };

        if !entry.attempt.first_bytes_reported {
            tracing::info!(device_id = ?device_id, endpoint = ?entry.endpoint, "First bytes");
            entry.attempt.first_bytes_reported = true;
        }
        entry.attempt.delivered_bytes += count;
    }
}
