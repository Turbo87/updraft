use super::reconnect::ReconnectBackoff;
use crate::driver::DriverHandle;
#[cfg(test)]
use std::time::Duration;
use tokio::io::AsyncReadExt as _;
use tokio::net::TcpStream;
use updraft_core::{ConnectionId, ConnectionState, Input};

const READ_BUFFER_BYTES: usize = 4_096;

/// Maintains a TCP link until the process ends.
///
/// The core asked for this link to exist, so reconnection and backoff are
/// this task's business, not the core's. The core only learns the current
/// state through [`Input::ConnectionChanged`].
pub fn run(connection: ConnectionId, host: String, port: u16, handle: DriverHandle) {
    tokio::spawn(async move {
        let mut backoff = ReconnectBackoff::default();

        loop {
            handle.send(Input::connection_changed(
                connection,
                ConnectionState::Connecting,
            ));

            let delivered_bytes = match TcpStream::connect((host.as_str(), port)).await {
                Ok(stream) => {
                    handle.send(Input::connection_changed(
                        connection,
                        ConnectionState::Connected,
                    ));
                    pump(connection, &host, port, stream, &handle).await
                }
                Err(error) => {
                    tracing::warn!(?connection, %host, port, %error, "TCP connect failed");
                    false
                }
            };

            handle.send(Input::connection_changed(
                connection,
                ConnectionState::Disconnected,
            ));

            tokio::time::sleep(backoff.after_attempt(delivered_bytes)).await;
        }
    });
}

/// Reads until the link closes or errors. Returns whether any bytes
/// arrived, which is what tells the caller the connection was real.
async fn pump(
    connection: ConnectionId,
    host: &str,
    port: u16,
    mut stream: TcpStream,
    handle: &DriverHandle,
) -> bool {
    let mut buffer = vec![0_u8; READ_BUFFER_BYTES];
    let mut received = false;

    loop {
        match stream.read(&mut buffer).await {
            Ok(0) => return received,
            Ok(read) => {
                received = true;
                handle.send(Input::bytes(connection, &buffer[..read]));
            }
            Err(error) => {
                tracing::warn!(?connection, %host, port, %error, "TCP read failed");
                return received;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::driver::Driver;
    use claims::assert_some;
    use tokio::io::AsyncWriteExt as _;
    use tokio::net::TcpListener;
    use tokio::sync::mpsc;
    use tokio::time::{sleep, timeout};
    use tracing_test::traced_test;
    use updraft_core::{ConnectionSpec, CoreConfig, Topic};

    const PATIENCE: Duration = Duration::from_secs(5);

    fn driver() -> DriverHandle {
        Driver::spawn(
            CoreConfig::default(),
            Box::new(|_, _, _| {}),
            Duration::from_millis(100),
        )
    }

    fn assert_warning_context(
        line: &str,
        connection: ConnectionId,
        port: u16,
    ) -> Result<(), String> {
        let missing: Vec<_> = [
            format!(" connection={connection:?}"),
            " host=127.0.0.1".to_owned(),
            format!(" port={port}"),
            " error=".to_owned(),
        ]
        .into_iter()
        .filter(|field| !line.contains(field))
        .collect();

        if missing.is_empty() {
            Ok(())
        } else {
            Err(format!("warning lacks {missing:?}: {line:?}"))
        }
    }

    #[tokio::test]
    #[traced_test]
    async fn failed_connect_logs_connection_endpoint_and_error() {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("binds");
        let port = listener.local_addr().expect("has an address").port();
        drop(listener);
        let connection = ConnectionId(1);

        run(connection, "127.0.0.1".to_owned(), port, driver());

        timeout(PATIENCE, async {
            while !logs_contain("TCP connect failed") {
                sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("a connection failure warning within the timeout");

        logs_assert(|lines| {
            let Some(line) = lines
                .iter()
                .find(|line| line.contains("TCP connect failed"))
            else {
                return Err("missing connection failure warning".to_owned());
            };
            assert_warning_context(line, connection, port)
        });
    }

    #[tokio::test]
    #[traced_test]
    async fn failed_read_logs_connection_endpoint_and_error() {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("binds");
        let port = listener.local_addr().expect("has an address").port();
        let connection = ConnectionId(1);

        let (client, accepted) =
            tokio::join!(TcpStream::connect(("127.0.0.1", port)), listener.accept());
        let client = client.expect("connects");
        let stream = accepted.expect("accepts").0;

        stream.set_zero_linger().expect("sets zero linger");
        drop(stream);

        let handle = driver();
        timeout(
            PATIENCE,
            pump(connection, "127.0.0.1", port, client, &handle),
        )
        .await
        .expect("a read failure within the timeout");

        logs_assert(|lines| {
            let Some(line) = lines.iter().find(|line| line.contains("TCP read failed")) else {
                return Err("missing read failure warning".to_owned());
            };
            assert_warning_context(line, connection, port)
        });
    }

    #[tokio::test]
    async fn bytes_from_a_listening_peer_reach_the_core() {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("binds");
        let port = listener.local_addr().expect("has an address").port();
        let connection = ConnectionId(1);

        let handle = Driver::spawn(
            CoreConfig {
                connections: vec![(connection, ConnectionSpec::tcp("127.0.0.1", port))],
                ..CoreConfig::default()
            },
            Box::new(|_, _, _| {}),
            Duration::from_millis(100),
        );

        let (sender, mut topics) = mpsc::unbounded_channel();
        handle.subscribe(Box::new(move |topic: &Topic| {
            sender.send(topic.clone()).is_ok()
        }));

        run(connection, "127.0.0.1".to_owned(), port, handle.clone());

        let (mut stream, _) = listener.accept().await.expect("accepts");
        stream
            .write_all(b"$GPRMC,120000.00,A,5049.38,N,00611.16,E,45.0,270.0,010126,,,A\r\n")
            .await
            .expect("writes");

        loop {
            let received = timeout(PATIENCE, topics.recv())
                .await
                .expect("a topic within the timeout");
            let Topic::Instruments(instruments) = assert_some!(received) else {
                continue;
            };
            if instruments.position.is_some() {
                return;
            }
        }
    }
}
