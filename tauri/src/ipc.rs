use crate::driver::DriverHandle;
use serde::Serialize;
use tauri::ipc::Channel;
use updraft_core::{SetLocale, Topic};

#[derive(Debug, Serialize, thiserror::Error)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum DriverCommandError {
    #[error("driver stopped")]
    DriverStopped,
}

#[tauri::command]
pub async fn set_locale(
    locale: updraft_core::Locale,
    handle: tauri::State<'_, DriverHandle>,
) -> Result<(), DriverCommandError> {
    handle
        .send(SetLocale::new(locale))
        .await
        .map_err(|_| DriverCommandError::DriverStopped)
}

/// Registers the webview's channel as a subscriber.
///
/// `Channel::send` fails once the webview is gone, which is exactly the
/// signal the driver uses to prune the sink.
#[tauri::command]
pub fn subscribe(channel: Channel<Topic>, handle: tauri::State<'_, DriverHandle>) {
    handle.subscribe(Box::new(move |topic: &Topic| {
        channel.send(topic.clone()).is_ok()
    }));
}
