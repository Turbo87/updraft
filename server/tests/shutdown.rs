//! Spawns the real server binary and checks that it shuts down cleanly on
//! the termination signals. Signal delivery is Unix-only, so these tests
//! don't cover the Windows Ctrl-C path.
#![cfg(unix)]

use claims::assert_some;
use std::io::{BufRead, BufReader};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::time::{Duration, Instant};
use tempfile::TempDir;

/// Kills the server on drop so a failed assertion doesn't leak the process.
struct ServerProcess {
    child: Child,
    _static_dir: TempDir,
}

impl Drop for ServerProcess {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl ServerProcess {
    /// Spawns the server on an ephemeral port and waits until it reports
    /// that it is listening.
    fn spawn() -> Self {
        let static_dir = tempfile::tempdir().expect("failed to create temporary directory");

        let mut child = Command::new(env!("CARGO_BIN_EXE_updraft_server"))
            .args(["--ip", "127.0.0.1", "--port", "0", "--static-dir"])
            .arg(static_dir.path())
            .stderr(Stdio::piped())
            .spawn()
            .expect("failed to spawn server");

        // The server reports its readiness through `tracing`, which writes to
        // stderr, so scan stderr until the startup line appears.
        let stderr = child
            .stderr
            .take()
            .expect("failed to capture server stderr");
        let mut reader = BufReader::new(stderr);
        let mut line = String::new();
        loop {
            line.clear();
            let read = reader
                .read_line(&mut line)
                .expect("failed to read server startup output");
            assert_ne!(read, 0, "server exited before reporting readiness");
            if line.contains("listening on http://") {
                break;
            }
        }

        Self {
            child,
            _static_dir: static_dir,
        }
    }

    fn send_signal(&self, signal: &str) {
        let status = Command::new("kill")
            .args(["-s", signal, &self.child.id().to_string()])
            .status()
            .expect("failed to send signal");
        assert!(status.success(), "kill -s {signal} failed: {status}");
    }

    fn wait_for_exit(&mut self, timeout: Duration) -> Option<ExitStatus> {
        let deadline = Instant::now() + timeout;
        while Instant::now() < deadline {
            if let Some(status) = self
                .child
                .try_wait()
                .expect("failed to poll server process")
            {
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

    let status = assert_some!(
        server.wait_for_exit(Duration::from_secs(10)),
        "server did not exit within timeout after SIG{signal}"
    );
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
