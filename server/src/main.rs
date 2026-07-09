use std::net::{IpAddr, SocketAddr};
use std::path::PathBuf;

use anyhow::Context;
use clap::Parser;
use tokio::net::TcpListener;
use updraft_core::{App, CoreRuntime};

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

    /// Enables simulator input routes
    #[arg(long, env = "UPDRAFT_SIMULATION", default_value_t = false)]
    simulation: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let runtime = CoreRuntime::spawn(App::default());
    let app = if args.simulation {
        updraft_server::simulation_router(&args.static_dir, runtime)
    } else {
        updraft_server::router(&args.static_dir, runtime)
    };

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simulation_profile_is_enabled_by_flag() {
        let args = Args::try_parse_from(["updraft-server", "--simulation"]).unwrap();

        assert!(args.simulation);
    }

    #[test]
    fn simulation_profile_is_disabled_by_default() {
        let args = Args::try_parse_from(["updraft-server"]).unwrap();

        assert!(!args.simulation);
    }
}
