//! The `hot` dataset.

use crate::code::codes;
use crate::common::{Countries, Elevation, ElevationGeoid, Point, WindDirection};
use serde::Deserialize;

/// One thermal hotspot record.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Hotspot {
    #[serde(rename = "_id")]
    pub id: String,
    pub name: String,
    pub r#type: HotspotType,
    pub reliability: Reliability,
    pub occurrence: Occurrence,
    /// Raw category codes. The API documentation does not describe them.
    #[serde(default)]
    pub category: Vec<u16>,
    pub country: Countries,
    pub geometry: Point,
    pub elevation: Elevation,
    pub elevation_geoid: Option<ElevationGeoid>,
    /// The times of day at which the hotspot can work.
    #[serde(default)]
    pub time_of_day: Vec<TimeOfDay>,
    /// The times of day at which the hotspot works best.
    #[serde(default)]
    pub fav_time_of_day: Vec<TimeOfDay>,
    /// The wind directions in which the hotspot works best.
    #[serde(default)]
    pub fav_wind_direction: Vec<WindDirection>,
    /// The wind directions that the hotspot needs to work.
    #[serde(default)]
    pub req_wind_direction: Vec<WindDirection>,
    pub remarks: Option<String>,
    pub created_at: String,
    pub created_by: String,
    pub updated_at: String,
    pub updated_by: String,
}

codes! {
    /// The hotspot type.
    pub enum HotspotType {
        0 => Natural,
        1 => Artificial,
    }
}

codes! {
    /// How reliable the hotspot is.
    pub enum Reliability {
        0 => Poor,
        1 => Fair,
        2 => High,
        3 => VeryHigh,
    }
}

codes! {
    /// How often the hotspot occurs.
    pub enum Occurrence {
        0 => IrregularIntervals,
        1 => ScheduledInterval,
        2 => NearlyConstant,
    }
}

codes! {
    /// A time of day.
    pub enum TimeOfDay {
        0 => EarlyMorning,
        1 => Morning,
        2 => Noon,
        3 => Afternoon,
        4 => Evening,
    }
}
