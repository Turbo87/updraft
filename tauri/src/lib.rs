/// Installs the process-wide `tracing` subscriber for the Tauri host.
///
/// Emits human-readable records to stderr (desktop / `tauri … dev`), filtered by
/// `UPDRAFT_LOG` (then `RUST_LOG`, else `debug` in debug builds and `info` in
/// release). `.init()` also installs a `LogTracer`, so dependencies that still
/// emit through the `log` facade (including parts of Tauri) are captured too.
fn init_tracing() {
    use tracing_subscriber::{EnvFilter, fmt, prelude::*};

    let default_level = if cfg!(debug_assertions) { "debug" } else { "info" };
    let filter = EnvFilter::try_from_env("UPDRAFT_LOG")
        .or_else(|_| EnvFilter::try_from_default_env())
        .unwrap_or_else(|_| EnvFilter::new(default_level));

    tracing_subscriber::registry()
        .with(filter)
        .with(fmt::layer().with_writer(std::io::stderr))
        .init();
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    init_tracing();

    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
