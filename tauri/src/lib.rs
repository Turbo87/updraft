use std::sync::{Arc, Mutex};
use tauri::Manager;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::rolling::Rotation;
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

mod activity;
mod airspace_resource;
mod airspace_storage;
mod basemap;
mod driver;
mod file_picker;
mod ipc;
mod settings;
// A session only exists on Android. `test` keeps the adapter, and the tests
// that pin the wire contract it implements, compiling on the host.
#[cfg(any(target_os = "android", test))]
mod session;
mod transport;
mod updraft_uri;

/// Installs the process-wide `tracing` subscriber for the Tauri host.
///
/// Returns the file writer's [`WorkerGuard`], which the caller must keep alive.
fn init_tracing<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> Option<WorkerGuard> {
    let filter = EnvFilter::try_from_env("UPDRAFT_LOG")
        .or_else(|_| EnvFilter::try_from_default_env())
        .unwrap_or_else(|_| EnvFilter::new("info"));

    // Rolling daily file in the OS app-log dir
    let (logs_path, file_layer, guard) = match app.path().app_log_dir() {
        Ok(dir) if std::fs::create_dir_all(&dir).is_ok() => {
            let appender = tracing_appender::rolling::RollingFileAppender::builder()
                .rotation(Rotation::DAILY)
                .filename_prefix("updraft")
                .filename_suffix("log")
                .max_log_files(7)
                .build(&dir)
                .expect("failed to initialize rolling file appender");

            let (writer, guard) = tracing_appender::non_blocking(appender);
            let layer = fmt::layer().with_ansi(false).with_writer(writer);
            (Some(dir), Some(layer), Some(guard))
        }
        _ => (None, None, None),
    };

    tracing_subscriber::registry()
        .with(filter)
        .with(fmt::layer().with_writer(std::io::stderr))
        .with(file_layer)
        .init();

    if let Some(logs_path) = logs_path {
        tracing::info!("Logs will be written to {}", logs_path.display());
    }

    guard
}

/// Asks the platform plugin for a foreground session, prompting for location
/// access on the way. The session reports every fix its receiver produces on
/// `fixes`.
///
/// Android only allows a foreground service to start while an activity is
/// visible, which is why this runs from `setup` rather than from wherever the
/// first fix is needed. The call blocks until the pilot has answered the
/// permission prompt, so it cannot run on the thread that has to show it.
#[cfg(target_os = "android")]
fn start_session<R: tauri::Runtime>(app: tauri::AppHandle<R>, fixes: tauri::ipc::Channel) {
    use tauri_plugin_updraft::UpdraftMobileExt;

    tauri::async_runtime::spawn_blocking(move || match app.updraft_mobile().start_session(fixes) {
        Ok(()) => tracing::info!("Background session started"),
        Err(error) => tracing::error!(%error, "Failed to start the background session"),
    });
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .register_asynchronous_uri_scheme_protocol("updraft", updraft_uri::handle_updraft_uri)
        .invoke_handler(tauri::generate_handler![
            ipc::bonded_bluetooth_devices,
            ipc::import_airspace,
            ipc::remove_airspace,
            ipc::set_locale,
            ipc::set_units,
            ipc::add_external_device,
            ipc::delete_external_device,
            ipc::reorder_external_devices,
            ipc::edit_external_device,
            ipc::set_external_device_enabled,
            ipc::subscribe,
            ipc::quit,
        ])
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updraft::init())
        .setup(|app| {
            if let Some(guard) = init_tracing(app.handle()) {
                app.manage(guard);
            }
            let settings_file = settings::SettingsFile::new(app.path().app_config_dir()?);
            let snapshot = settings_file.load();
            let airspace_storage =
                airspace_storage::AirspaceStorage::new(app.path().app_data_dir()?);
            let airspace = airspace_storage.load();
            let basemap_directory = app.path().app_data_dir()?.join("enroute");
            let basemaps = basemap::Basemaps::load(&basemap_directory).unwrap_or_else(|error| {
                tracing::warn!(%error, "Could not scan offline basemap directory");
                basemap::Basemaps::default()
            });
            app.manage(Arc::new(Mutex::new(basemaps)));

            // `setup` runs on the main thread outside any runtime context,
            // so `tokio::spawn` inside the driver would panic. Enter Tauri's
            // runtime for the call rather than making the driver depend on
            // Tauri to spawn itself.
            let handle = {
                let app_handle = app.handle().clone();
                let runtime = tauri::async_runtime::handle();
                let _guard = runtime.inner().enter();
                let persist = Box::new(settings_file.writer());
                driver::Driver::spawn(
                    snapshot,
                    airspace,
                    Box::new(move |device_id, spec, handle| {
                        transport::open(device_id, spec, handle, app_handle.clone())
                    }),
                    persist,
                    std::time::Duration::from_millis(100),
                )
            };

            #[cfg(target_os = "android")]
            let fixes = session::fix_channel(handle.clone());

            let file_picker: file_picker::FileBytesPickerState =
                Box::new(file_picker::TauriFileBytesPicker::new(app.handle().clone()));
            app.manage(handle);
            app.manage(file_picker);
            app.manage(ipc::AirspaceCommandState::new(airspace_storage));

            #[cfg(target_os = "android")]
            start_session(app.handle().clone(), fixes);

            activity::watch(app.handle().clone());

            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|_app, event| match event {
            #[cfg(target_os = "android")]
            tauri::RunEvent::ExitRequested {
                code: None, api, ..
            } => {
                // tao's Android event loop calls `std::process::exit` when the
                // last window closes, which kills the foreground service with
                // it. A session has to outlive the activity that started it.
                // A deliberate quit ends the process through the platform
                // instead, which this arm never sees. See `ipc::quit`.
                api.prevent_exit();
            }
            _ => {}
        });
}
