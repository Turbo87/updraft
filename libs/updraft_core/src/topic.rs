use crate::external_device::PublishedExternalDevice;
use crate::settings::Settings;
use crate::traffic::TrafficUpdate;
use serde::Serialize;

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct LatLon {
    pub latitude_degrees: f64,
    pub longitude_degrees: f64,
}

/// Fast-changing instrument values.
///
/// Every field is SI and names its unit. Conversion to display units and
/// formatting belong to the frontend.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct Instruments {
    pub position: Option<LatLon>,
    pub track_degrees: Option<f64>,
    pub ground_speed_meters_per_second: Option<f64>,
    pub altitude_msl_meters: Option<f64>,
}

impl Instruments {
    pub fn as_topic(&self) -> Topic {
        Topic::Instruments(*self)
    }
}

/// One group of client-visible state.
///
/// Topics are grouped by how often they change, so a fast instrument
/// update does not pay to re-serialize slow-changing state.
/// Most topics carry complete state. Traffic sends a complete onboarding
/// snapshot and then sends deltas.
///
/// Adjacently tagged so the wire form is `{ topic, value }` in both JSON
/// and the generated TypeScript. An internally tagged enum would generate
/// an intersection type, which is awkward to narrow on in the frontend.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "topic", content = "value", rename_all = "camelCase")]
pub enum Topic {
    Instruments(Instruments),
    Settings(Settings),
    ExternalDevices(Vec<PublishedExternalDevice>),
    Traffic(TrafficUpdate),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::Locale;
    use crate::traffic::{
        PublishedTrafficTarget, TrafficAlarmLevel, TrafficDelta, TrafficTarget, TrafficTargetId,
        TrafficTargetIdType, TrafficType, TrafficUpdate,
    };
    use updraft_geo::LatLon as GeoLatLon;
    use updraft_units::{Angle, Length, MslAltitude};

    fn published_target(id_type: TrafficTargetIdType) -> PublishedTrafficTarget {
        TrafficTarget {
            id: TrafficTargetId {
                id_type,
                value: 0xABC123,
            },
            position: GeoLatLon::from_degrees(50.832, 6.189),
            altitude_msl: Some(MslAltitude::new(Length::from_meters(350.0))),
            traffic_type: TrafficType::Glider,
            track: Some(Angle::from_degrees(90.0)),
            alarm_level: TrafficAlarmLevel::Low,
            stale: false,
        }
        .into()
    }

    #[test]
    fn topic_serializes_to_tagged_camel_case_json() {
        let topic = Instruments {
            position: Some(LatLon {
                latitude_degrees: 50.823,
                longitude_degrees: 6.186,
            }),
            track_degrees: Some(270.0),
            ground_speed_meters_per_second: Some(45.0),
            altitude_msl_meters: None,
        }
        .as_topic();

        insta::assert_json_snapshot!(topic);
    }

    #[test]
    fn settings_locale_serializes_in_lowercase() {
        let topic = Settings {
            locale: Some(Locale::De),
            ..Settings::default()
        }
        .as_topic();

        insta::assert_json_snapshot!(topic, @r#"
        {
          "topic": "settings",
          "value": {
            "locale": "de",
            "units": {
              "altitude": "m",
              "distance": "km",
              "speed": "km/h",
              "verticalSpeed": "m/s"
            }
          }
        }
        "#);
    }

    #[test]
    fn empty_traffic_snapshot_serializes_to_json() {
        let topic = Topic::Traffic(TrafficUpdate::Snapshot(Vec::new()));

        insta::assert_json_snapshot!(topic);
    }

    #[test]
    fn non_empty_traffic_snapshot_serializes_typed_values_as_scalars() {
        let topic = Topic::Traffic(TrafficUpdate::Snapshot(vec![published_target(
            TrafficTargetIdType::Flarm,
        )]));

        insta::assert_json_snapshot!(topic);
    }

    #[test]
    fn traffic_delta_serializes_complete_upserts_and_id_removals() {
        let topic = Topic::Traffic(TrafficUpdate::Delta(TrafficDelta {
            upserts: vec![published_target(TrafficTargetIdType::Flarm)],
            removed: vec!["icao:DEF456".into()],
        }));

        insta::assert_json_snapshot!(topic);
    }

    #[test]
    fn other_traffic_target_id_type_serializes_its_value() {
        let topic = Topic::Traffic(TrafficUpdate::Snapshot(vec![published_target(
            TrafficTargetIdType::Other(7),
        )]));

        insta::assert_json_snapshot!(topic);
    }
}
