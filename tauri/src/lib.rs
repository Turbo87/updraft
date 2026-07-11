use tauri::Manager;
use tracing_appender::non_blocking::WorkerGuard;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            if let Some(guard) = init_tracing(app.handle()) {
                // Keep the non-blocking file writer's worker alive for the whole
                // process; dropping it would discard buffered log lines on exit.
                app.manage(guard);
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// Installs the process-wide `tracing` subscriber for the Tauri host.
///
/// Composes, in one subscriber:
/// - an `EnvFilter` (`UPDRAFT_LOG`, then `RUST_LOG`, else `debug` in debug builds
///   and `info` in release),
/// - a human-readable layer to stderr for desktop / `tauri … dev`,
/// - a rolling daily file in the platform app-log directory (best effort),
/// - Android logcat via `paranoid-android` and iOS oslog via `tracing-oslog`.
///
/// `.init()` also installs a `LogTracer`, so dependencies that still emit through
/// the `log` facade (including parts of Tauri) are captured too.
///
/// Returns the file writer's [`WorkerGuard`], which the caller must keep alive.
fn init_tracing<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> Option<WorkerGuard> {
    use tracing_subscriber::{EnvFilter, fmt, prelude::*};

    let default_level = if cfg!(debug_assertions) { "debug" } else { "info" };
    let filter = EnvFilter::try_from_env("UPDRAFT_LOG")
        .or_else(|_| EnvFilter::try_from_default_env())
        .unwrap_or_else(|_| EnvFilter::new(default_level));

    // Rolling daily file in the OS app-log dir (best effort — skipped if the
    // directory can't be resolved or created, so logging never blocks startup).
    let (file_layer, guard) = match app.path().app_log_dir() {
        Ok(dir) if std::fs::create_dir_all(&dir).is_ok() => {
            let appender = tracing_appender::rolling::daily(dir, "updraft.log");
            let (writer, guard) = tracing_appender::non_blocking(appender);
            let layer = fmt::layer().with_ansi(false).with_writer(writer);
            (Some(layer), Some(guard))
        }
        _ => (None, None),
    };

    // `Option<Layer>` is itself a `Layer` (a no-op when `None`), which keeps the
    // subscriber one concrete type regardless of which platform layer is active.
    #[cfg(target_os = "android")]
    let platform_layer = Some(fmt::layer().with_ansi(false).with_writer(
        paranoid_android::AndroidLogMakeWriter::new("Updraft".to_owned()),
    ));
    #[cfg(target_os = "ios")]
    let platform_layer = Some(tracing_oslog::OsLogger::new("aero.updraft", "default"));
    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    let platform_layer: Option<tracing_subscriber::layer::Identity> = None;

    tracing_subscriber::registry()
        .with(filter)
        .with(fmt::layer().with_writer(std::io::stderr))
        .with(file_layer)
        .with(platform_layer)
        .init();

    guard
}
