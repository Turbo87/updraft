use crate::Topic;
use crate::connection::{ConnectionSpec, ExternalDeviceId};
use crate::connection_diagnostics::ConnectionDiagnostics;
use crate::decoder::Decoder;
use crate::ownship::GpsCandidate;
use crate::ownship::Timed;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use updraft_units::PressureAltitude;

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
    pub gps: GpsCandidate,
    pub pressure_altitude: Option<Timed<PressureAltitude>>,
}

impl ExternalDevice {
    pub fn reset_runtime(&mut self) {
        self.decoder = Decoder::default();
        self.diagnostics = ConnectionDiagnostics::default();
        self.gps = GpsCandidate::default();
        self.pressure_altitude = None;
    }
}

#[derive(Debug)]
pub struct ExternalDevices {
    next_device_id: u32,
    entries: Vec<ExternalDevice>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
#[error("unknown external device: {device_id:?}")]
pub struct UnknownExternalDevice {
    pub device_id: ExternalDeviceId,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
#[error("invalid external device order")]
pub struct InvalidExternalDeviceOrder;

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
                gps: GpsCandidate::default(),
                pressure_altitude: None,
            });
        }
        external_devices
    }

    pub fn iter(&self) -> impl Iterator<Item = &ExternalDevice> {
        self.entries.iter()
    }

    pub fn get(&self, device_id: ExternalDeviceId) -> Option<&ExternalDevice> {
        self.entries
            .iter()
            .find(|device| device.device_id == device_id)
    }

    pub fn get_mut(&mut self, device_id: ExternalDeviceId) -> Option<&mut ExternalDevice> {
        self.entries
            .iter_mut()
            .find(|device| device.device_id == device_id)
    }

    pub fn add(&mut self, spec: ConnectionSpec) -> ExternalDeviceId {
        let device_id = self.allocate_device_id();
        self.entries.push(ExternalDevice {
            device_id,
            config: ExternalDeviceConfig {
                enabled: true,
                spec,
            },
            decoder: Decoder::default(),
            diagnostics: ConnectionDiagnostics::default(),
            gps: GpsCandidate::default(),
            pressure_altitude: None,
        });
        device_id
    }

    pub fn remove(&mut self, device_id: ExternalDeviceId) -> Option<ExternalDevice> {
        let index = self
            .entries
            .iter()
            .position(|device| device.device_id == device_id)?;
        Some(self.entries.remove(index))
    }

    pub fn reorder(
        &mut self,
        order: &[ExternalDeviceId],
    ) -> Result<bool, InvalidExternalDeviceOrder> {
        if self
            .entries
            .iter()
            .map(|device| device.device_id)
            .eq(order.iter().copied())
        {
            return Ok(false);
        }

        let current = self
            .entries
            .iter()
            .map(|device| device.device_id)
            .collect::<BTreeSet<_>>();
        let requested = order.iter().copied().collect::<BTreeSet<_>>();
        if order.len() != self.entries.len()
            || requested.len() != order.len()
            || requested != current
        {
            return Err(InvalidExternalDeviceOrder);
        }

        let mut entries = std::mem::take(&mut self.entries);
        self.entries = order
            .iter()
            .map(|device_id| {
                let index = entries
                    .iter()
                    .position(|device| device.device_id == *device_id)
                    .expect("validated external device order");
                entries.remove(index)
            })
            .collect();
        Ok(true)
    }

    pub fn as_topic(&self) -> Topic {
        Topic::ExternalDevices(self.published_devices())
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
