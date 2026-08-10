//! The `hgl` dataset.

use crate::code::codes;
use crate::common::{Countries, Elevation, ElevationGeoid, Image, Point, WindDirection};
use serde::Deserialize;

/// One hang gliding site record.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HangGlidingSite {
    #[serde(rename = "_id")]
    pub id: Box<str>,
    pub name: Box<str>,
    pub r#type: HangGlidingType,
    /// Raw category codes. The API documentation does not describe them.
    #[serde(default)]
    pub category: Vec<u16>,
    /// Raw access codes. The API documentation does not describe them.
    #[serde(default)]
    pub access: Vec<u16>,
    /// The site is officially certified.
    pub certified: bool,
    /// The suitable wind directions. Other directions are unfavourable or
    /// dangerous.
    #[serde(default)]
    pub suitable_wind_direction: Vec<WindDirection>,
    pub country: Countries,
    pub geometry: Point,
    pub elevation: Elevation,
    pub elevation_geoid: Option<ElevationGeoid>,
    #[serde(default)]
    pub images: Vec<Image>,
    pub remarks: Option<Box<str>>,
    pub created_at: Box<str>,
    pub created_by: Box<str>,
    pub updated_at: Box<str>,
    pub updated_by: Box<str>,
}

codes! {
    /// The purpose of a hang gliding site.
    pub enum HangGlidingType {
        0 => TakeOff,
        1 => Landing,
    }
}
