use super::queue::{DownloadOutcome, DownloadQueue, DownloadStatus};
use super::{BasemapEntry, download::BasemapDownload};
use crate::basemap::Basemaps;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use tauri::ipc::Channel;

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
        attempt: &Arc<BasemapEntry>,
        download: BasemapDownload,
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
