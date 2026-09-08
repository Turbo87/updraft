use super::{BasemapSource, Basemaps};
use serde::Serialize;
use std::sync::{Arc, Mutex};
use tauri::ipc::Channel;

#[derive(Serialize)]
pub struct BasemapStatus {
    generation: u64,
    sources: Vec<BasemapSourceStatus>,
}

#[derive(Serialize)]
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
    fn status(&self) -> BasemapStatus {
        let sources = self
            .files
            .iter()
            .map(|(path, source)| {
                let source_name = path
                    .file_name()
                    .expect("Basemap files have names")
                    .to_string_lossy()
                    .into_owned();
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
