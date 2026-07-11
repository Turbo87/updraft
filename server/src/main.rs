use std::net::{IpAddr, SocketAddr};
use std::path::PathBuf;

use anyhow::Context;
use clap::Parser;
use tokio::net::TcpListener;

#[derive(Parser)]
struct Args {
    /// IP address to bind the HTTP server to
    #[arg(long, env = "UPDRAFT_IP", default_value = "127.0.0.1")]
    ip: IpAddr,

    /// Port to bind the HTTP server to
    #[arg(long, env = "UPDRAFT_PORT", default_value_t = 4449)]
    port: u16,

    /// Directory holding the built frontend assets
    #[arg(long, env = "UPDRAFT_STATIC_DIR", default_value = "frontend/build")]
    static_dir: PathBuf,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_tracing();

    let args = Args::parse();

    let app = updraft_server::router(&args.static_dir);

    let addr = SocketAddr::new(args.ip, args.port);
    let listener = TcpListener::bind(addr)
        .await
        .with_context(|| format!("failed to bind to {addr}"))?;

    let local_addr = listener.local_addr()?;
    tracing::info!(%local_addr, "listening on http://{local_addr}");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

/// Installs the process-wide `tracing` subscriber.
///
/// Output goes to stderr as human-readable text. Verbosity is controlled by the
/// `UPDRAFT_LOG` environment variable (falling back to `RUST_LOG`), e.g.
/// `UPDRAFT_LOG=debug` or `UPDRAFT_LOG=updraft_server=trace,tower_http=debug`.
/// With the `env-filter` feature the subscriber also captures `log`-only
/// dependencies, so nothing routed through the `log` facade is lost.
fn init_tracing() {
    use tracing_subscriber::{EnvFilter, fmt, prelude::*};

    let filter = EnvFilter::try_from_env("UPDRAFT_LOG")
        .or_else(|_| EnvFilter::try_from_default_env())
        .unwrap_or_else(|_| EnvFilter::new("info"));

    tracing_subscriber::registry()
        .with(filter)
        .with(fmt::layer().with_writer(std::io::stderr))
        .init();
}

/// Resolves when the process receives Ctrl-C (SIGINT) or, on Unix, SIGTERM.
async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl-C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => {}
        () = terminate => {}
    }
}
