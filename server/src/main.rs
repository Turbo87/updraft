use anyhow::Context;
use clap::Parser;
use std::env;
use std::net::{IpAddr, SocketAddr};
use std::path::PathBuf;
use tokio::net::TcpListener;
use updraft_core::App;

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
    init_tracing();

    let args = Args::parse();

    let runtime = updraft_server::start_runtime(App::new());
    let app = updraft_server::router(
        updraft_server::ServerState {
            runtime: runtime.handle(),
        },
        updraft_server::RouterOptions {
            static_dir: Some(args.static_dir),
            simulation: args.simulation,
        },
    );

    let addr = SocketAddr::new(args.ip, args.port);
    let listener = TcpListener::bind(addr)
        .await
        .with_context(|| format!("failed to bind to {addr}"))?;

    let local_addr = listener.local_addr()?;
    tracing::info!("listening on http://{local_addr}");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    runtime.shutdown();

    Ok(())
}

/// Installs the process-wide `tracing` subscriber.
///
/// Verbosity is controlled by the `UPDRAFT_LOG` environment variable (falling
/// back to `RUST_LOG`), e.g. `UPDRAFT_LOG=debug` or
/// `UPDRAFT_LOG=updraft_server=trace,tower_http=debug`.
fn init_tracing() {
    use tracing_subscriber::{EnvFilter, fmt, prelude::*};

    let filter = EnvFilter::try_from_env("UPDRAFT_LOG")
        .or_else(|_| EnvFilter::try_from_default_env())
        .unwrap_or_else(|error| {
            if env::var_os("UPDRAFT_LOG").is_some() || env::var_os("RUST_LOG").is_some() {
                eprintln!("invalid log filter, falling back to `info`: {error}");
            }

            EnvFilter::new("info")
        });

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

#[cfg(test)]
mod tests {
    use super::Args;
    use claims::assert_ok;
    use clap::Parser as _;

    #[test]
    fn simulation_profile_is_enabled_by_flag() {
        let args = assert_ok!(Args::try_parse_from(["updraft-server", "--simulation"]));

        assert!(args.simulation);
    }

    #[test]
    fn simulation_profile_is_disabled_by_default() {
        let args = assert_ok!(Args::try_parse_from(["updraft-server"]));

        assert!(!args.simulation);
    }
}
