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
    pub id: String,
    pub name: String,
    pub r#type: ObstacleType,
    pub country: Countries,
    pub geometry: Point,
    pub elevation: Elevation,
    pub elevation_geoid: Option<ElevationGeoid>,
    /// The height above ground.
    pub height: Option<Height>,
    pub osm_id: String,
    /// The `OpenStreetMap` tags of the source node.
    #[serde(default)]
    pub osm_tags: BTreeMap<String, String>,
    pub osm_import_job_id: String,
    /// The last `OpenStreetMap` update as an RFC 3339 timestamp.
    pub osm_updated_at: String,
    pub created_at: String,
    pub created_by: String,
    pub updated_at: String,
    pub updated_by: String,
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
