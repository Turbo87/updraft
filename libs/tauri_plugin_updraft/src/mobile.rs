use serde::de::DeserializeOwned;
use tauri::{
    AppHandle, Runtime,
    plugin::{PluginApi, PluginHandle},
};

pub fn init<R: Runtime, C: DeserializeOwned>(
    _app: &AppHandle<R>,
    api: PluginApi<R, C>,
) -> crate::Result<UpdraftMobile<R>> {
    let handle = api.register_android_plugin("aero.updraft.mobile", "UpdraftMobilePlugin")?;
    Ok(UpdraftMobile(handle))
}

/// Access to the session controls the Kotlin plugin implements.
pub struct UpdraftMobile<R: Runtime>(PluginHandle<R>);

impl<R: Runtime> UpdraftMobile<R> {
    pub fn start_session(&self) -> crate::Result<()> {
        self.0
            .run_mobile_plugin("startSession", ())
            .map_err(Into::into)
    }

    pub fn stop_session(&self) -> crate::Result<()> {
        self.0
            .run_mobile_plugin("stopSession", ())
            .map_err(Into::into)
    }
}
