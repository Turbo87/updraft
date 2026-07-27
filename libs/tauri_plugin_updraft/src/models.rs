use serde::{Deserialize, Serialize};

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
/// Everything but the position is optional: a receiver can have a position
/// without yet having a track, a speed or an altitude, and reporting a
/// placeholder for one of those would be indistinguishable from a real
/// reading.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Fix {
    pub latitude_degrees: f64,
    pub longitude_degrees: f64,
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

#[cfg(test)]
mod tests {
    use super::SppEvent;
    use tauri::ipc::InvokeResponseBody;

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

        assert!(result.is_err());
    }
}
