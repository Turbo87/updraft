//! The TCP listener where HW VSP3 delivers the Condor NMEA stream.

use crate::nmea::{self, SharedOwnFix};
use bytes::Bytes;
use std::io;
use std::net::SocketAddr;
use std::time::{Duration, Instant};
use tokio::io::AsyncReadExt as _;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::broadcast;
use tokio::task::JoinHandle;
use tracing::{info, warn};

/// Condor sends one sentence set per second. Silence this long means the
/// stream is not running.
const IDLE_WARNING: Duration = Duration::from_secs(10);

/// Accepts one source at a time. A new connection replaces the old one.
pub async fn run(
    listener: TcpListener,
    own_fix: SharedOwnFix,
    sender: broadcast::Sender<Bytes>,
) -> io::Result<()> {
    let mut current: Option<JoinHandle<()>> = None;
    loop {
        let (stream, peer) = listener.accept().await?;
        if let Some(previous) = current.take() {
            previous.abort();
            info!("replaced the previous Condor NMEA source");
        }
        info!(%peer, "Condor NMEA source connected");
        current = Some(tokio::spawn(read_source(
            stream,
            peer,
            own_fix.clone(),
            sender.clone(),
        )));
    }
}

async fn read_source(
    mut stream: TcpStream,
    peer: SocketAddr,
    own_fix: SharedOwnFix,
    sender: broadcast::Sender<Bytes>,
) {
    let mut buffer = Vec::new();
    let mut chunk = [0u8; 4096];
    let mut idle_warned = false;
    loop {
        match tokio::time::timeout(IDLE_WARNING, stream.read(&mut chunk)).await {
            Err(_) => {
                if !idle_warned {
                    warn!(
                        "no Condor NMEA data for {} s, check the NMEA output and the NOTAM PDA option",
                        IDLE_WARNING.as_secs()
                    );
                    idle_warned = true;
                }
            }
            Ok(Ok(0)) => {
                info!(%peer, "Condor NMEA source disconnected");
                return;
            }
            Ok(Ok(count)) => {
                idle_warned = false;
                buffer.extend_from_slice(&chunk[..count]);
                let output = {
                    let mut own = own_fix
                        .lock()
                        .unwrap_or_else(|poisoned| poisoned.into_inner());
                    nmea::process(&mut buffer, &mut own, Instant::now())
                };
                if !output.is_empty() {
                    // No client is connected: nothing to deliver.
                    let _ = sender.send(Bytes::from(output));
                }
            }
            Ok(Err(error)) => {
                warn!(%peer, %error, "Condor NMEA source read failed");
                return;
            }
        }
    }
}
