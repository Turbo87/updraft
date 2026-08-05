use serde::{Deserialize, Serialize};
use tauri::ipc::Channel;

/// One device in the Android bonded Bluetooth set.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BondedBluetoothDevice {
    pub address: String,
    pub name: Option<String>,
}

/// The current bonded Bluetooth devices or the reason they are unavailable.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(tag = "status", rename_all = "camelCase", deny_unknown_fields)]
pub enum BondedBluetoothDevices {
    Unsupported,
    PermissionDenied,
    Disabled,
    Available { devices: Vec<BondedBluetoothDevice> },
}

/// A position report from the device's own GNSS receiver.
///
/// Fielded as `Location.toFix()` fields it in
/// `libs/tauri_plugin_updraft/android/src/main/java/GpsSource.kt`, which is
/// the other half of this contract.
///
/// `deny_unknown_fields` is what makes a rename there loud. Without it a
/// renamed optional field deserializes to `None`, and because the core writes
/// only the values a fix carries, that instrument would hold its last reading
/// while the position kept moving. A frozen track beside a live position
/// reads as a working receiver. Rejecting the whole fix costs a log line
/// once a second instead.
///
/// Altitude is height above the WGS84 ellipsoid, which is what the platform
/// reports. Correcting it to mean sea level is a domain conversion and stays
/// out of the plugin.
///
/// Track, speed, and altitude are optional. A receiver can have a position
/// and time without those values. A placeholder would be indistinguishable
/// from a real reading.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Fix {
    pub latitude_degrees: f64,
    pub longitude_degrees: f64,
    pub unix_time_milliseconds: i64,
    pub altitude_ellipsoid_meters: Option<f64>,
    pub track_degrees: Option<f64>,
    pub ground_speed_meters_per_second: Option<f64>,
}

/// One event from an Android Bluetooth SPP connection attempt.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "camelCase", deny_unknown_fields)]
pub enum SppEvent {
    Connected,
    Bytes { data: String },
    Disconnected { error: Option<String> },
}

/// Identifies one maintained Bluetooth SPP connection by its Tauri channel.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct SppConnectionId(u32);

impl SppConnectionId {
    pub fn from_channel(channel: &Channel) -> Self {
        Self(channel.id())
    }
}

#[cfg(test)]
mod tests {
    use super::{BondedBluetoothDevices, SppEvent};
    use claims::assert_err;
    use tauri::ipc::InvokeResponseBody;

    fn bonded_devices(payload: &str) -> BondedBluetoothDevices {
        InvokeResponseBody::Json(payload.to_owned())
            .deserialize()
            .expect("payload is a valid bonded-device result")
    }

    #[test]
    fn bonded_device_results_use_a_tagged_camel_case_contract() {
        let results = [
            bonded_devices(r#"{"status":"unsupported"}"#),
            bonded_devices(r#"{"status":"permissionDenied"}"#),
            bonded_devices(r#"{"status":"disabled"}"#),
            bonded_devices(
                r#"{"status":"available","devices":[{"address":"00:11:22:33:44:55","name":"Flight recorder"},{"address":"AA:BB:CC:DD:EE:FF","name":null}]}"#,
            ),
        ];

        insta::assert_json_snapshot!(results);
    }

    #[test]
    fn bonded_device_results_reject_unknown_fields() {
        let result = InvokeResponseBody::Json(
            r#"{"status":"available","devices":[],"unexpected":true}"#.to_owned(),
        )
        .deserialize::<BondedBluetoothDevices>();

        result.expect_err("unknown bonded-device fields should be rejected");
    }

    fn event(payload: &str) -> SppEvent {
        InvokeResponseBody::Json(payload.to_owned())
            .deserialize()
            .expect("payload is a valid SPP event")
    }

    #[test]
    fn spp_events_use_a_tagged_camel_case_contract() {
        let events = [
            event(r#"{"type":"connected"}"#),
            event(r#"{"type":"bytes","data":"JEc="}"#),
            event(r#"{"type":"disconnected","error":"socket closed"}"#),
            event(r#"{"type":"disconnected"}"#),
        ];

        insta::assert_json_snapshot!(events);
    }

    #[test]
    fn spp_events_reject_unknown_fields() {
        let result = InvokeResponseBody::Json(
            r#"{"type":"bytes","data":"JEc=","unexpected":true}"#.to_owned(),
        )
        .deserialize::<SppEvent>();

        assert_err!(result);
    }
}
