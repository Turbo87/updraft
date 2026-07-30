use serde::{Deserialize, Serialize};
use uuid::{Uuid, uuid};

pub const STANDARD_SPP_SERVICE_UUID: Uuid = uuid!("00001101-0000-1000-8000-00805F9B34FB");

/// Identifies one link to an external device.
///
/// The identity travels with every byte the link produces, because
/// position-source arbitration and failover need to know which device a
/// value came from.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ExternalDeviceId(pub u32);

/// How the shell should reach a device.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "type", rename_all_fields = "camelCase")]
pub enum ConnectionSpec {
    /// A TCP client link. Used for flight simulators, WiFi-attached
    /// instruments, and any device exposing a TCP server, as well as for
    /// feeding recorded NMEA during development.
    #[serde(rename = "tcp")]
    Tcp { host: String, port: u16 },
    /// A Bluetooth Classic Serial Port Profile link.
    #[serde(rename = "bluetooth")]
    BluetoothSpp {
        address: String,

        #[serde(
            default = "default_spp_service_uuid",
            skip_serializing_if = "is_standard_spp_service_uuid"
        )]
        service_uuid: Uuid,
    },
}

fn default_spp_service_uuid() -> Uuid {
    STANDARD_SPP_SERVICE_UUID
}

fn is_standard_spp_service_uuid(service_uuid: &Uuid) -> bool {
    *service_uuid == STANDARD_SPP_SERVICE_UUID
}

impl ConnectionSpec {
    pub fn tcp(host: impl Into<String>, port: u16) -> Self {
        Self::Tcp {
            host: host.into(),
            port,
        }
    }

    pub fn bluetooth_spp(address: impl Into<String>) -> Self {
        Self::BluetoothSpp {
            address: address.into(),
            service_uuid: default_spp_service_uuid(),
        }
    }
}

/// What the shell reports back about a link.
///
/// The shell owns reconnection and backoff between an open and a close
/// effect, so `Disconnected` describes the current situation rather than a
/// request for the core to do anything.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConnectionState {
    Connecting,
    Connected,
    Disconnected,
}
