mod replay;
mod server;

use crate::replay::Replay;
use anyhow::{Context as _, Result};
use clap::Parser;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::sync::broadcast;
use tracing::{info, warn};
use tracing_indicatif::IndicatifLayer;
use tracing_subscriber::Layer as _;
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::layer::SubscriberExt as _;
use tracing_subscriber::util::SubscriberInitExt as _;

const BROADCAST_CAPACITY: usize = 64;

#[derive(Parser)]
#[command(about = "Replay an NMEA file through a TCP server")]
struct Args {
    /// NMEA file to replay.
    file: PathBuf,

    /// TCP address to listen on.
    #[arg(long, default_value = "127.0.0.1:4353")]
    listen: SocketAddr,

    /// Restart the replay after the file ends.
    #[arg(long)]
    r#loop: bool,

    /// Start the first pass at this many seconds.
    #[arg(long, default_value_t = 0)]
    skip: u64,
}

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing();
    let args = Args::parse();
    let bytes = tokio::fs::read(&args.file)
        .await
        .with_context(|| format!("failed to read {}", args.file.display()))?;
    let replay = Replay::from_bytes(bytes)?;
    let skip = Duration::from_secs(args.skip);
    replay.events_from(skip)?;

    if replay.has_timestamp_regression() {
        warn!("NMEA replay timestamps moved backward");
    }

    let listener = TcpListener::bind(args.listen)
        .await
        .with_context(|| format!("failed to listen on {}", args.listen))?;
    info!("NMEA replay server listening on {}", args.listen);
    let (sender, _) = broadcast::channel(BROADCAST_CAPACITY);

    tokio::select! {
        result = server::run(listener, sender.clone()) => result?,
        result = replay.play(sender, skip, args.r#loop) => result?,
    }

    Ok(())
}

fn init_tracing() {
    let indicatif = IndicatifLayer::new();
    let format = tracing_subscriber::fmt::layer()
        .compact()
        .with_writer(indicatif.get_stderr_writer())
        .with_filter(LevelFilter::INFO);

    tracing_subscriber::registry()
        .with(format)
        .with(indicatif)
        .init();
}
