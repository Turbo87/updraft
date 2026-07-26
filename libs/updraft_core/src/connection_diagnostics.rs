use crate::connection::{ConnectionId, ConnectionSpec, ConnectionState};
use std::collections::BTreeMap;

#[derive(Debug, Default)]
pub struct ConnectionDiagnostics {
    connections: BTreeMap<ConnectionId, Connection>,
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
    pub fn insert(&mut self, connection: ConnectionId, endpoint: ConnectionSpec) {
        self.connections
            .insert(connection, Connection::new(endpoint));
    }

    #[cfg_attr(not(test), expect(dead_code))]
    pub fn remove(&mut self, connection: ConnectionId) {
        self.connections.remove(&connection);
    }

    pub fn changed(&mut self, connection: ConnectionId, state: ConnectionState) {
        let Some(entry) = self.connections.get_mut(&connection) else {
            return;
        };

        match state {
            ConnectionState::Connecting => {
                entry.attempt = Attempt::default();
                tracing::debug!(connection = ?connection, endpoint = ?entry.endpoint, "Connecting");
            }
            ConnectionState::Connected => {
                entry.attempt = Attempt {
                    connected: true,
                    ..Attempt::default()
                };
                tracing::info!(connection = ?connection, endpoint = ?entry.endpoint, "Connected");
            }
            ConnectionState::Disconnected => {
                if entry.attempt.connected {
                    tracing::info!(
                        connection = ?connection,
                        endpoint = ?entry.endpoint,
                        delivered_bytes = entry.attempt.delivered_bytes,
                        "Disconnected"
                    );
                } else {
                    tracing::debug!(connection = ?connection, endpoint = ?entry.endpoint, "Disconnected");
                }
                entry.attempt = Attempt::default();
            }
        }
    }

    pub fn bytes(&mut self, connection: ConnectionId, count: usize) {
        if count == 0 {
            return;
        }

        let Some(entry) = self.connections.get_mut(&connection) else {
            return;
        };

        if !entry.attempt.first_bytes_reported {
            tracing::info!(connection = ?connection, endpoint = ?entry.endpoint, "First bytes");
            entry.attempt.first_bytes_reported = true;
        }
        entry.attempt.delivered_bytes += count;
    }
}
