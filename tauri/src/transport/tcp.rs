use super::reconnect::ReconnectBackoff;
use crate::driver::{DriverHandle, StopFn};
#[cfg(test)]
use std::time::Duration;
use tokio::io::AsyncReadExt as _;
use tokio::net::TcpStream;
use tokio::sync::oneshot;
use updraft_core::{ConnectionState, ExternalDeviceId, Input};

const READ_BUFFER_BYTES: usize = 4_096;

/// Maintains a TCP link until the returned stop function is invoked.
///
/// The core asked for this link to exist, so reconnection and backoff are
/// this task's business, not the core's. The core only learns the current
/// state through [`Input::ConnectionChanged`].
pub fn run(device_id: ExternalDeviceId, host: String, port: u16, handle: DriverHandle) -> StopFn {
    let (stop_sender, mut stop_receiver) = oneshot::channel();

    tokio::spawn(async move {
        let mut backoff = ReconnectBackoff::default();

        loop {
            handle.send(Input::connection_changed(
                device_id,
                ConnectionState::Connecting,
            ));

            let stream = tokio::select! {
                biased;
                _ = &mut stop_receiver => {
                    handle.send(Input::connection_changed(
                        device_id,
                        ConnectionState::Disconnected,
                    ));
                    return;
                }
                result = TcpStream::connect((host.as_str(), port)) => result,
            };

            let delivered_bytes = match stream {
                Ok(stream) => {
                    handle.send(Input::connection_changed(
                        device_id,
                        ConnectionState::Connected,
                    ));
                    match pump(device_id, &host, port, stream, &handle, &mut stop_receiver).await {
                        PumpResult::Disconnected { delivered_bytes } => delivered_bytes,
                        PumpResult::Stopped => {
                            handle.send(Input::connection_changed(
                                device_id,
                                ConnectionState::Disconnected,
                            ));
                            return;
                        }
                    }
                }
                Err(error) => {
                    tracing::warn!(?device_id, %host, port, %error, "TCP connect failed");
                    false
                }
            };

            handle.send(Input::connection_changed(
                device_id,
                ConnectionState::Disconnected,
            ));

            tokio::select! {
                biased;
                _ = &mut stop_receiver => return,
                _ = tokio::time::sleep(backoff.after_attempt(delivered_bytes)) => {}
            }
        }
    });

    Box::new(move || {
        let _ = stop_sender.send(());
    })
}

enum PumpResult {
    Disconnected { delivered_bytes: bool },
    Stopped,
}

