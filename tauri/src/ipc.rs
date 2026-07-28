use crate::driver::DriverHandle;
use tauri::ipc::Channel;
use updraft_core::Topic;

#[tauri::command]
pub fn set_locale(locale: updraft_core::Locale, handle: tauri::State<'_, DriverHandle>) {
    handle.send(updraft_core::Input::SetLocale(locale));
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
