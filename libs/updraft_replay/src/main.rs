mod replay;
mod server;

use crate::replay::Replay;
use anyhow::{Context as _, Result};
use clap::Parser;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
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

fn has_nmea_extension(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("nmea"))
}

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
    let replay = load_replay(&args.file).await?;
    let skip = Duration::from_secs(args.skip);
    replay.events_from(skip)?;

    for warning in replay.warnings() {
        warn!("{warning}");
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

async fn load_replay(path: &Path) -> Result<Replay> {
    if !has_nmea_extension(path) {
        anyhow::bail!("replay input must use the .nmea file extension");
    }

    let bytes = tokio::fs::read(path)
        .await
        .with_context(|| format!("failed to read {}", path.display()))?;
    Ok(Replay::from_nmea(bytes)?)
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

#[cfg(test)]
mod tests {
    use super::*;
    use claims::{assert_err, assert_ok};
    use std::path::Path;

    const BASIC: &str = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../testdata/nmea/basic.nmea"
    );

    #[test]
    fn selects_nmea_input_case_insensitively() {
        for path in ["flight.nmea", "flight.NMEA"] {
            assert!(has_nmea_extension(Path::new(path)));
        }
    }

    #[test]
    fn rejects_non_nmea_input_extensions() {
        for path in ["flight.igc", "flight", "flight.txt"] {
            assert!(!has_nmea_extension(Path::new(path)));
        }
    }

    #[cfg(unix)]
    #[test]
    fn rejects_a_non_utf8_input_extension() {
        use std::ffi::OsStr;
        use std::os::unix::ffi::OsStrExt as _;

        let path = Path::new(OsStr::from_bytes(b"flight.\xFF"));
        assert!(!has_nmea_extension(path));
    }

    #[tokio::test]
    async fn rejects_the_extension_before_reading_the_file() {
        let path = std::env::temp_dir().join(format!(
            "updraft-replay-missing-input-{}.igc",
            std::process::id()
        ));
        assert!(!path.exists(), "test input must not exist");

        let error = assert_err!(load_replay(&path).await);

        assert_eq!(
            error.to_string(),
            "replay input must use the .nmea file extension"
        );
    }

    #[tokio::test]
    async fn loads_nmea_input() {
        let replay = assert_ok!(load_replay(Path::new(BASIC)).await);

        assert_eq!(replay.duration(), Duration::from_secs(3));
    }
}
