//! The TCP server that sends the merged NMEA stream to every client.

use bytes::Bytes;
use std::io;
use std::net::SocketAddr;
use tokio::io::AsyncWriteExt as _;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::broadcast;
use tracing::{info, warn};

pub async fn run(listener: TcpListener, sender: broadcast::Sender<Bytes>) -> io::Result<()> {
    loop {
        let (stream, peer) = listener.accept().await?;
        let receiver = sender.subscribe();
        info!(%peer, "Updraft client connected");
        tokio::spawn(write_client(stream, peer, receiver));
    }
}

async fn write_client(
    mut stream: TcpStream,
    peer: SocketAddr,
    mut receiver: broadcast::Receiver<Bytes>,
) {
    loop {
        match receiver.recv().await {
            Ok(payload) => {
                if let Err(error) = stream.write_all(&payload).await {
                    warn!(%peer, %error, "Updraft client write failed");
                    return;
                }
            }
            Err(broadcast::error::RecvError::Lagged(skipped)) => {
                warn!(%peer, skipped, "Updraft client fell behind, dropping it");
                return;
            }
            Err(broadcast::error::RecvError::Closed) => return,
        }
    }
}
