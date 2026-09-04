use serde::Serialize;
use std::{collections::BTreeMap, sync::Arc};
use updraft_waypoint::WaypointDataset;

/// Source names are exact display names, not filesystem paths.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WaypointCatalog {
    pub sources: BTreeMap<String, Result<Arc<WaypointDataset>, WaypointLoadError>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub enum WaypointLoadError {
    ParseFailed,
    ReadFailed,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct WaypointDiagnostic {
    pub line: Option<u32>,
    pub message: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum WaypointSourceStatus {
    Active {
        source_name: String,
        waypoint_count: usize,
        warnings: Vec<WaypointDiagnostic>,
    },
    Unavailable {
        source_name: String,
        error: WaypointLoadError,
    },
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct WaypointStatus {
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub generation: u64,
    pub sources: Vec<WaypointSourceStatus>,
}

impl WaypointCatalog {
    pub fn status(&self, generation: u64) -> WaypointStatus {
        let sources = self
            .sources
            .iter()
            .map(|(name, source)| match source {
                Ok(dataset) => WaypointSourceStatus::Active {
                    source_name: name.clone(),
                    waypoint_count: dataset.waypoints().len(),
                    warnings: dataset
                        .warnings()
                        .iter()
                        .map(|warning| WaypointDiagnostic {
                            line: warning.line.and_then(|line| line.try_into().ok()),
                            message: warning.message.clone(),
                        })
                        .collect(),
                },
                Err(error) => WaypointSourceStatus::Unavailable {
                    source_name: name.clone(),
                    error: *error,
                },
            })
            .collect();
        WaypointStatus {
            generation,
            sources,
        }
    }
}