/// Reads until the link closes, errors, or is stopped. A disconnection
/// reports whether any bytes arrived so reconnect backoff can be reset.
async fn pump(
    device_id: ExternalDeviceId,
    host: &str,
    port: u16,
    mut stream: TcpStream,
    handle: &DriverHandle,
    stop_receiver: &mut oneshot::Receiver<()>,
) -> PumpResult {
    let mut buffer = vec![0_u8; READ_BUFFER_BYTES];
    let mut received = false;

    loop {
        let read = tokio::select! {
            biased;
            _ = &mut *stop_receiver => return PumpResult::Stopped,
            result = stream.read(&mut buffer) => result,
        };
        match read {
            Ok(0) => {
                return PumpResult::Disconnected {
                    delivered_bytes: received,
                };
            }
            Ok(read) => {
                received = true;
                handle.send(Input::bytes(device_id, &buffer[..read]));
            }
            Err(error) => {
                tracing::warn!(?device_id, %host, port, %error, "TCP read failed");
                return PumpResult::Disconnected {
                    delivered_bytes: received,
                };
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
    use updraft_core::{ConnectionSpec, ExternalDeviceConfig, SettingsSnapshot, Topic};

    const PATIENCE: Duration = Duration::from_secs(5);

    fn driver() -> DriverHandle {
        Driver::spawn(
            SettingsSnapshot::default(),
            Box::new(|_, _, _| Box::new(|| {})),
            Box::new(|_| {}),
            Duration::from_millis(100),
        )
    }

    fn assert_warning_context(
        line: &str,
        device_id: ExternalDeviceId,
        port: u16,
    ) -> Result<(), String> {
        let missing: Vec<_> = [
            format!(" device_id={device_id:?}"),
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
        let device_id = ExternalDeviceId(1);

        let _stop = run(device_id, "127.0.0.1".to_owned(), port, driver());

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
            assert_warning_context(line, device_id, port)
        });
    }

    #[tokio::test]
    #[traced_test]
    async fn failed_read_logs_connection_endpoint_and_error() {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("binds");
        let port = listener.local_addr().expect("has an address").port();
        let device_id = ExternalDeviceId(1);

        let (client, accepted) =
            tokio::join!(TcpStream::connect(("127.0.0.1", port)), listener.accept());
        let client = client.expect("connects");
        let stream = accepted.expect("accepts").0;

        stream.set_zero_linger().expect("sets zero linger");
        drop(stream);

        let handle = driver();
        let (_stop_sender, mut stop_receiver) = oneshot::channel();
        timeout(
            PATIENCE,
            pump(
                device_id,
                "127.0.0.1",
                port,
                client,
                &handle,
                &mut stop_receiver,
            ),
        )
        .await
        .expect("a read failure within the timeout");

        logs_assert(|lines| {
            let Some(line) = lines.iter().find(|line| line.contains("TCP read failed")) else {
                return Err("missing read failure warning".to_owned());
            };
            assert_warning_context(line, device_id, port)
        });
    }

    #[tokio::test]
    async fn bytes_from_a_listening_peer_reach_the_core() {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("binds");
        let port = listener.local_addr().expect("has an address").port();
        let handle = Driver::spawn(
            SettingsSnapshot {
                settings: Default::default(),
                external_devices: vec![ExternalDeviceConfig {
                    enabled: true,
                    spec: ConnectionSpec::tcp("127.0.0.1", port),
                }],
            },
            Box::new(|_, _, _| Box::new(|| {})),
            Box::new(|_| {}),
            Duration::from_millis(100),
        );

        let (sender, mut topics) = mpsc::unbounded_channel();
        handle.subscribe(Box::new(move |topic: &Topic| {
            sender.send(topic.clone()).is_ok()
        }));

        let device_id = loop {
            let received = timeout(PATIENCE, topics.recv())
                .await
                .expect("a topic within the timeout");
            let Topic::ExternalDevices(devices) = assert_some!(received) else {
                continue;
            };
            break devices[0].device_id;
        };
        let _stop = run(device_id, "127.0.0.1".to_owned(), port, handle.clone());

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

    #[tokio::test(start_paused = true)]
    async fn stopping_tcp_drops_the_active_socket_and_prevents_reconnect() {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("binds");
        let port = listener.local_addr().expect("has an address").port();
        let stop = run(ExternalDeviceId(1), "127.0.0.1".to_owned(), port, driver());
        let (mut peer, _) = timeout(PATIENCE, listener.accept())
            .await
            .expect("a connection within the timeout")
            .expect("accepts");

        stop();

        let mut byte = [0];
        assert_eq!(
            timeout(PATIENCE, peer.read(&mut byte))
                .await
                .expect("the peer closes within the timeout")
                .expect("reads EOF"),
            0
        );

        tokio::time::advance(Duration::from_secs(11)).await;
        assert!(
            timeout(Duration::from_millis(1), listener.accept())
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn two_tcp_workers_connect_independently() {
        let first = TcpListener::bind("127.0.0.1:0").await.expect("binds");
        let second = TcpListener::bind("127.0.0.1:0").await.expect("binds");
        let first_port = first.local_addr().expect("has an address").port();
        let second_port = second.local_addr().expect("has an address").port();
        let handle = driver();

        let stop_first = run(
            ExternalDeviceId(1),
            "127.0.0.1".to_owned(),
            first_port,
            handle.clone(),
        );
        let stop_second = run(
            ExternalDeviceId(2),
            "127.0.0.1".to_owned(),
            second_port,
            handle,
        );

        let (first_result, second_result) = timeout(PATIENCE, async {
            tokio::join!(first.accept(), second.accept())
        })
        .await
        .expect("both connections within the timeout");
        first_result.expect("first listener accepts");
        second_result.expect("second listener accepts");

        stop_first();
        stop_second();
    }

    #[tokio::test(start_paused = true)]
    async fn stopping_at_the_reconnect_boundary_prevents_another_tcp_attempt() {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("binds");
        let port = listener.local_addr().expect("has an address").port();
        let stop = run(ExternalDeviceId(1), "127.0.0.1".to_owned(), port, driver());
        let (peer, _) = listener.accept().await.expect("accepts");
        drop(peer);
        tokio::task::yield_now().await;

        stop();
        tokio::time::advance(Duration::from_millis(250)).await;
        tokio::task::yield_now().await;

        assert!(
            timeout(Duration::from_millis(1), listener.accept())
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn stopping_before_tcp_connect_prevents_an_attempt() {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("binds");
        let port = listener.local_addr().expect("has an address").port();
        let stop = run(ExternalDeviceId(1), "127.0.0.1".to_owned(), port, driver());

        stop();
        tokio::task::yield_now().await;

        assert!(
            timeout(Duration::from_millis(1), listener.accept())
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn stopping_with_bytes_ready_does_not_deliver_them() {
        const RMC: &[u8] = b"$GPRMC,120000.00,A,5049.38,N,00611.16,E,45.0,270.0,010126,,,A\r\n";

        let listener = TcpListener::bind("127.0.0.1:0").await.expect("binds");
        let port = listener.local_addr().expect("has an address").port();
        let handle = Driver::spawn(
            SettingsSnapshot {
                settings: Default::default(),
                external_devices: vec![ExternalDeviceConfig {
                    enabled: true,
                    spec: ConnectionSpec::tcp("127.0.0.1", port),
                }],
            },
            Box::new(|_, _, _| Box::new(|| {})),
            Box::new(|_| {}),
            Duration::from_millis(100),
        );
        let (sender, mut topics) = mpsc::unbounded_channel();
        handle.subscribe(Box::new(move |topic: &Topic| {
            sender.send(topic.clone()).is_ok()
        }));
        let device_id = loop {
            let received = timeout(PATIENCE, topics.recv())
                .await
                .expect("a topic within the timeout");
            let Topic::ExternalDevices(devices) = assert_some!(received) else {
                continue;
            };
            break devices[0].device_id;
        };
        let stop = run(device_id, "127.0.0.1".to_owned(), port, handle.clone());
        let (peer, _) = listener.accept().await.expect("accepts");

        peer.writable().await.expect("peer becomes writable");
        assert_eq!(
            peer.try_write(RMC).expect("writes without yielding"),
            RMC.len()
        );
        stop();
        tokio::task::yield_now().await;

        let (sender, mut current) = mpsc::unbounded_channel();
        handle.subscribe(Box::new(move |topic: &Topic| {
            sender.send(topic.clone()).is_ok()
        }));
        loop {
            let received = timeout(PATIENCE, current.recv())
                .await
                .expect("current topics within the timeout");
            let Topic::Instruments(instruments) = assert_some!(received) else {
                continue;
            };
            assert!(instruments.position.is_none());
            break;
        }
    }
}
