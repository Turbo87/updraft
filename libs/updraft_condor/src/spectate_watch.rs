//! Watches `Spectate.json` and sends traffic sentences for every new
//! snapshot.

use crate::nmea::SharedOwnFix;
use crate::spectate::{self, Reference};
use anyhow::{Context as _, Result};
use bytes::Bytes;
use notify::{Event, RecursiveMode, Watcher as _};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::sync::{broadcast, mpsc};
use tracing::{debug, info, warn};

/// Condor writes the file in several steps. Events this close together
/// belong to one write.
const DEBOUNCE: Duration = Duration::from_millis(50);

/// The NMEA position is a usable fallback reference while it is this fresh.
const OWN_FIX_MAX_AGE: Duration = Duration::from_secs(3);

pub async fn run(
    path: PathBuf,
    competition_number: String,
    own_fix: SharedOwnFix,
    sender: broadcast::Sender<Bytes>,
) -> Result<()> {
    let folder = path
        .parent()
        .filter(|folder| folder.is_dir())
        .with_context(|| format!("the Spectate folder of {} does not exist", path.display()))?
        .to_path_buf();
    let file_name = path
        .file_name()
        .context("the Spectate path has no file name")?
        .to_os_string();

    let (changes, mut receiver) = mpsc::channel::<()>(16);
    let mut watcher = notify::recommended_watcher(move |result: notify::Result<Event>| {
        match result {
            Ok(event) => {
                let concerns_file = event.paths.iter().any(|event_path| {
                    event_path
                        .file_name()
                        .is_some_and(|name| name.eq_ignore_ascii_case(&file_name))
                });
                if concerns_file {
                    // A full queue already holds a pending signal.
                    let _ = changes.try_send(());
                }
            }
            Err(error) => warn!(%error, "Spectate folder watch failed"),
        }
    })
    .context("failed to create the Spectate folder watcher")?;
    watcher
        .watch(&folder, RecursiveMode::NonRecursive)
        .with_context(|| format!("failed to watch {}", folder.display()))?;
    info!("watching {}", path.display());

    let mut state = State {
        path,
        competition_number,
        own_fix,
        sender,
        last_content: None,
    };
    state.handle_change();

    while receiver.recv().await.is_some() {
        tokio::time::sleep(DEBOUNCE).await;
        while receiver.try_recv().is_ok() {}
        state.handle_change();
    }
    Ok(())
}

struct State {
    path: PathBuf,
    competition_number: String,
    own_fix: SharedOwnFix,
    sender: broadcast::Sender<Bytes>,
    last_content: Option<Vec<u8>>,
}

impl State {
    fn handle_change(&mut self) {
        let content = match std::fs::read(&self.path) {
            Ok(content) => content,
            Err(error) => {
                debug!(%error, "Spectate file is not readable yet");
                return;
            }
        };
        if self.last_content.as_ref() == Some(&content) {
            return;
        }
        let players = match spectate::parse_snapshot(&content) {
            Ok(players) => players,
            Err(error) => {
                debug!(%error, "Spectate file is incomplete, waiting for the next write");
                return;
            }
        };
        if self.last_content.is_none() {
            info!("Spectate file found with {} players", players.len());
        }
        self.last_content = Some(content);

        let own = spectate::own_player(&players, &self.competition_number);
        let reference = match own {
            Some(own) => Reference {
                position: own.position,
                altitude: own.altitude,
            },
            None => match nmea_reference(&self.own_fix) {
                Some(reference) => reference,
                None => {
                    debug!("no own position for the Spectate snapshot, skipping it");
                    return;
                }
            },
        };
        let output = spectate::traffic_sentences(&players, own, reference);
        // No client is connected: nothing to deliver.
        let _ = self.sender.send(Bytes::from(output));
    }
}

fn nmea_reference(own_fix: &SharedOwnFix) -> Option<Reference> {
    let own_fix = own_fix
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let fix = own_fix.as_ref()?;
    (fix.at.elapsed() < OWN_FIX_MAX_AGE).then_some(Reference {
        position: fix.position,
        altitude: fix.altitude,
    })
}

/// The file Condor 3 writes during a multiplayer session.
pub fn spectate_path(condor_folder: &Path) -> PathBuf {
    condor_folder.join("Logs").join("Spectate.json")
}
