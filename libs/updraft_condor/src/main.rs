//! Bridge from the Condor 3 soaring simulator to Updraft.
//!
//! See `README.md` in this crate for the design and the implementation plan.

mod config;
mod console;
mod ini;
mod nmea;
mod nmea_input;
mod server;

use crate::config::{CompetitionNumber, Config};
use anyhow::{Context as _, Result};
use bytes::Bytes;
use clap::Parser;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use tokio::net::TcpListener;
use tokio::sync::broadcast;
use tracing::info;
use tracing_subscriber::filter::LevelFilter;

const BROADCAST_CAPACITY: usize = 64;

#[derive(Parser)]
#[command(about = "Bridge Condor 3 outputs to Updraft as one NMEA stream over TCP")]
struct Args {
    /// Configuration file. Defaults to `updraft_condor.ini` next to the executable.
    #[arg(long)]
    config: Option<PathBuf>,
}

/// The effective values after the config file, detection, and prompts.
struct Settings {
    condor_folder: PathBuf,
    competition_number: String,
    config: Config,
}

fn main() {
    tracing_subscriber::fmt()
        .compact()
        .with_ansi(false)
        .with_target(false)
        .with_max_level(LevelFilter::INFO)
        .init();

    if let Err(error) = run() {
        eprintln!("Error: {error:#}");
        console::wait_for_enter();
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let args = Args::parse();
    let config_path = match args.config {
        Some(path) => path,
        None => default_config_path()?,
    };
    let settings = resolve_settings(&config_path)?;
    print_summary(&settings, &config_path);

    let runtime = tokio::runtime::Runtime::new().context("failed to start the runtime")?;
    runtime.block_on(serve(&settings))
}

async fn serve(settings: &Settings) -> Result<()> {
    let config = &settings.config;
    let output = bind(config.output_listen).await?;
    let nmea_input = bind(config.nmea_listen).await?;
    let (sender, _) = broadcast::channel::<Bytes>(BROADCAST_CAPACITY);
    info!("listening for Updraft on {}", config.output_listen);
    info!("listening for Condor NMEA on {}", config.nmea_listen);

    tokio::select! {
        result = server::run(output, sender.clone()) => result?,
        result = nmea_input::run(nmea_input, sender.clone()) => result?,
    }
    Ok(())
}

async fn bind(address: SocketAddr) -> Result<TcpListener> {
    TcpListener::bind(address)
        .await
        .with_context(|| format!("failed to listen on {address}"))
}

fn default_config_path() -> Result<PathBuf> {
    let executable = std::env::current_exe().context("failed to locate the executable")?;
    let folder = executable
        .parent()
        .context("the executable has no parent folder")?;
    Ok(folder.join(config::FILE_NAME))
}

fn resolve_settings(config_path: &Path) -> Result<Settings> {
    let mut config = Config::load_or_create(config_path)?;

    let condor_folder = match &config.condor_folder {
        Some(folder) => folder.clone(),
        None => match config::detect_condor_folder() {
            Some(folder) => folder,
            None => {
                let answer = console::prompt("Condor 3 installation folder")?;
                let folder = PathBuf::from(answer);
                if !config::is_condor_folder(&folder) {
                    anyhow::bail!("{} has no Settings folder", folder.display());
                }
                config.condor_folder = Some(folder.clone());
                config.save(config_path)?;
                folder
            }
        },
    };

    let competition_number = match &config.competition_number {
        Some(number) => number.clone(),
        None => match documents_folder()
            .map(|documents| config::detect_competition_number(&documents))
        {
            Some(CompetitionNumber::Found(number)) => number,
            detection => {
                if let Some(CompetitionNumber::Ambiguous(count)) = detection {
                    info!("found {count} pilot profiles, asking for the competition number");
                }
                let number = console::prompt("Own competition number (as in Condor)")?;
                if number.is_empty() {
                    anyhow::bail!("the competition number must not be empty");
                }
                config.competition_number = Some(number.clone());
                config.save(config_path)?;
                number
            }
        },
    };

    Ok(Settings {
        condor_folder,
        competition_number,
        config,
    })
}

/// The user's Documents folder, where Condor keeps the pilot profiles.
fn documents_folder() -> Option<PathBuf> {
    std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(|home| PathBuf::from(home).join("Documents"))
}

fn print_summary(settings: &Settings, config_path: &Path) {
    let config = &settings.config;
    println!("updraft_condor");
    println!("  Config file:          {}", config_path.display());
    println!(
        "  Condor folder:        {}",
        settings.condor_folder.display()
    );
    println!("  Competition number:   {}", settings.competition_number);
    println!("  HW VSP3 connects to:  {}", config.nmea_listen);
    println!("  Condor UDP sends to:  {}", config.udp_listen);
    println!("  Updraft connects to:  {}", config.output_listen);
    if let Some(address) = local_address() {
        println!("  This PC on the LAN:   {address}");
    }
}

/// The address of the interface that routes to the local network. The
/// socket is never used to send.
fn local_address() -> Option<std::net::IpAddr> {
    let socket = std::net::UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.connect("192.168.0.1:9").ok()?;
    socket.local_addr().ok().map(|address| address.ip())
}
