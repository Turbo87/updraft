use serde::de::DeserializeOwned;
use tauri::{AppHandle, Runtime, ipc::Channel, plugin::PluginApi};

pub fn init<R: Runtime, C: DeserializeOwned>(
    app: &AppHandle<R>,
    _api: PluginApi<R, C>,
) -> crate::Result<UpdraftMobile<R>> {
    Ok(UpdraftMobile(app.clone()))
}

/// A no-op implementation so the crate builds on desktop.
pub struct UpdraftMobile<R: Runtime>(AppHandle<R>);

impl<R: Runtime> UpdraftMobile<R> {
    pub fn start_session(&self, _fixes: Channel) -> crate::Result<()> {
        Ok(())
    }

    pub fn stop_session(&self) -> crate::Result<()> {
        Ok(())
    }
}
