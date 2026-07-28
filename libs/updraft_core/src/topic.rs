use crate::settings::Settings;
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

/// One group of client-visible state, sent whole rather than as a delta.
///
/// Topics are grouped by how often they change, so a fast instrument
/// update does not pay to re-serialize slow-changing state.
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::Locale;

    #[test]
    fn topic_serializes_to_tagged_camel_case_json() {
        let topic = Topic::Instruments(Instruments {
            position: Some(LatLon {
                latitude_degrees: 50.823,
                longitude_degrees: 6.186,
            }),
            track_degrees: Some(270.0),
            ground_speed_meters_per_second: Some(45.0),
            altitude_msl_meters: None,
        });

        insta::assert_json_snapshot!(topic);
    }

    #[test]
    fn settings_locale_serializes_in_lowercase() {
        let topic = Topic::Settings(Settings {
            locale: Some(Locale::De),
        });

        insta::assert_json_snapshot!(topic, @r###"
        {
          "topic": "settings",
          "value": {
            "locale": "de"
          }
        }
        "###);
    }
}
