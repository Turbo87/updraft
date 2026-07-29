use crate::connection::{ConnectionSpec, ExternalDeviceId};
use crate::connection_diagnostics::ConnectionDiagnostics;
use crate::decoder::Decoder;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct ExternalDeviceConfig {
    pub enabled: bool,
    #[serde(flatten)]
    pub spec: ConnectionSpec,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct PublishedExternalDevice {
    pub device_id: ExternalDeviceId,
    #[serde(flatten)]
    pub config: ExternalDeviceConfig,
}

#[derive(Debug)]
pub struct ExternalDevice {
    pub device_id: ExternalDeviceId,
    pub config: ExternalDeviceConfig,
    pub decoder: Decoder,
    pub diagnostics: ConnectionDiagnostics,
}

#[derive(Debug)]
pub struct ExternalDevices {
    next_device_id: u32,
    entries: Vec<ExternalDevice>,
}

impl Default for ExternalDevices {
    fn default() -> Self {
        Self {
            next_device_id: 1,
            entries: Vec::new(),
        }
    }
}

impl ExternalDevices {
    pub fn from_device_configs(devices: Vec<ExternalDeviceConfig>) -> Self {
        let mut external_devices = Self::default();
        for config in devices {
            let device_id = external_devices.allocate_device_id();
            external_devices.entries.push(ExternalDevice {
                device_id,
                config,
                decoder: Decoder::default(),
                diagnostics: ConnectionDiagnostics::default(),
            });
        }
        external_devices
    }

    pub fn iter(&self) -> impl Iterator<Item = &ExternalDevice> {
        self.entries.iter()
    }

    pub fn get_mut(&mut self, device_id: ExternalDeviceId) -> Option<&mut ExternalDevice> {
        self.entries
            .iter_mut()
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

    pub fn published_devices(&self) -> Vec<PublishedExternalDevice> {
        self.entries
            .iter()
            .map(|device| PublishedExternalDevice {
                device_id: device.device_id,
                config: device.config.clone(),
            })
            .collect()
    }

    pub fn device_configs(&self) -> Vec<ExternalDeviceConfig> {
        self.entries
            .iter()
            .map(|device| device.config.clone())
            .collect()
    }

    fn allocate_device_id(&mut self) -> ExternalDeviceId {
        let device_id = ExternalDeviceId(self.next_device_id);
        self.next_device_id += 1;
        device_id
    }
}
