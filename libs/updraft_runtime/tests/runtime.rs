//! Runtime invariant tests: the stream contract (atomic subscribe, FIFO
//! ordering, slow-subscriber drops) and the worker lifecycle
//! (panic-to-`ComputeFailed`, recovery, epoch handling).

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use updraft_core::{
    App, AppConfig, Change, Command, ComputeJob, ComputeKind, ComputeResult, Input, Observation,
    PositionFix, Query, QueryResult,
};
use updraft_geo::LatLon;
use updraft_runtime::{Handle, PureWorker, Runtime, Worker};
use updraft_units::Length;

const TIMEOUT: Duration = Duration::from_secs(10);

/// An app whose compute throttle is short enough for wall-clock tests.
fn app() -> App {
    App::with_config(AppConfig {
        trace_stats_interval: Duration::from_millis(50),
    })
}

fn fix(handle: &Handle, latitude: f64) -> PositionFix {
    PositionFix {
        observed_at: handle.now(),
        position: LatLon::from_degrees(latitude, 6.),
        altitude: Some(Length::from_meters(1000.)),
        track: None,
        ground_speed: None,
    }
}

fn submit_fix(handle: &Handle, latitude: f64) -> PositionFix {
    let fix = fix(handle, latitude);
    handle
        .submit(Input::Observation(Observation::Position(fix)))
        .expect("runtime is running");
    fix
}

#[test]
fn atomic_subscribe_and_fifo_ordering() {
    let runtime = Runtime::builder(app())
        .worker(ComputeKind::TraceStats, PureWorker)
        .start();
    let handle = runtime.handle();

    let first = submit_fix(&handle, 50.);

    // The subscription request is ordered behind the first fix on the
    // same queue, so its snapshot must already contain it.
    let subscription = handle.subscribe().unwrap();
    assert_eq!(subscription.snapshot.position, Some(first));

    let second = submit_fix(&handle, 50.01);
    let third = submit_fix(&handle, 50.02);

    let mut positions = Vec::new();
    let deadline = Instant::now() + TIMEOUT;
    while positions.len() < 2 {
        let remaining = deadline.saturating_duration_since(Instant::now());
        let changes = subscription.changes.recv_timeout(remaining).unwrap();
        positions.extend(changes.into_iter().filter_map(|change| match change {
            Change::Position(fix) => Some(fix),
            Change::TraceStats(_) => None,
        }));
    }
    assert_eq!(
        positions,
        vec![second, third],
        "changes arrive in input order"
    );

    runtime.shutdown();
}

#[test]
fn slow_subscriber_is_dropped() {
    let runtime = Runtime::builder(app())
        .worker(ComputeKind::TraceStats, PureWorker)
        .subscriber_buffer_len(1)
        .start();
    let handle = runtime.handle();

    let subscription = handle.subscribe().unwrap();
    for i in 0..10 {
        submit_fix(&handle, 50. + f64::from(i) * 0.01);
    }

    // Without reading, the one-batch buffer overflows and the runtime
    // drops the subscription: the receiver disconnects after the batches
    // that fit.
    let deadline = Instant::now() + TIMEOUT;
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        match subscription.changes.recv_timeout(remaining) {
            Ok(_) => continue,
            Err(err) => {
                assert_eq!(err, std::sync::mpsc::RecvTimeoutError::Disconnected);
                break;
            }
        }
    }
    assert!(handle.metrics().slow_subscriber_drops() >= 1);

    // Reconnect is resubscribe: a fresh subscription works and starts
    // from a fresh snapshot.
    let subscription = handle.subscribe().unwrap();
    assert!(subscription.snapshot.position.is_some());

    runtime.shutdown();
}

#[test]
fn worker_computes_trace_stats() {
    let runtime = Runtime::builder(app())
        .worker(ComputeKind::TraceStats, PureWorker)
        .start();
    let handle = runtime.handle();

    let subscription = handle.subscribe().unwrap();
    submit_fix(&handle, 50.);
    submit_fix(&handle, 50.1);

    let stats = wait_for_stats(&subscription).expect("worker returns statistics");
    assert!(stats.fix_count >= 1);

    runtime.shutdown();
}

/// A worker whose first job panics. Later jobs succeed.
struct PanicsOnce {
    panicked: bool,
}

impl Worker for PanicsOnce {
    fn run(&mut self, job: ComputeJob) -> Result<ComputeResult, String> {
        if !self.panicked {
            self.panicked = true;
            panic!("intentional test panic");
        }
        Ok(job.run())
    }
}

#[test]
fn worker_panic_becomes_typed_failure_and_recovers() {
    let runtime = Runtime::builder(app())
        .worker(ComputeKind::TraceStats, PanicsOnce { panicked: false })
        .start();
    let handle = runtime.handle();

    let subscription = handle.subscribe().unwrap();
    // The first fix starts the job that panics. The second marks the slot
    // pending again, so the core schedules a retry that must succeed.
    submit_fix(&handle, 50.);
    submit_fix(&handle, 50.1);

    let stats = wait_for_stats(&subscription).expect("runtime recovers from the panic");
    assert_eq!(stats.fix_count, 2);
    assert_eq!(handle.metrics().worker_failures(), 1);

    runtime.shutdown();
}

