//! The `rpp` dataset.

use crate::common::{Countries, Elevation, ElevationGeoid, Point};
use serde::Deserialize;

/// One reporting point record.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportingPoint {
    #[serde(rename = "_id")]
    pub id: String,
    pub name: String,
    /// A report at this point is compulsory.
    pub compulsory: bool,
    pub country: Countries,
    pub geometry: Point,
    pub elevation: Elevation,
    pub elevation_geoid: Option<ElevationGeoid>,
    /// The OpenAIP identifiers of the airports that use this point.
    #[serde(default)]
    pub airports: Vec<String>,
    pub remarks: Option<String>,
    pub created_at: String,
    pub created_by: String,
    pub updated_at: String,
    pub updated_by: String,
}
