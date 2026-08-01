use tauri::{
    Manager, Runtime,
    plugin::{Builder, TauriPlugin},
};

#[cfg(desktop)]
mod desktop;
mod error;
#[cfg(mobile)]
mod mobile;
mod models;

pub use error::{Error, Result};
pub use models::{Fix, SppConnectionId, SppEvent};

#[cfg(desktop)]
use desktop::UpdraftMobile;
#[cfg(mobile)]
use mobile::UpdraftMobile;

/// Extends [`tauri::AppHandle`] with access to the plugin's session controls.
pub trait UpdraftMobileExt<R: Runtime> {
    fn updraft_mobile(&self) -> &UpdraftMobile<R>;
}

impl<R: Runtime, T: Manager<R>> UpdraftMobileExt<R> for T {
    fn updraft_mobile(&self) -> &UpdraftMobile<R> {
        self.state::<UpdraftMobile<R>>().inner()
    }
}

/// Initializes the plugin.
pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("updraft")
        .setup(|app, api| {
            #[cfg(mobile)]
            let updraft_mobile = mobile::init(app, api)?;
            #[cfg(desktop)]
            let updraft_mobile = desktop::init(app, api)?;
            app.manage(updraft_mobile);
            Ok(())
        })
        .build()
}
