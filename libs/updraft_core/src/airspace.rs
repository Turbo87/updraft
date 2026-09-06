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

/// A safe machine-readable failure from loading a stored airspace source.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub enum AirspaceLoadError {
    ReadFailed,
    ParseFailed,
    GeometryFailed,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct AirspaceStatus {
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub generation: u64,
    pub sources: Vec<AirspaceSourceStatus>,
}

impl AirspaceCatalog {
    pub fn status(&self, generation: u64) -> AirspaceStatus {
        AirspaceStatus {
            generation,
            sources: self.source_statuses(),
        }
    }
}

/// The catalog and its process-local generation from one core query.
#[derive(Clone, Debug)]
pub struct AirspaceSnapshot {
    pub generation: u64,
    pub catalog: Arc<AirspaceCatalog>,
}

/// Owns immutable source datasets and their process-local generation.
#[derive(Debug, Default)]
pub struct AirspaceState {
    catalog: Arc<AirspaceCatalog>,
    generation: u64,
}

impl AirspaceState {
    pub fn none_at_startup() -> Self {
        Self::default()
    }

    pub fn at_startup(catalog: AirspaceCatalog) -> Self {
        Self {
            catalog: Arc::new(catalog),
            generation: 0,
        }
    }

    pub fn active_at_startup(dataset: Arc<AirspaceDataset>, source_name: Option<String>) -> Self {
        Self::at_startup(AirspaceCatalog {
            sources: BTreeMap::from([(
                source_name.unwrap_or_else(|| "airspace.txt".into()),
                Ok(dataset),
            )]),
        })
    }

    pub fn unavailable_at_startup(source_name: Option<String>, error: AirspaceLoadError) -> Self {
        Self::at_startup(AirspaceCatalog {
            sources: BTreeMap::from([(
                source_name.unwrap_or_else(|| "airspace.txt".into()),
                Err(error),
            )]),
        })
    }

    pub fn status(&self) -> AirspaceStatus {
        self.catalog.status(self.generation)
    }

    pub fn replace(&mut self, catalog: Arc<AirspaceCatalog>) -> AirspaceStatus {
        self.catalog = catalog;
        self.generation += 1;
        self.status()
    }

    pub fn activate(
        &mut self,
        dataset: Arc<AirspaceDataset>,
        source_name: Option<String>,
    ) -> AirspaceStatus {
        self.replace(Self::active_at_startup(dataset, source_name).catalog)
    }

    pub fn clear(&mut self) -> AirspaceStatus {
        self.replace(Arc::default())
    }

    pub fn mark_unavailable(
        &mut self,
        source_name: Option<String>,
        error: AirspaceLoadError,
    ) -> AirspaceStatus {
        self.replace(Self::unavailable_at_startup(source_name, error).catalog)
    }

    pub fn snapshot(&self) -> AirspaceSnapshot {
        AirspaceSnapshot {
            generation: self.generation,
            catalog: self.catalog.clone(),
        }
    }
}
