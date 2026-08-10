//! The `nav` dataset.

use crate::code::codes;
use crate::common::{
    Countries, Elevation, ElevationGeoid, FrequencyUnit, HoursOfOperation, Image, Point,
};
use serde::Deserialize;

/// One navaid record.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Navaid {
    #[serde(rename = "_id")]
    pub id: String,
    pub name: String,
    /// The published identifier, for example `TRA`.
    pub identifier: String,
    pub r#type: NavaidType,
    pub country: Countries,
    pub geometry: Point,
    pub elevation: Elevation,
    pub elevation_geoid: Option<ElevationGeoid>,
    /// The magnetic declination at the site in degrees.
    pub magnetic_declination: f64,
    /// The navaid is aligned with true north instead of magnetic north.
    pub aligned_true_north: bool,
    pub frequency: Frequency,
    /// The paired VHF channel of a TACAN or DME station.
    pub channel: Option<String>,
    pub range: Option<Range>,
    pub hours_of_operation: HoursOfOperation,
    #[serde(default)]
    pub images: Vec<Image>,
    pub remarks: Option<String>,
    pub created_at: String,
    pub created_by: String,
    pub updated_at: String,
    pub updated_by: String,
}

/// The frequency of a navaid.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct Frequency {
    pub value: String,
    pub unit: FrequencyUnit,
}

/// The range of a navaid.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
pub struct Range {
    pub value: i32,
    pub unit: RangeUnit,
}

codes! {
    /// The unit of a navaid range.
    pub enum RangeUnit {
        2 => NauticalMile,
    }
}

codes! {
    /// The navaid type.
    pub enum NavaidType {
        /// Distance measuring equipment.
        0 => Dme,
        /// Tactical air navigation system.
        1 => Tacan,
        /// Non-directional beacon.
        2 => Ndb,
        /// VHF omnidirectional range.
        3 => Vor,
        4 => VorDme,
        5 => Vortac,
        /// Doppler VHF omnidirectional range.
        6 => Dvor,
        7 => DvorDme,
        8 => Dvortac,
    }
}
