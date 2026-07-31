use serde::de::DeserializeOwned;
use tauri::{AppHandle, Runtime, ipc::Channel, plugin::PluginApi};
use uuid::Uuid;

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

    pub fn start_spp_attempt(
        &self,
        _address: &str,
        _service_uuid: Uuid,
        _events: Channel,
    ) -> crate::Result<()> {
        Ok(())
    }

    pub fn cancel_spp_attempt(&self) -> crate::Result<()> {
        Ok(())
    }

    pub fn watch_activities(&self, _activities: Channel) -> crate::Result<()> {
        Ok(())
    }
}
