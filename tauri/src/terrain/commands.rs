use super::{Terrain, TerrainSource};
use serde::Serialize;
use std::sync::{Arc, Mutex};
use tauri::ipc::Channel;

#[derive(Serialize)]
pub struct TerrainStatus {
    generation: u64,
    sources: Vec<TerrainSourceStatus>,
}

#[derive(Serialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
enum TerrainSourceStatus {
    Active { source_name: String },
    Disabled { source_name: String },
    Unavailable { source_name: String },
}

impl Terrain {
    fn status(&self) -> TerrainStatus {
        let sources = self
            .files
            .iter()
            .map(|(path, source)| {
                let source_name = path
                    .file_name()
                    .expect("Terrain files have names")
                    .to_string_lossy()
                    .into_owned();
                match source {
                    TerrainSource::Active(_) => TerrainSourceStatus::Active { source_name },
                    TerrainSource::Disabled => TerrainSourceStatus::Disabled { source_name },
                    TerrainSource::Unavailable(_) => {
                        TerrainSourceStatus::Unavailable { source_name }
                    }
                }
            })
            .collect();
        TerrainStatus {
            generation: self.generation,
            sources,
        }
    }

    fn subscribe(&mut self, channel: Channel<TerrainStatus>) -> tauri::Result<()> {
        channel.send(self.status())?;
        self.subscribers.insert(channel.id(), channel);
        Ok(())
    }
}

#[tauri::command(async)]
pub fn subscribe_terrain(
    channel: Channel<TerrainStatus>,
    state: tauri::State<'_, Arc<Mutex<Terrain>>>,
) -> Result<(), &'static str> {
    state
        .lock()
        .expect("Terrain access should not panic")
        .subscribe(channel)
        .map_err(|_| "Could not subscribe to terrain status")
}

#[tauri::command(async)]
pub fn unsubscribe_terrain(channel_id: u32, state: tauri::State<'_, Arc<Mutex<Terrain>>>) {
    state
        .lock()
        .expect("Terrain access should not panic")
        .subscribers
        .remove(&channel_id);
}

#[cfg(test)]
mod tests {
    use super::*;
    use claims::assert_err;

    #[test]
    fn failed_initial_delivery_does_not_register_a_subscriber() {
        let mut terrain = Terrain::default();
        let channel = Channel::new(|_| Err(std::io::Error::other("closed channel").into()));
        assert_err!(terrain.subscribe(channel));
        assert!(terrain.subscribers.is_empty());
    }
}
