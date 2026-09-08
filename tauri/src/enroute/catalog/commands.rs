use super::{CatalogService, CatalogStatus};
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

#[cfg(test)]
mod tests;