/// A worker whose first `reset` panics. Later resets and runs succeed.
struct ResetPanicsOnce {
    panicked: bool,
}

impl Worker for ResetPanicsOnce {
    fn run(&mut self, job: ComputeJob) -> Result<ComputeResult, String> {
        Ok(job.run())
    }

    fn reset(&mut self) {
        if !self.panicked {
            self.panicked = true;
            panic!("intentional reset panic");
        }
    }
}

#[test]
fn worker_reset_panic_becomes_typed_failure_and_recovers() {
    let runtime = Runtime::builder(app())
        .worker(ComputeKind::TraceStats, ResetPanicsOnce { panicked: false })
        .start();
    let handle = runtime.handle();

    let subscription = handle.subscribe().unwrap();
    // The first job resets the worker before running, and that reset
    // panics, which must free the job slot instead of stalling the kind.
    submit_fix(&handle, 50.);
    submit_fix(&handle, 50.1);

    let stats = wait_for_stats(&subscription).expect("runtime recovers from the reset panic");
    assert_eq!(stats.fix_count, 2);
    assert_eq!(handle.metrics().worker_failures(), 1);

    runtime.shutdown();
}

/// A worker that counts how often the runtime resets it.
struct CountingResets {
    resets: Arc<AtomicUsize>,
}

impl Worker for CountingResets {
    fn run(&mut self, job: ComputeJob) -> Result<ComputeResult, String> {
        Ok(job.run())
    }

    fn reset(&mut self) {
        self.resets.fetch_add(1, Ordering::SeqCst);
    }
}

#[test]
fn worker_is_reset_when_the_epoch_changes() {
    let resets = Arc::new(AtomicUsize::new(0));
    let runtime = Runtime::builder(app())
        .worker(
            ComputeKind::TraceStats,
            CountingResets {
                resets: Arc::clone(&resets),
            },
        )
        .start();
    let handle = runtime.handle();

    let subscription = handle.subscribe().unwrap();
    submit_fix(&handle, 50.);
    wait_for_stats(&subscription).expect("first job completes");
    let before_clear = resets.load(Ordering::SeqCst);

    // Clearing the trace bumps the epoch. The next job runs under a new
    // epoch and must reset the worker's cache before it does.
    handle
        .submit(Input::Command(Command::ClearTrace))
        .expect("runtime is running");
    submit_fix(&handle, 50.1);

    let deadline = Instant::now() + TIMEOUT;
    while resets.load(Ordering::SeqCst) <= before_clear {
        assert!(
            Instant::now() < deadline,
            "worker was not reset on epoch change"
        );
        std::thread::yield_now();
    }

    runtime.shutdown();
}

#[test]
fn missing_worker_fails_the_job_without_stalling() {
    // No worker registered: every job fails immediately, and the core
    // keeps running.
    let runtime = Runtime::builder(app()).start();
    let handle = runtime.handle();

    submit_fix(&handle, 50.);

    let deadline = Instant::now() + TIMEOUT;
    while handle.metrics().worker_failures() == 0 {
        assert!(Instant::now() < deadline, "job failure was never recorded");
        std::thread::yield_now();
    }

    let QueryResult::Position(position) = handle.query(Query::Position).unwrap() else {
        panic!("unexpected query result");
    };
    assert!(position.is_some(), "the core still answers queries");

    runtime.shutdown();
}

#[test]
#[should_panic(expected = "already registered for TraceStats")]
fn duplicate_worker_kind_is_rejected() {
    let _ = Runtime::builder(app())
        .worker(ComputeKind::TraceStats, PureWorker)
        .worker(ComputeKind::TraceStats, PureWorker);
}

#[test]
fn handle_reports_runtime_stopped_after_shutdown() {
    let runtime = Runtime::builder(app()).start();
    let handle = runtime.handle();
    runtime.shutdown();

    let input = Input::Clock { now: handle.now() };
    assert!(handle.submit(input).is_err());
    assert!(handle.query(Query::Position).is_err());
    assert!(handle.subscribe().is_err());
}

#[test]
fn dropping_the_runtime_stops_the_core() {
    let runtime = Runtime::builder(app())
        .worker(ComputeKind::TraceStats, PureWorker)
        .start();
    let handle = runtime.handle();
    submit_fix(&handle, 50.);

    // Dropping the runtime without an explicit shutdown must still stop
    // the core loop and join its threads, not leave them running.
    drop(runtime);

    let input = Input::Clock { now: handle.now() };
    assert!(handle.submit(input).is_err());
}

fn wait_for_stats(
    subscription: &updraft_runtime::Subscription,
) -> Option<updraft_core::TraceStats> {
    let deadline = Instant::now() + TIMEOUT;
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        let changes = subscription.changes.recv_timeout(remaining).ok()?;
        for change in changes {
            if let Change::TraceStats(Some(stats)) = change {
                return Some(stats);
            }
        }
    }
}
