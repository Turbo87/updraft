use super::{CatalogService, CatalogStatus};
use crate::basemap::Basemaps;
use crate::terrain::Terrain;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use tauri::ipc::Channel;
use tokio::sync::watch;

pub struct CatalogSubscriptions {
    status: watch::Receiver<CatalogStatus>,
    channels: Arc<Mutex<BTreeMap<u32, Channel<CatalogStatus>>>>,
}

impl CatalogSubscriptions {
    pub fn new(service: &CatalogService) -> Self {
        let mut updates = service.subscribe();
        let status = updates.clone();
        let channels = Arc::new(Mutex::new(BTreeMap::<u32, Channel<CatalogStatus>>::new()));
        let subscribers = channels.clone();
        tauri::async_runtime::spawn(async move {
            while updates.changed().await.is_ok() {
                let mut subscribers = subscribers.lock().unwrap();
                // Read after locking channels to preserve initial-delivery ordering.
                let status = updates.borrow_and_update().clone();
                subscribers.retain(|_, channel| channel.send(status.clone()).is_ok());
            }
        });
        Self { status, channels }
    }

    fn subscribe(&self, channel: Channel<CatalogStatus>) -> tauri::Result<()> {
        let mut channels = self.channels.lock().unwrap();
        channel.send(self.status.borrow().clone())?;
        channels.insert(channel.id(), channel);
        Ok(())
    }
}

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
    state.channels.lock().unwrap().remove(&channel_id);
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
