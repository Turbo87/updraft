use tracing_subscriber::{EnvFilter, fmt, prelude::*};

/// Installs the process-wide `tracing` subscriber for the Tauri host.
fn init_tracing() {
    let filter = EnvFilter::try_from_env("UPDRAFT_LOG")
        .or_else(|_| EnvFilter::try_from_default_env())
        .unwrap_or_else(|_| EnvFilter::new("info"));

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
