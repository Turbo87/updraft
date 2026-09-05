use crate::AirspaceLoadError;
use serde::Serialize;
use std::{collections::BTreeMap, sync::Arc};
use updraft_airspace::AirspaceDataset;

/// Source names are exact display filenames, not filesystem paths.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct AirspaceCatalog {
    pub sources: BTreeMap<String, Result<Arc<AirspaceDataset>, AirspaceLoadError>>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum AirspaceSourceStatus {
    Active {
        source_name: String,
        airspace_count: usize,
    },
    Unavailable {
        source_name: String,
        error: AirspaceLoadError,
    },
}

impl AirspaceCatalog {
    /// Lists each source in filename order without copying geometry.
    pub fn source_statuses(&self) -> Vec<AirspaceSourceStatus> {
        self.sources
            .iter()
            .map(|(name, source)| match source {
                Ok(dataset) => AirspaceSourceStatus::Active {
                    source_name: name.clone(),
                    airspace_count: dataset.airspaces().len(),
                },
                Err(error) => AirspaceSourceStatus::Unavailable {
                    source_name: name.clone(),
                    error: *error,
                },
            })
            .collect()
    }
}
