//! Spawns the real server binary and checks that it shuts down cleanly on
//! the termination signals. Signal delivery is Unix-only, so these tests
//! don't cover the Windows Ctrl-C path.
#![cfg(unix)]

use std::io::{BufRead, BufReader};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::time::{Duration, Instant};

/// Kills the server on drop so a failed assertion doesn't leak the process.
struct ServerProcess(Child);

impl Drop for ServerProcess {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

impl ServerProcess {
    /// Spawns the server on an ephemeral port and waits until it reports
    /// that it is listening.
    fn spawn() -> Self {
        let dir = tempfile::tempdir().expect("create tempdir");

        let mut child = Command::new(env!("CARGO_BIN_EXE_updraft_server"))
            .args(["--ip", "127.0.0.1", "--port", "0", "--static-dir"])
            .arg(dir.path())
            .stdout(Stdio::piped())
            .spawn()
            .expect("spawn server");

        let stdout = child.stdout.take().expect("capture stdout");
        let mut line = String::new();
        BufReader::new(stdout)
            .read_line(&mut line)
            .expect("read startup line");
        assert!(
            line.starts_with("listening on "),
            "unexpected startup output: {line:?}"
        );

        Self(child)
    }

    fn send_signal(&self, signal: &str) {
        let status = Command::new("kill")
            .args(["-s", signal, &self.0.id().to_string()])
            .status()
            .expect("send signal");
        assert!(status.success(), "kill -s {signal} failed: {status}");
    }

    fn wait_for_exit(&mut self, timeout: Duration) -> Option<ExitStatus> {
        let deadline = Instant::now() + timeout;
        while Instant::now() < deadline {
            if let Some(status) = self.0.try_wait().expect("poll server process") {
                return Some(status);
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        None
    }
}

fn assert_clean_shutdown_on(signal: &str) {
    let mut server = ServerProcess::spawn();

    server.send_signal(signal);

    let status = server
        .wait_for_exit(Duration::from_secs(10))
        .unwrap_or_else(|| panic!("server did not exit within timeout after SIG{signal}"));
    assert!(status.success(), "server exited unsuccessfully: {status}");
}

#[test]
fn shuts_down_cleanly_on_sigterm() {
    assert_clean_shutdown_on("TERM");
}

#[test]
fn shuts_down_cleanly_on_sigint() {
    assert_clean_shutdown_on("INT");
}
