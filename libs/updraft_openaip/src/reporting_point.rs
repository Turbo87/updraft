//! The `rpp` dataset.

use crate::common::{Countries, Elevation, ElevationGeoid, Point};
use serde::Deserialize;

/// One reporting point record.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportingPoint {
    #[serde(rename = "_id")]
    pub id: Box<str>,
    pub name: Box<str>,
    /// A report at this point is compulsory.
    pub compulsory: bool,
    pub country: Countries,
    pub geometry: Point,
    pub elevation: Elevation,
    pub elevation_geoid: Option<ElevationGeoid>,
    /// The OpenAIP identifiers of the airports that use this point.
    #[serde(default)]
    pub airports: Vec<Box<str>>,
    pub remarks: Option<Box<str>>,
    pub created_at: Box<str>,
    pub created_by: Box<str>,
    pub updated_at: Box<str>,
    pub updated_by: Box<str>,
}
