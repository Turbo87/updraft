use super::catalog::CatalogService;
use super::queue::{DownloadOutcome, DownloadQueue, DownloadStatus};
use super::{CatalogEntry, download::DownloadFile};
use crate::basemap::Basemaps;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use tauri::ipc::Channel;
use tauri::{AppHandle, Manager, Runtime};

pub struct DownloadCommands {
    pub queue: Arc<Mutex<DownloadQueue>>,
    subscribers: Arc<Mutex<BTreeMap<u32, Channel<Vec<DownloadStatus>>>>>,
}

impl Default for DownloadCommands {
    fn default() -> Self {
        let queue = Arc::new(Mutex::new(DownloadQueue::default()));
        let mut status = queue.lock().unwrap().subscribe();
        let subscribers = Arc::new(Mutex::new(
            BTreeMap::<u32, Channel<Vec<DownloadStatus>>>::new(),
        ));
        let channels = subscribers.clone();
        tauri::async_runtime::spawn(async move {
            while status.changed().await.is_ok() {
                let mut channels = channels.lock().unwrap();
                // Read after locking channels to preserve initial-delivery ordering.
                let update = status.borrow_and_update().clone();
                channels.retain(|_, channel| channel.send(update.clone()).is_ok());
            }
        });
        Self { queue, subscribers }
    }
}

impl DownloadCommands {
    /// Runs on a blocking worker. The queue lock excludes cancellation during installation.
    pub fn install_download(
        &self,
        basemaps: &Mutex<Basemaps>,
        attempt: &Arc<CatalogEntry>,
        download: DownloadFile,
    ) {
        let mut queue = self.queue.lock().unwrap();
        if !queue.is_active(attempt) {
            return;
        }
        let name = format!("enroute/{}", attempt.path);
        let result = basemaps.lock().unwrap().install_download(&name, download);
        let outcome = match result {
            Ok(()) => DownloadOutcome::Installed,
            Err(error) => {
                tracing::warn!(%error, name, "Could not install downloaded basemap");
                DownloadOutcome::Failed
            }
        };
        queue.finish(attempt, outcome);
    }

    fn subscribe(&self, channel: Channel<Vec<DownloadStatus>>) -> tauri::Result<()> {
        let queue = self.queue.lock().unwrap();
        let mut subscribers = self.subscribers.lock().unwrap();
        let status = queue.subscribe();
        channel.send(status.borrow().clone())?;
        subscribers.insert(channel.id(), channel);
        Ok(())
    }
}

#[tauri::command(async)]
pub fn download_enroute_basemaps<R: Runtime>(
    paths: Vec<String>,
    app: AppHandle<R>,
    state: tauri::State<'_, DownloadCommands>,
    catalog: tauri::State<'_, Arc<CatalogService>>,
) -> Result<(), &'static str> {
    let cached = catalog
        .status()
        .cached
        .ok_or("Basemap catalog is unavailable")?;
    let entries = paths
        .iter()
        .map(|path| {
            let entry = cached.entries.iter().find(|entry| entry.path == path);
            entry.cloned().ok_or("Basemap is not in the catalog")
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
                let downloads = installer.state::<DownloadCommands>();
                downloads.install_download(basemaps.inner(), &attempt, download);
            })
            .await;
            if let Err(error) = result {
                tracing::error!(%error, "Basemap installation worker failed");
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
    state.subscribers.lock().unwrap().remove(&channel_id);
}

#[tauri::command(async)]
pub fn cancel_enroute_download(path: String, state: tauri::State<'_, DownloadCommands>) {
    state.queue.lock().unwrap().cancel(&path);
}

#[cfg(test)]
mod tests;
