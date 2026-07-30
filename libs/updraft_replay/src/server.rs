use std::io;
use std::net::SocketAddr;

use bytes::Bytes;
use tokio::io::AsyncWriteExt as _;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::broadcast;
use tracing::{info, warn};

pub async fn run(listener: TcpListener, sender: broadcast::Sender<Bytes>) -> io::Result<()> {
    loop {
        let (stream, peer) = listener.accept().await?;
        let receiver = sender.subscribe();
        info!(%peer, "NMEA replay client connected");
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
                    warn!(%peer, %error, "NMEA replay client write failed");
                    return;
                }
            }
            Err(broadcast::error::RecvError::Lagged(skipped)) => {
                warn!(%peer, skipped, "NMEA replay client lagged");
                return;
            }
            Err(broadcast::error::RecvError::Closed) => {
                info!(%peer, "NMEA replay client disconnected");
                return;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::replay::Replay;
    use claims::assert_ok;
    use std::time::Duration;
    use tokio::io::AsyncReadExt as _;
    use tokio::net::{TcpListener, TcpStream};
    use tokio::sync::broadcast;

    const BASIC: &[u8] = include_bytes!("../../../testdata/nmea/basic.nmea");

    #[tokio::test(start_paused = true)]
    async fn broadcasts_the_current_replay_position_to_all_clients() {
        // Keep Tokio from advancing paused time while real sockets connect.
        let keep_time_paused = tokio::spawn(async {
            loop {
                tokio::task::yield_now().await;
            }
        });
        let replay = assert_ok!(Replay::from_bytes(BASIC.to_vec()));
        let second = replay.events()[1].payload().to_vec();
        let third = replay.events()[2].payload().to_vec();

        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("binds test listener");
        let address = listener.local_addr().expect("has local address");
        let (sender, _) = broadcast::channel(8);

        let server_sender = sender.clone();
        let server = tokio::spawn(async move { run(listener, server_sender).await });

        let playback_sender = sender.clone();
        let playback =
            tokio::spawn(async move { replay.play(playback_sender, Duration::ZERO, false).await });

        tokio::task::yield_now().await;
        let mut first = TcpStream::connect(address)
            .await
            .expect("connects first client");
        tokio::task::yield_now().await;

        tokio::time::advance(Duration::from_secs(1)).await;
        let mut received = vec![0; second.len()];
        first
            .read_exact(&mut received)
            .await
            .expect("reads second event");
        assert_eq!(received, second);

        let mut second_client = TcpStream::connect(address)
            .await
            .expect("connects second client");
        tokio::task::yield_now().await;

        tokio::time::advance(Duration::from_secs(1)).await;
        let mut first_received = vec![0; third.len()];
        let mut second_received = vec![0; third.len()];
        first
            .read_exact(&mut first_received)
            .await
            .expect("first client reads third event");
        second_client
            .read_exact(&mut second_received)
            .await
            .expect("second client reads third event");
        assert_eq!(first_received, third);
        assert_eq!(second_received, third);

        server.abort();
        playback.abort();
        keep_time_paused.abort();
    }
}
