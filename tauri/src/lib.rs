use tauri::Manager;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::rolling::Rotation;
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            if let Some(guard) = init_tracing(app.handle()) {
                app.manage(guard);
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
