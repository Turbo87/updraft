use serde::{Deserialize, Serialize, de::DeserializeOwned};
use tauri::{
    AppHandle, Runtime,
    ipc::Channel,
    plugin::{PluginApi, PluginHandle},
};
use uuid::Uuid;

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

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct StartSppAttemptArgs<'a> {
    address: &'a str,
    service_uuid: Uuid,
    events: Channel,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CancelSppAttemptArgs {
    connection_id: crate::SppConnectionId,
}

#[derive(Serialize)]
struct WatchActivitiesArgs {
    activities: Channel,
}

#[derive(Serialize)]
struct DocumentDisplayNameArgs<'a> {
    uri: &'a str,
}

#[derive(Deserialize)]
struct DocumentDisplayName {
    name: Option<String>,
}

/// Access to the session controls the Kotlin plugin implements.
pub struct UpdraftMobile<R: Runtime>(PluginHandle<R>);

impl<R: Runtime> UpdraftMobile<R> {
    /// Reads the display name of a selected Android document URI.
    pub fn document_display_name(&self, uri: &str) -> crate::Result<Option<String>> {
        let result: DocumentDisplayName = self
            .0
            .run_mobile_plugin("documentDisplayName", DocumentDisplayNameArgs { uri })?;
        Ok(result.name)
    }

    /// Returns the current Android bonded-device state.
    pub fn bonded_bluetooth_devices(&self) -> crate::Result<crate::BondedBluetoothDevices> {
        self.0
            .run_mobile_plugin("bondedBluetoothDevices", ())
            .map_err(Into::into)
    }

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

    /// Stops the session and ends the process.
    ///
    /// The platform ends the process while the call is in flight, so an `Ok`
    /// says only that the platform accepted the quit.
    pub fn quit(&self) -> crate::Result<()> {
        self.0.run_mobile_plugin("quit", ()).map_err(Into::into)
    }

    pub fn start_spp_attempt(
        &self,
        address: &str,
        service_uuid: Uuid,
        events: Channel,
    ) -> crate::Result<()> {
        let args = StartSppAttemptArgs {
            address,
            service_uuid,
            events,
        };

        self.0
            .run_mobile_plugin("startSppAttempt", args)
            .map_err(Into::into)
    }

    pub fn cancel_spp_attempt(&self, connection_id: crate::SppConnectionId) -> crate::Result<()> {
        self.0
            .run_mobile_plugin("cancelSppAttempt", CancelSppAttemptArgs { connection_id })
            .map_err(Into::into)
    }

    /// Reports every activity lifecycle transition on `activities`, so the
    /// caller learns about an activity the platform created behind its back.
    pub fn watch_activities(&self, activities: Channel) -> crate::Result<()> {
        self.0
            .run_mobile_plugin("watchActivities", WatchActivitiesArgs { activities })
            .map_err(Into::into)
    }
}
