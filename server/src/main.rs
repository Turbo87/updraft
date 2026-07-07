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
    let args = Args::parse();

    let app = updraft_server::router(&args.static_dir);

    let addr = SocketAddr::new(args.ip, args.port);
    let listener = TcpListener::bind(addr)
        .await
        .with_context(|| format!("failed to bind to {addr}"))?;

    println!("listening on http://{}", listener.local_addr()?);

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
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
