use crate::core::AirspaceStatus;
use crate::external_device::PublishedExternalDevice;
use crate::fix::FixTime as CoreFixTime;
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

/// A canonical GPS fix time at the frontend boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum FixTime {
    UtcInstant {
        #[cfg_attr(feature = "ts", ts(type = "number"))]
        unix_milliseconds: i64,
    },
    UtcTimeOfDay {
        milliseconds_since_midnight: u32,
    },
}

impl From<CoreFixTime> for FixTime {
    fn from(value: CoreFixTime) -> Self {
        match value {
            CoreFixTime::UtcInstant(time) => Self::UtcInstant {
                unix_milliseconds: time.unix_milliseconds(),
            },
            CoreFixTime::UtcTimeOfDay(time) => Self::UtcTimeOfDay {
                milliseconds_since_midnight: time.milliseconds_since_midnight(),
            },
        }
    }
}

/// The selected GPS domain at the frontend boundary.
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct GpsInstruments {
    pub position: LatLon,
    pub altitude_meters: Option<f64>,
    pub ground_speed_meters_per_second: Option<f64>,
    pub track_degrees: Option<f64>,
    pub fix_time: Option<FixTime>,
    pub stale: bool,
}

/// An altitude with its freshness state.
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct AltitudeInstrument {
    pub meters: f64,
    pub stale: bool,
}

/// A speed with its freshness state.
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct SpeedInstrument {
    pub meters_per_second: f64,
    pub stale: bool,
}

/// Values that the sensor-fusion estimate derives from selected sensor data.
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct DerivedInstruments {
    pub raw_vertical_speed: Option<SpeedInstrument>,
    pub vertical_speed: Option<SpeedInstrument>,
    pub vario: Option<SpeedInstrument>,
}

/// Fast-changing instrument values grouped by source-selection domain.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct Instruments {
    pub gps: Option<GpsInstruments>,
    pub pressure_altitude: Option<AltitudeInstrument>,
    pub true_airspeed: Option<SpeedInstrument>,
    pub derived: Option<Box<DerivedInstruments>>,
}

impl Instruments {
    pub fn as_topic(&self) -> Topic {
        Topic::Instruments(self.clone())
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
    Airspace(AirspaceStatus),
    Traffic(TrafficUpdate),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{AirspaceLoadError, AirspaceStatus};
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
            gps: Some(GpsInstruments {
                position: LatLon {
                    latitude_degrees: 50.823,
                    longitude_degrees: 6.186,
                },
                altitude_meters: None,
                ground_speed_meters_per_second: Some(45.0),
                track_degrees: Some(270.0),
                fix_time: Some(FixTime::UtcInstant {
                    unix_milliseconds: 1_767_268_800_000,
                }),
                stale: false,
            }),
            pressure_altitude: Some(AltitudeInstrument {
                meters: 1_000.0,
                stale: true,
            }),
            true_airspeed: Some(SpeedInstrument {
                meters_per_second: 50.0,
                stale: false,
            }),
            derived: Some(Box::new(DerivedInstruments {
                raw_vertical_speed: Some(SpeedInstrument {
                    meters_per_second: 1.2,
                    stale: false,
                }),
                vertical_speed: Some(SpeedInstrument {
                    meters_per_second: 1.1,
                    stale: false,
                }),
                vario: Some(SpeedInstrument {
                    meters_per_second: 1.5,
                    stale: true,
                }),
            })),
        }
        .as_topic();

        insta::assert_json_snapshot!(topic);
    }

    #[test]
    fn fix_time_variants_serialize_to_tagged_scalar_values() {
        insta::assert_json_snapshot!([
            FixTime::UtcInstant {
                unix_milliseconds: 1_767_268_800_000,
            },
            FixTime::UtcTimeOfDay {
                milliseconds_since_midnight: 43_201_250,
            },
        ]);
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
    fn airspace_none_status_serializes_to_json() {
        let topic = Topic::Airspace(AirspaceStatus::None);

        insta::assert_json_snapshot!(topic, @r#"
        {
          "topic": "airspace",
          "value": {
            "type": "none"
          }
        }
        "#);
    }

    #[test]
    fn airspace_active_status_serializes_to_json() {
        let topic = Topic::Airspace(AirspaceStatus::Active {
            source_name: Some("Local airspace.txt".into()),
            airspace_count: 42,
            generation: 7,
        });

        insta::assert_json_snapshot!(topic, @r#"
        {
          "topic": "airspace",
          "value": {
            "type": "active",
            "sourceName": "Local airspace.txt",
            "airspaceCount": 42,
            "generation": 7
          }
        }
        "#);
    }

    #[test]
    fn airspace_unavailable_status_serializes_to_json() {
        let topic = Topic::Airspace(AirspaceStatus::Unavailable {
            source_name: None,
            error: AirspaceLoadError::GeometryFailed,
        });

        insta::assert_json_snapshot!(topic, @r#"
        {
          "topic": "airspace",
          "value": {
            "type": "unavailable",
            "sourceName": null,
            "error": "geometryFailed"
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
