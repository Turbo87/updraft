mod openair;
mod types;

use std::sync::Arc;

pub use types::{
    Airspace, AirspaceAltitude, AirspaceClass, AirspaceDataset, AirspaceGeometryError, AirspaceId,
    AirspaceImportError, AirspaceLoadError, AirspaceParseError, AirspacePolygon, AirspaceStatus,
    AirspaceType,
};

/// Owns one valid core airspace state and its process-local generation.
#[derive(Debug)]
pub enum AirspaceState {
    None {
        generation: u32,
    },
    Active {
        dataset: Arc<AirspaceDataset>,
        source_name: Option<String>,
        generation: u32,
    },
    Unavailable {
        source_name: Option<String>,
        error: AirspaceLoadError,
        generation: u32,
    },
}

impl Default for AirspaceState {
    fn default() -> Self {
        Self::None { generation: 0 }
    }
}

impl AirspaceState {
    pub fn status(&self) -> AirspaceStatus {
        match self {
            Self::None { .. } => AirspaceStatus::None,
            Self::Active {
                dataset,
                source_name,
                generation,
            } => AirspaceStatus::Active {
                source_name: source_name.clone(),
                airspace_count: dataset.airspaces().len(),
                generation: *generation,
            },
            Self::Unavailable {
                source_name, error, ..
            } => AirspaceStatus::Unavailable {
                source_name: source_name.clone(),
                error: *error,
            },
        }
    }

    pub fn activate(
        &mut self,
        dataset: Arc<AirspaceDataset>,
        source_name: Option<String>,
    ) -> AirspaceStatus {
        let generation = self.generation_for_dataset(Some(&dataset));
        *self = Self::Active {
            dataset,
            source_name,
            generation,
        };
        self.status()
    }

    pub fn clear(&mut self) -> AirspaceStatus {
        let generation = self.generation_for_dataset(None);
        *self = Self::None { generation };
        self.status()
    }

    pub fn mark_unavailable(
        &mut self,
        source_name: Option<String>,
        error: AirspaceLoadError,
    ) -> AirspaceStatus {
        let generation = self.generation_for_dataset(None);
        *self = Self::Unavailable {
            source_name,
            error,
            generation,
        };
        self.status()
    }

    pub fn snapshot(&self) -> Option<Arc<AirspaceDataset>> {
        let Self::Active { dataset, .. } = self else {
            return None;
        };
        Some(dataset.clone())
    }

    /// Returns the generation for a replacement dataset identity.
    fn generation_for_dataset(&self, replacement: Option<&Arc<AirspaceDataset>>) -> u32 {
        let (current, generation) = match self {
            Self::None { generation } | Self::Unavailable { generation, .. } => (None, *generation),
            Self::Active {
                dataset,
                generation,
                ..
            } => (Some(dataset), *generation),
        };
        let unchanged = match (current, replacement) {
            (None, None) => true,
            (Some(current), Some(replacement)) => Arc::ptr_eq(current, replacement),
            _ => false,
        };

        if unchanged {
            generation
        } else {
            generation.wrapping_add(1)
        }
    }
}
