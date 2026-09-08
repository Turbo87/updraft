use super::queue::{DownloadQueue, DownloadStatus};
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

#[cfg(test)]
mod tests;
