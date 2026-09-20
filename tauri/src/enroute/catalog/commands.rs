use super::{CatalogService, CatalogStatus};
use crate::basemap::Basemaps;
use crate::enroute::subscriptions::StatusChannels;
use crate::terrain::Terrain;
use std::sync::{Arc, Mutex};
use tauri::ipc::Channel;

pub type CatalogSubscriptions = StatusChannels<CatalogStatus>;

#[tauri::command(async)]
pub fn subscribe_enroute_catalog(
    channel: Channel<CatalogStatus>,
    state: tauri::State<'_, CatalogSubscriptions>,
) -> Result<(), &'static str> {
    state
        .subscribe(channel)
        .map_err(|_| "Could not subscribe to catalog status")
}

#[tauri::command(async)]
pub fn unsubscribe_enroute_catalog(channel_id: u32, state: tauri::State<'_, CatalogSubscriptions>) {
    state.unsubscribe(channel_id);
}

#[tauri::command]
pub async fn get_enroute_basemap_updates(
    catalog: tauri::State<'_, Arc<CatalogService>>,
    basemaps: tauri::State<'_, Arc<Mutex<Basemaps>>>,
) -> Result<Vec<&'static str>, &'static str> {
    let cached = catalog
        .status()
        .cached
        .ok_or("Basemap catalog is unavailable")?;
    let basemaps = basemaps.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        basemaps.lock().unwrap().available_updates(&cached.entries)
    })
    .await
    .unwrap_or_else(|error| Err(error.into()))
    .map_err(|error| {
        tracing::warn!(%error, "Could not check basemap updates");
        "Could not check basemap updates"
    })
}

#[tauri::command]
pub async fn get_enroute_terrain_updates(
    catalog: tauri::State<'_, Arc<CatalogService>>,
    terrain: tauri::State<'_, Arc<Mutex<Terrain>>>,
) -> Result<Vec<&'static str>, &'static str> {
    let cached = catalog
        .status()
        .cached
        .ok_or("Terrain catalog is unavailable")?;
    let terrain = terrain.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        terrain.lock().unwrap().available_updates(&cached.entries)
    })
    .await
    .unwrap_or_else(|error| Err(error.into()))
    .map_err(|error| {
        tracing::warn!(%error, "Could not check terrain updates");
        "Could not check terrain updates"
    })
}

#[cfg(test)]
mod tests;
