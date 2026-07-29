use crate::connection::{ConnectionSpec, ConnectionState, ExternalDeviceId};

#[derive(Debug, Default)]
pub struct ConnectionDiagnostics {
    attempt: Attempt,
}

#[derive(Debug, Default)]
struct Attempt {
    connected: bool,
    first_bytes_reported: bool,
    delivered_bytes: usize,
}

impl ConnectionDiagnostics {
    pub fn changed(
        &mut self,
        device_id: ExternalDeviceId,
        spec: &ConnectionSpec,
        state: ConnectionState,
    ) {
        match state {
            ConnectionState::Connecting => {
                self.attempt = Attempt::default();
                tracing::debug!(device_id = ?device_id, endpoint = ?spec, "Connecting");
            }
            ConnectionState::Connected => {
                self.attempt = Attempt {
                    connected: true,
                    ..Attempt::default()
                };
                tracing::info!(device_id = ?device_id, endpoint = ?spec, "Connected");
            }
            ConnectionState::Disconnected => {
                if self.attempt.connected {
                    tracing::info!(
                        device_id = ?device_id,
                        endpoint = ?spec,
                        delivered_bytes = self.attempt.delivered_bytes,
                        "Disconnected"
                    );
                } else {
                    tracing::debug!(device_id = ?device_id, endpoint = ?spec, "Disconnected");
                }
                self.attempt = Attempt::default();
            }
        }
    }

    pub fn bytes(&mut self, device_id: ExternalDeviceId, spec: &ConnectionSpec, count: usize) {
        if count == 0 {
            return;
        }

        if !self.attempt.first_bytes_reported {
            tracing::info!(device_id = ?device_id, endpoint = ?spec, "First bytes");
            self.attempt.first_bytes_reported = true;
        }
        self.attempt.delivered_bytes += count;
    }
}
