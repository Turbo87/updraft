use super::storage::WaypointStorage;
use crate::{driver::DriverHandle, file_picker::FileBytesPickerState};
use serde::Serialize;
use std::sync::Arc;
use tokio::sync::Mutex;
use updraft_core::{GetWaypointCatalog, ReplaceWaypointCatalog};
use updraft_waypoint::WaypointImportError;

pub struct WaypointCommandState {
    storage: WaypointStorage,
    mutation: Mutex<()>,
}

impl WaypointCommandState {
    pub fn new(storage: WaypointStorage) -> Self {
        Self {
            storage,
            mutation: Mutex::new(()),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum ImportWaypointsResult {
    Imported { source_name: String },
    Cancelled,
}

#[derive(Debug, Serialize, thiserror::Error)]
#[serde(rename_all = "camelCase")]
#[error("{self:?}")]
pub enum WaypointCommandError {
    Busy,
    ReadFailed,
    MissingName,
    ParseFailed,
    StorageFailed,
    DriverStopped,
    WorkerFailed,
}

#[tauri::command]
pub async fn import_waypoints(
    state: tauri::State<'_, WaypointCommandState>,
    picker: tauri::State<'_, FileBytesPickerState>,
    handle: tauri::State<'_, DriverHandle>,
) -> Result<ImportWaypointsResult, WaypointCommandError> {
    let _guard = state
        .mutation
        .try_lock()
        .map_err(|_| WaypointCommandError::Busy)?;
    let selected = picker
        .pick_file_bytes()
        .await
        .map_err(|_| WaypointCommandError::ReadFailed)?;
    let Some(selected) = selected else {
        return Ok(ImportWaypointsResult::Cancelled);
    };
    let name = selected
        .display_name
        .filter(|name| !name.is_empty())
        .ok_or(WaypointCommandError::MissingName)?;
    let catalog = handle
        .send(GetWaypointCatalog)
        .await
        .map_err(|_| WaypointCommandError::DriverStopped)?;
    let storage = state.storage.clone();
    let source_name = name.clone();
    let (dataset, change) =
        tokio::task::spawn_blocking(move || storage.import(&source_name, &selected.bytes))
            .await
            .map_err(|_| WaypointCommandError::WorkerFailed)?
            .map_err(|error| {
                if error.downcast_ref::<WaypointImportError>().is_some() {
                    WaypointCommandError::ParseFailed
                } else {
                    tracing::warn!(%error, "Could not store waypoint source");
                    WaypointCommandError::StorageFailed
                }
            })?;
    let mut replacement = (*catalog).clone();
    replacement.sources.insert(name.clone(), Ok(dataset));
    if handle
        .send(ReplaceWaypointCatalog(Arc::new(replacement)))
        .await
        .is_err()
    {
        tokio::task::spawn_blocking(move || change.rollback()).await
            .map_err(|_| WaypointCommandError::WorkerFailed)?
            .map_err(|error| {
                tracing::error!(%error, "Could not restore waypoint source after activation failed");
                WaypointCommandError::StorageFailed
            })?;
        return Err(WaypointCommandError::DriverStopped);
    }
    Ok(ImportWaypointsResult::Imported { source_name: name })
}

#[tauri::command]
pub async fn remove_waypoints(
    source_name: String,
    state: tauri::State<'_, WaypointCommandState>,
    handle: tauri::State<'_, DriverHandle>,
) -> Result<(), WaypointCommandError> {
    let _guard = state
        .mutation
        .try_lock()
        .map_err(|_| WaypointCommandError::Busy)?;
    let catalog = handle
        .send(GetWaypointCatalog)
        .await
        .map_err(|_| WaypointCommandError::DriverStopped)?;
    let mut replacement = (*catalog).clone();
    if replacement.sources.remove(&source_name).is_none() {
        return Ok(());
    }
    let storage = state.storage.clone();
    let change = tokio::task::spawn_blocking(move || storage.remove(&source_name))
        .await
        .map_err(|_| WaypointCommandError::WorkerFailed)?
        .map_err(|error| {
            tracing::warn!(%error, "Could not remove waypoint source");
            WaypointCommandError::StorageFailed
        })?;
    if handle
        .send(ReplaceWaypointCatalog(Arc::new(replacement)))
        .await
        .is_err()
    {
        tokio::task::spawn_blocking(move || change.rollback())
            .await
            .map_err(|_| WaypointCommandError::WorkerFailed)?
            .map_err(|error| {
                tracing::error!(%error, "Could not restore waypoint source after removal failed");
                WaypointCommandError::StorageFailed
            })?;
        return Err(WaypointCommandError::DriverStopped);
    }
    Ok(())
}

#[cfg(test)]
mod tests;
