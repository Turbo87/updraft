use crate::connection::{ConnectionSpec, ExternalDeviceId};
use crate::connection_diagnostics::ConnectionDiagnostics;
use crate::decoder::Decoder;

#[derive(Debug)]
pub struct ExternalDevice {
    pub device_id: ExternalDeviceId,
    pub spec: ConnectionSpec,
    pub decoder: Decoder,
    pub diagnostics: ConnectionDiagnostics,
}

#[derive(Debug, Default)]
pub struct ExternalDevices {
    entries: Vec<ExternalDevice>,
}

impl ExternalDevices {
    pub fn from_connections(connections: Vec<(ExternalDeviceId, ConnectionSpec)>) -> Self {
        Self {
            entries: connections
                .into_iter()
                .map(|(device_id, spec)| ExternalDevice {
                    device_id,
                    spec,
                    decoder: Decoder::default(),
                    diagnostics: ConnectionDiagnostics::default(),
                })
                .collect(),
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = &ExternalDevice> {
        self.entries.iter()
    }

    pub fn get_mut(&mut self, device_id: ExternalDeviceId) -> Option<&mut ExternalDevice> {
        self.entries
            .iter_mut()
            .rev()
            .find(|device| device.device_id == device_id)
    }

    #[cfg_attr(not(test), expect(dead_code))]
    pub fn remove(&mut self, device_id: ExternalDeviceId) -> Option<ExternalDevice> {
        let index = self
            .entries
            .iter()
            .position(|device| device.device_id == device_id)?;
        Some(self.entries.remove(index))
    }
}
