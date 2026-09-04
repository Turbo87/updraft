//! Canonical waypoint data imported from CUP sources.

use seeyou_cup::CupFile;
use updraft_geo::LatLon;

pub use seeyou_cup::WaypointStyle as WaypointKind;

#[derive(Clone, Debug, PartialEq)]
pub struct Waypoint {
    pub name: String,
    pub position: LatLon,
    pub kind: WaypointKind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WaypointWarning {
    pub line: Option<u64>,
    pub message: String,
}

/// One source's accepted waypoints and import diagnostics.
#[derive(Clone, Debug, PartialEq)]
pub struct WaypointDataset {
    waypoints: Vec<Waypoint>,
    warnings: Vec<WaypointWarning>,
}

#[derive(Debug, thiserror::Error)]
pub enum WaypointImportError {
    #[error(transparent)]
    Cup(#[from] seeyou_cup::Error),
    #[error("The file contains no valid waypoints")]
    Empty,
}

impl WaypointDataset {
    /// Imports the waypoints and warnings from a CUP file.
    pub fn from_cup(bytes: &[u8]) -> Result<Self, WaypointImportError> {
        let (cup, warnings) = CupFile::from_reader(bytes)?;
        if cup.waypoints.is_empty() {
            return Err(WaypointImportError::Empty);
        }
        let waypoints = cup.waypoints.into_iter().map(Waypoint::from).collect();
        let warnings = warnings
            .into_iter()
            .map(|warning| WaypointWarning {
                line: warning.line(),
                message: warning.message().to_owned(),
            })
            .collect();
        Ok(Self {
            waypoints,
            warnings,
        })
    }

    pub fn waypoints(&self) -> &[Waypoint] {
        &self.waypoints
    }

    pub fn warnings(&self) -> &[WaypointWarning] {
        &self.warnings
    }
}

impl From<seeyou_cup::Waypoint> for Waypoint {
    fn from(point: seeyou_cup::Waypoint) -> Self {
        Self {
            name: point.name,
            position: LatLon::from_degrees(point.latitude, point.longitude),
            kind: point.style,
        }
    }
}
