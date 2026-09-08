use super::{BasemapSource, Basemaps};
use serde::Serialize;
use std::sync::{Arc, Mutex};
use tauri::ipc::Channel;

#[derive(Clone, Serialize)]
pub struct BasemapStatus {
    generation: u64,
    sources: Vec<BasemapSourceStatus>,
}

#[derive(Clone, Serialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
enum BasemapSourceStatus {
    Active { source_name: String },
    Disabled { source_name: String },
    Unavailable { source_name: String },
}

impl Basemaps {
    pub fn publish(&mut self) {
        let status = self.status();
        self.subscribers
            .retain(|_, channel| channel.send(status.clone()).is_ok());
    }

    fn status(&self) -> BasemapStatus {
        let sources = self
            .files
            .iter()
            .map(|(id, source)| {
                let source_name = id.clone();
                match source {
                    BasemapSource::Active(_) => BasemapSourceStatus::Active { source_name },
                    BasemapSource::Disabled => BasemapSourceStatus::Disabled { source_name },
                    BasemapSource::Unavailable(_) => {
                        BasemapSourceStatus::Unavailable { source_name }
                    }
                }
            })
            .collect();
        BasemapStatus {
            generation: self.generation,
            sources,
        }
    }

    fn subscribe(&mut self, channel: Channel<BasemapStatus>) -> tauri::Result<()> {
        channel.send(self.status())?;
        self.subscribers.insert(channel.id(), channel);
        Ok(())
    }
}

#[tauri::command]
pub async fn set_basemap_enabled(
    source_name: String,
    enabled: bool,
    state: tauri::State<'_, Arc<Mutex<Basemaps>>>,
) -> Result<(), &'static str> {
    let basemaps = state.inner().clone();
    let name = source_name.clone();
    tauri::async_runtime::spawn_blocking(move || {
        basemaps
            .lock()
            .expect("Basemap access should not panic")
            .set_enabled(&name, enabled)
    })
    .await
    .unwrap_or_else(|error| Err(error.into()))
    .map_err(|error| {
        tracing::warn!(%error, source_name, "Could not change basemap activation");
        "Could not change basemap activation"
    })
}

#[tauri::command(async)]
pub fn subscribe_basemaps(
    channel: Channel<BasemapStatus>,
    state: tauri::State<'_, Arc<Mutex<Basemaps>>>,
) -> Result<(), &'static str> {
    state
        .lock()
        .expect("Basemap access should not panic")
        .subscribe(channel)
        .map_err(|_| "Could not subscribe to basemap status")
}

#[tauri::command]
pub async fn remove_basemap(
    source_name: String,
    state: tauri::State<'_, Arc<Mutex<Basemaps>>>,
) -> Result<(), &'static str> {
    let basemaps = state.inner().clone();
    let name = source_name.clone();
    tauri::async_runtime::spawn_blocking(move || {
        basemaps
            .lock()
            .expect("Basemap access should not panic")
            .remove(&name)
    })
    .await
    .unwrap_or_else(|error| Err(error.into()))
    .map_err(|error| {
        tracing::warn!(%error, source_name, "Could not remove basemap file");
        "Could not remove basemap file"
    })
}

#[tauri::command(async)]
pub fn unsubscribe_basemaps(channel_id: u32, state: tauri::State<'_, Arc<Mutex<Basemaps>>>) {
    state
        .lock()
        .expect("Basemap access should not panic")
        .subscribers
        .remove(&channel_id);
}

#[cfg(test)]
mod tests {
    use super::*;
    use claims::assert_err;

    #[test]
    fn failed_initial_delivery_does_not_register_a_subscriber() {
        let mut basemaps = Basemaps::default();
        let channel = Channel::new(|_| Err(std::io::Error::other("closed channel").into()));
        assert_err!(basemaps.subscribe(channel));
        assert!(basemaps.subscribers.is_empty());
    }
}
