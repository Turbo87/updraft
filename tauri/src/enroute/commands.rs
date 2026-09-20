use super::catalog::CatalogService;
use super::queue::{DownloadOutcome, DownloadQueue, DownloadStatus};
use super::subscriptions::StatusChannels;
use super::{CatalogEntry, download::DownloadFile};
use crate::basemap::Basemaps;
use crate::terrain::Terrain;
use std::sync::{Arc, Mutex};
use tauri::ipc::Channel;
use tauri::{AppHandle, Manager, Runtime};

pub struct DownloadCommands {
    pub queue: Arc<Mutex<DownloadQueue>>,
    subscribers: StatusChannels<Vec<DownloadStatus>>,
}

impl Default for DownloadCommands {
    fn default() -> Self {
        let queue = Arc::new(Mutex::new(DownloadQueue::default()));
        let status = queue.lock().unwrap().subscribe();
        let subscribers = StatusChannels::new(status);
        Self { queue, subscribers }
    }
}

impl DownloadCommands {
    /// Runs on a blocking worker. The queue lock excludes cancellation during installation.
    pub fn install_download(
        &self,
        basemaps: &Mutex<Basemaps>,
        terrain: &Mutex<Terrain>,
        attempt: &Arc<CatalogEntry>,
        download: DownloadFile,
    ) {
        let mut queue = self.queue.lock().unwrap();
        if !queue.is_active(attempt) {
            return;
        }
        let name = format!("enroute/{}", attempt.path);
        let result = if attempt.path.ends_with(".terrain") {
            terrain.lock().unwrap().install_download(&name, download)
        } else {
            basemaps.lock().unwrap().install_download(&name, download)
        };
        let outcome = match result {
            Ok(()) => DownloadOutcome::Installed,
            Err(error) => {
                tracing::warn!(%error, name, "Could not install downloaded Enroute file");
                DownloadOutcome::Failed
            }
        };
        queue.finish(attempt, outcome);
    }

    fn subscribe(&self, channel: Channel<Vec<DownloadStatus>>) -> tauri::Result<()> {
        // Keep queue mutations out of initial delivery.
        let _queue = self.queue.lock().unwrap();
        self.subscribers.subscribe(channel)
    }
}

#[tauri::command(async)]
pub fn download_enroute_files<R: Runtime>(
    paths: Vec<String>,
    app: AppHandle<R>,
    state: tauri::State<'_, DownloadCommands>,
    catalog: tauri::State<'_, Arc<CatalogService>>,
) -> Result<(), &'static str> {
    let cached = catalog
        .status()
        .cached
        .ok_or("Enroute catalog is unavailable")?;
    let entries = paths
        .iter()
        .map(|path| {
            let entry = cached.entries.iter().find(|entry| entry.path == path);
            entry.cloned().ok_or("File is not in the Enroute catalog")
        })
        .collect::<Result<Vec<_>, _>>()?;
    let directory = app
        .path()
        .app_data_dir()
        .map_err(|_| "Could not locate data directory")?;
    let queue = state.queue.clone();
    {
        let mut queue = queue.lock().unwrap();
        for entry in entries {
            queue.enqueue(entry);
        }
        if queue.has_active() {
            return Ok(());
        }
    }
    tauri::async_runtime::spawn(async move {
        while let Some((attempt, download)) = DownloadQueue::transfer_next(&queue, &directory).await
        {
            let installer = app.clone();
            let active = attempt.clone();
            let result = tauri::async_runtime::spawn_blocking(move || {
                let basemaps = installer.state::<Arc<Mutex<Basemaps>>>();
                let terrain = installer.state::<Arc<Mutex<Terrain>>>();
                let downloads = installer.state::<DownloadCommands>();
                downloads.install_download(basemaps.inner(), terrain.inner(), &attempt, download);
            })
            .await;
            if let Err(error) = result {
                tracing::error!(%error, "Enroute installation worker failed");
                match queue.lock() {
                    Ok(mut queue) => {
                        queue.finish(&active, DownloadOutcome::Failed);
                    }
                    Err(_) => break,
                }
            }
        }
    });
    Ok(())
}

#[tauri::command(async)]
pub fn subscribe_enroute_downloads(
    channel: Channel<Vec<DownloadStatus>>,
    state: tauri::State<'_, DownloadCommands>,
) -> Result<(), &'static str> {
    state
        .subscribe(channel)
        .map_err(|_| "Could not subscribe to download status")
}

#[tauri::command(async)]
pub fn unsubscribe_enroute_downloads(channel_id: u32, state: tauri::State<'_, DownloadCommands>) {
    state.subscribers.unsubscribe(channel_id);
}

#[tauri::command(async)]
pub fn cancel_enroute_download(path: String, state: tauri::State<'_, DownloadCommands>) {
    state.queue.lock().unwrap().cancel(&path);
}

#[cfg(test)]
mod tests;
