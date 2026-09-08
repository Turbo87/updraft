use crate::{
    driver::DriverHandle,
    file_picker::{FileBytesPickerState, PickedFileBytes},
    ipc::{AirspaceCommandError, AirspaceCommandState},
    waypoints::commands::{WaypointCommandError, WaypointCommandState},
};
use serde::Serialize;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::sync::Mutex;

#[derive(Default)]
pub struct DataImportState {
    selection: Mutex<Option<Selection>>,
    next_id: AtomicU64,
}

struct Selection {
    info: SelectedDataFile,
    file: PickedFileBytes,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SelectedDataFile {
    selection_id: String,
    source_name: String,
    data_type: DataType,
}

#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
enum DataType {
    Airspace,
    Waypoints,
}

#[derive(Debug, Serialize, thiserror::Error)]
#[serde(tag = "kind", content = "error", rename_all = "camelCase")]
#[error("{self:?}")]
pub enum DataImportError {
    Busy,
    ReadFailed,
    MissingName,
    UnsupportedType,
    NoSelection,
    Airspace(#[from] AirspaceCommandError),
    Waypoints(#[from] WaypointCommandError),
}

#[tauri::command]
pub async fn select_data_file(
    state: tauri::State<'_, DataImportState>,
    picker: tauri::State<'_, FileBytesPickerState>,
) -> Result<Option<SelectedDataFile>, DataImportError> {
    let mut selection = state
        .selection
        .try_lock()
        .map_err(|_| DataImportError::Busy)?;
    *selection = None;
    let Some(file) = picker.pick_file_bytes().await.map_err(|error| {
        tracing::warn!(%error, "Could not select data file");
        DataImportError::ReadFailed
    })?
    else {
        return Ok(None);
    };
    let name = file
        .display_name
        .as_deref()
        .filter(|name| !name.is_empty())
        .ok_or(DataImportError::MissingName)?;
    let extension = std::path::Path::new(name)
        .extension()
        .and_then(|ext| ext.to_str());
    let data_type = match extension.map(str::to_ascii_lowercase).as_deref() {
        Some("txt") => DataType::Airspace,
        Some("cup") => DataType::Waypoints,
        _ => return Err(DataImportError::UnsupportedType),
    };
    let info = SelectedDataFile {
        selection_id: state.next_id.fetch_add(1, Ordering::Relaxed).to_string(),
        source_name: name.to_owned(),
        data_type,
    };
    *selection = Some(Selection {
        info: info.clone(),
        file,
    });
    Ok(Some(info))
}

#[tauri::command]
pub async fn discard_data_file(
    selection_id: String,
    state: tauri::State<'_, DataImportState>,
) -> Result<(), DataImportError> {
    let mut selection = state
        .selection
        .try_lock()
        .map_err(|_| DataImportError::Busy)?;
    if selection
        .as_ref()
        .is_some_and(|s| s.info.selection_id == selection_id)
    {
        *selection = None;
    }
    Ok(())
}

#[tauri::command]
pub async fn import_data_file(
    selection_id: String,
    state: tauri::State<'_, DataImportState>,
    airspace: tauri::State<'_, AirspaceCommandState>,
    waypoints: tauri::State<'_, WaypointCommandState>,
    handle: tauri::State<'_, DriverHandle>,
) -> Result<SelectedDataFile, DataImportError> {
    let mut selection = state
        .selection
        .try_lock()
        .map_err(|_| DataImportError::Busy)?;
    let selected = selection
        .take_if(|s| s.info.selection_id == selection_id)
        .ok_or(DataImportError::NoSelection)?;
    match selected.info.data_type {
        DataType::Airspace => {
            airspace.import_selected(selected.file, &handle).await?;
        }
        DataType::Waypoints => {
            waypoints.import_selected(selected.file, &handle).await?;
        }
    }
    Ok(selected.info)
}

#[cfg(test)]
mod tests;
