//! The UDP socket where Condor sends its telemetry datagrams.

use crate::udp::{self, MacCreadyEmitter};
use bytes::Bytes;
use std::io;
use std::time::Instant;
use tokio::net::UdpSocket;
use tokio::sync::broadcast;
use tracing::info;

pub async fn run(socket: UdpSocket, sender: broadcast::Sender<Bytes>) -> io::Result<()> {
    let mut datagram = [0u8; 4096];
    let mut emitter = MacCreadyEmitter::default();
    let mut started = false;
    loop {
        let (count, _) = socket.recv_from(&mut datagram).await?;
        if !started {
            info!("Condor UDP stream started");
            started = true;
        }
        let Some(value) = udp::mac_cready(&datagram[..count]) else {
            continue;
        };
        if let Some(sentence) = emitter.offer(value, Instant::now()) {
            // No client is connected: nothing to deliver.
            let _ = sender.send(Bytes::from(sentence));
        }
    }
}
