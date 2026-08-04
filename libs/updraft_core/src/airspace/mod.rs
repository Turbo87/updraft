mod openair;
mod types;

use std::sync::Arc;

pub use types::{
    Airspace, AirspaceAltitude, AirspaceClass, AirspaceDataset, AirspaceGeometryError, AirspaceId,
    AirspaceImportError, AirspaceLoadError, AirspaceParseError, AirspaceStatus, AirspaceType,
};

/// Owns one valid core airspace state and its process-local generation.
#[derive(Debug)]
pub struct AirspaceState {
    value: AirspaceStateValue,
    generation: u32,
}

#[derive(Debug)]
enum AirspaceStateValue {
    None,
    Active {
        dataset: Arc<AirspaceDataset>,
        source_name: Option<String>,
    },
    Unavailable {
        source_name: Option<String>,
        error: AirspaceLoadError,
    },
}

impl Default for AirspaceState {
    fn default() -> Self {
        Self {
            value: AirspaceStateValue::None,
            generation: 0,
        }
    }
}

impl AirspaceState {
    /// Creates an empty startup state with generation zero.
    pub fn none_at_startup() -> Self {
        Self::default()
    }

    /// Creates an active startup state with generation zero.
    pub fn active_at_startup(dataset: Arc<AirspaceDataset>, source_name: Option<String>) -> Self {
        Self {
            value: AirspaceStateValue::Active {
                dataset,
                source_name,
            },
            generation: 0,
        }
    }

    /// Creates an unavailable startup state with generation zero.
    pub fn unavailable_at_startup(source_name: Option<String>, error: AirspaceLoadError) -> Self {
        Self {
            value: AirspaceStateValue::Unavailable { source_name, error },
            generation: 0,
        }
    }

    /// Returns the client-visible airspace state without geometry.
    pub fn status(&self) -> AirspaceStatus {
        match &self.value {
            AirspaceStateValue::None => AirspaceStatus::None,
            AirspaceStateValue::Active {
                dataset,
                source_name,
            } => AirspaceStatus::Active {
                source_name: source_name.clone(),
                airspace_count: dataset.airspaces().len(),
                generation: self.generation,
            },
            AirspaceStateValue::Unavailable { source_name, error } => AirspaceStatus::Unavailable {
                source_name: source_name.clone(),
                error: *error,
            },
        }
    }

    /// Replaces the active dataset and updates its process-local generation.
    pub fn activate(
        &mut self,
        dataset: Arc<AirspaceDataset>,
        source_name: Option<String>,
    ) -> AirspaceStatus {
        self.update_generation_for_dataset(Some(&dataset));
        self.value = AirspaceStateValue::Active {
            dataset,
            source_name,
        };
        self.status()
    }

    /// Removes the active dataset and retains the process-local generation.
    pub fn clear(&mut self) -> AirspaceStatus {
        self.update_generation_for_dataset(None);
        self.value = AirspaceStateValue::None;
        self.status()
    }

    /// Removes the active dataset and records a safe startup load error.
    pub fn mark_unavailable(
        &mut self,
        source_name: Option<String>,
        error: AirspaceLoadError,
    ) -> AirspaceStatus {
        self.update_generation_for_dataset(None);
        self.value = AirspaceStateValue::Unavailable { source_name, error };
        self.status()
    }

    /// Returns a shared immutable snapshot of the active dataset.
    pub fn snapshot(&self) -> Option<Arc<AirspaceDataset>> {
        let AirspaceStateValue::Active { dataset, .. } = &self.value else {
            return None;
        };
        Some(dataset.clone())
    }

    /// Updates the generation for a replacement dataset identity.
    fn update_generation_for_dataset(&mut self, replacement: Option<&Arc<AirspaceDataset>>) {
        let current = match &self.value {
            AirspaceStateValue::None | AirspaceStateValue::Unavailable { .. } => None,
            AirspaceStateValue::Active { dataset, .. } => Some(dataset),
        };
        let unchanged = match (current, replacement) {
            (None, None) => true,
            (Some(current), Some(replacement)) => Arc::ptr_eq(current, replacement),
            _ => false,
        };

        if !unchanged {
            self.generation = self.generation.wrapping_add(1);
        }
    }
}
