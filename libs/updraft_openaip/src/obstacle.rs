//! The `obs` dataset.

use crate::code::codes;
use crate::common::{Countries, Elevation, ElevationGeoid, Point, VerticalDatum, VerticalUnit};
use serde::Deserialize;
use std::collections::BTreeMap;

/// One obstacle record.
///
/// OpenAIP imports obstacles from `OpenStreetMap`. Every record therefore keeps
/// its `OpenStreetMap` origin.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Obstacle {
    #[serde(rename = "_id")]
    pub id: Box<str>,
    pub name: Box<str>,
    pub r#type: ObstacleType,
    pub country: Countries,
    pub geometry: Point,
    pub elevation: Elevation,
    pub elevation_geoid: Option<ElevationGeoid>,
    /// The height above ground.
    pub height: Option<Height>,
    pub osm_id: Box<str>,
    /// The `OpenStreetMap` tags of the source node.
    #[serde(default)]
    pub osm_tags: BTreeMap<Box<str>, Box<str>>,
    pub osm_import_job_id: Box<str>,
    /// The last `OpenStreetMap` update as an RFC 3339 timestamp.
    pub osm_updated_at: Box<str>,
    pub created_at: Box<str>,
    pub created_by: Box<str>,
    pub updated_at: Box<str>,
    pub updated_by: Box<str>,
}

/// The height of an obstacle above ground.
#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Height {
    pub value: f64,
    pub unit: VerticalUnit,
    pub reference_datum: VerticalDatum,
}

codes! {
    /// The obstacle type.
    pub enum ObstacleType {
        0 => Obstacle,
        1 => Chimney,
        2 => Building,
        3 => WindTurbine,
        4 => Tower,
    }
}
