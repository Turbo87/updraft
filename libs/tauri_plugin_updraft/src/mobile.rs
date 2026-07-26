use serde::{Serialize, de::DeserializeOwned};
use tauri::{
    AppHandle, Runtime,
    ipc::Channel,
    plugin::{PluginApi, PluginHandle},
};

pub fn init<R: Runtime, C: DeserializeOwned>(
    _app: &AppHandle<R>,
    api: PluginApi<R, C>,
) -> crate::Result<UpdraftMobile<R>> {
    let handle = api.register_android_plugin("aero.updraft.mobile", "UpdraftMobilePlugin")?;
    Ok(UpdraftMobile(handle))
}

#[derive(Serialize)]
struct StartSessionArgs {
    fixes: Channel,
}

/// Access to the session controls the Kotlin plugin implements.
pub struct UpdraftMobile<R: Runtime>(PluginHandle<R>);

impl<R: Runtime> UpdraftMobile<R> {
    /// Starts a session, which reports every [`crate::Fix`] its receiver
    /// produces on `fixes` until the session is stopped.
    pub fn start_session(&self, fixes: Channel) -> crate::Result<()> {
        self.0
            .run_mobile_plugin("startSession", StartSessionArgs { fixes })
            .map_err(Into::into)
    }

    pub fn stop_session(&self) -> crate::Result<()> {
        self.0
            .run_mobile_plugin("stopSession", ())
            .map_err(Into::into)
    }
}
