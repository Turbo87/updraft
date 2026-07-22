//! Runtime invariant tests: the stream contract (atomic subscribe, FIFO
//! ordering, slow-subscriber drops) and the worker lifecycle
//! (panic-to-`ComputeFailed`, recovery, revision handling).

use claims::{
    assert_err, assert_err_eq, assert_ge, assert_lt, assert_none, assert_ok, assert_some,
    assert_some_eq,
};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::{Duration, Instant};
use updraft_core::flight::{
    FlightChange, FlightComputeKind, FlightConfig, FlightInput, GetTraceStats, GnssUpdate,
    Observation, PositionFix, Sourced, TraceStats,
};
use updraft_core::{AppConfig, Change, ComputeJob, ComputeKind, ComputeResult, Input};
use updraft_geo::LatLon;
use updraft_runtime::{CancellationToken, WorkerResult};
use updraft_runtime::{ChangeFilter, Handle, PureWorker, Runtime, RuntimeBuilder, Worker};
use updraft_units::{Length, MslAltitude};

const TIMEOUT: Duration = Duration::from_secs(10);

#[test]
fn subscription_omits_unselected_change_groups() {
    let runtime = runtime_builder()
        .worker(trace_stats_kind(), PureWorker)
        .start();
    let handle = runtime.handle();
    let subscription = assert_ok!(handle.subscribe(ChangeFilter::only([])));

    let fix = submit_fix(&handle, 50.);
    let current = assert_ok!(handle.subscribe(ChangeFilter::only([])));
    assert_some_eq!(current.snapshot.flight.position, fix);

    // The second subscription proves that the fix was handled before this check.
    assert_err_eq!(
        subscription.changes.try_recv(),
        std::sync::mpsc::TryRecvError::Empty
    );
    assert_eq!(handle.metrics().slow_subscriber_drops(), 0);
    runtime.shutdown();
}

#[test]
fn runtime_records_queue_and_handler_measurements() {
    let runtime = Runtime::builder()
        .worker(trace_stats_kind(), PureWorker)
        .start();
    let handle = runtime.handle();

    submit_fix(&handle, 50.);
    assert_ok!(
        handle.query(GetTraceStats),
        "query orders measurement after the input"
    );

    assert_ge!(handle.metrics().max_pending_messages(), 1);
    assert_ge!(handle.metrics().queue_wait_samples(), 2);
    assert_ge!(handle.metrics().inputs_handled(), 1);
    assert_ge!(
        handle.metrics().total_handler_time(),
        handle.metrics().max_handler_time(),
        "the maximum handler duration is part of the total"
    );
    runtime.shutdown();
}

struct CancelsFirstJob {
    first: bool,
    started: std::sync::mpsc::SyncSender<()>,
    cancelled: std::sync::mpsc::SyncSender<()>,
}

impl Worker for CancelsFirstJob {
    fn run(&mut self, job: ComputeJob, cancellation: &CancellationToken) -> WorkerResult {
        if self.first {
            self.first = false;
            assert_ok!(self.started.send(()));
            while !cancellation.is_cancelled() {
                std::thread::yield_now();
            }
            assert_ok!(self.cancelled.send(()));
            WorkerResult::Cancelled
        } else {
            WorkerResult::Completed(job.run())
        }
    }
}

struct NonCooperativeWorker {
    started: std::sync::mpsc::SyncSender<()>,
    release: std::sync::mpsc::Receiver<()>,
}

impl Worker for NonCooperativeWorker {
    fn run(&mut self, job: ComputeJob, _cancellation: &CancellationToken) -> WorkerResult {
        assert_ok!(self.started.send(()));
        assert_ok!(self.release.recv());
        WorkerResult::Completed(job.run())
    }
}

struct WorkerReleaseGuard(std::sync::mpsc::Sender<()>);

impl Drop for WorkerReleaseGuard {
    fn drop(&mut self) {
        let _ = self.0.send(());
        let _ = self.0.send(());
    }
}

struct ShutdownProbe {
    started: std::sync::mpsc::SyncSender<()>,
    observed: std::sync::mpsc::SyncSender<bool>,
    release: Arc<AtomicBool>,
}

impl Worker for ShutdownProbe {
    fn run(&mut self, _job: ComputeJob, cancellation: &CancellationToken) -> WorkerResult {
        assert_ok!(self.started.send(()));
        while !cancellation.is_cancelled() && !self.release.load(Ordering::Acquire) {
            std::thread::yield_now();
        }

        let cancelled = cancellation.is_cancelled();
        assert_ok!(self.observed.send(cancelled));
        if cancelled {
            WorkerResult::Cancelled
        } else {
            WorkerResult::Failed("test released an uncancelled worker".into())
        }
    }
}

#[test]
fn shutdown_cancels_an_active_worker_job() {
    let (started_tx, started_rx) = std::sync::mpsc::sync_channel(1);
    let (observed_tx, observed_rx) = std::sync::mpsc::sync_channel(1);
    let release = Arc::new(AtomicBool::new(false));
    let runtime = runtime_builder()
        .worker(
            trace_stats_kind(),
            ShutdownProbe {
                started: started_tx,
                observed: observed_tx,
                release: Arc::clone(&release),
            },
        )
        .start();
    let handle = runtime.handle();

    submit_fix(&handle, 50.);
    assert_ok!(started_rx.recv_timeout(TIMEOUT));

    let shutdown = std::thread::spawn(move || runtime.shutdown());
    let cancelled = match observed_rx.recv_timeout(TIMEOUT) {
        Ok(cancelled) => cancelled,
        Err(_) => {
            release.store(true, Ordering::Release);
            assert_ok!(
                observed_rx.recv_timeout(TIMEOUT),
                "worker did not exit after the test released it"
            )
        }
    };
    assert_ok!(shutdown.join());

    assert!(cancelled, "shutdown did not cancel the active worker job");
}

#[test]
fn invalidating_work_cancels_the_stale_worker_job() {
    let (started_tx, started_rx) = std::sync::mpsc::sync_channel(1);
    let (cancelled_tx, cancelled_rx) = std::sync::mpsc::sync_channel(1);
    let runtime = Runtime::builder()
        .with_app_config(AppConfig {
            flight: FlightConfig {
                trace_stats_interval: Duration::ZERO,
            },
        })
        .worker(
            trace_stats_kind(),
            CancelsFirstJob {
                first: true,
                started: started_tx,
                cancelled: cancelled_tx,
            },
        )
        .start();
    let handle = runtime.handle();
    let subscription = assert_ok!(handle.subscribe(ChangeFilter::all()));

    submit_fix(&handle, 50.);
    assert_ok!(started_rx.recv_timeout(TIMEOUT));
    assert_ok!(handle.submit(Input::Flight(FlightInput::ClearTrace)));
    submit_fix(&handle, 51.);

    assert_ok!(cancelled_rx.recv_timeout(TIMEOUT));
    let stats = assert_some!(
        wait_for_stats(&subscription),
        "fresh-revision work completes"
    );
    assert_eq!(stats.fix_count, 1);
    assert_eq!(handle.metrics().worker_failures(), 0);
    runtime.shutdown();
}

#[test]
fn stale_result_from_non_cooperative_worker_is_ignored() {
    let (started_tx, started_rx) = std::sync::mpsc::sync_channel(1);
    let (release_tx, release_rx) = std::sync::mpsc::channel();
    let runtime = Runtime::builder()
        .with_app_config(AppConfig {
            flight: FlightConfig {
                trace_stats_interval: Duration::ZERO,
            },
        })
        .worker(
            trace_stats_kind(),
            NonCooperativeWorker {
                started: started_tx,
                release: release_rx,
            },
        )
        .start();
    let _release_guard = WorkerReleaseGuard(release_tx.clone());
    let handle = runtime.handle();
    let subscription = assert_ok!(handle.subscribe(ChangeFilter::all()));

    submit_fix(&handle, 50.);
    assert_ok!(started_rx.recv_timeout(TIMEOUT));

    assert_ok!(handle.submit(Input::Flight(FlightInput::ClearTrace)));
    submit_fix(&handle, 51.);
    submit_fix(&handle, 52.);

    assert_ok!(release_tx.send(()));
    assert_ok!(started_rx.recv_timeout(TIMEOUT));
    assert_ok!(release_tx.send(()));

    let stats = assert_some!(
        wait_for_stats(&subscription),
        "fresh-revision work completes"
    );
    assert_eq!(stats.fix_count, 2);
    assert_eq!(handle.metrics().worker_failures(), 0);
    runtime.shutdown();
}

struct ReturnsPreviousResult {
    previous: Option<ComputeResult>,
}

impl Worker for ReturnsPreviousResult {
    fn run(&mut self, job: ComputeJob, _cancellation: &CancellationToken) -> WorkerResult {
        let result = job.run();
        let returned = self.previous.replace(result).unwrap_or(result);
        WorkerResult::Completed(returned)
    }
}

#[test]
fn worker_result_for_another_job_is_rejected() {
    let runtime = Runtime::builder()
        .with_app_config(AppConfig {
            flight: FlightConfig {
                trace_stats_interval: Duration::ZERO,
            },
        })
        .worker(trace_stats_kind(), ReturnsPreviousResult { previous: None })
        .start();
    let handle = runtime.handle();
    let subscription = assert_ok!(handle.subscribe(ChangeFilter::all()));

    submit_fix(&handle, 50.);
    assert_some!(
        wait_for_stats(&subscription),
        "first job completes normally"
    );
    let handled_before_mismatch = handle.metrics().inputs_handled();

    assert_ok!(handle.submit(Input::Flight(FlightInput::ClearTrace)));
    submit_fix(&handle, 51.);

    let deadline = Instant::now() + TIMEOUT;
    while handle.metrics().worker_failures() == 0 {
        assert_lt!(
            Instant::now(),
            deadline,
            "mismatched worker result was not rejected"
        );
        std::thread::yield_now();
    }
    while handle.metrics().inputs_handled() < handled_before_mismatch + 3 {
        assert_lt!(
            Instant::now(),
            deadline,
            "typed worker failure did not reach the core"
        );
        std::thread::yield_now();
    }

    assert_eq!(handle.metrics().worker_failures(), 1);
    assert_none!(assert_ok!(handle.query(GetTraceStats)));
    runtime.shutdown();
}

/// A runtime builder whose compute throttle is short enough for wall-clock tests.
fn runtime_builder() -> RuntimeBuilder {
    Runtime::builder().with_app_config(AppConfig {
        flight: FlightConfig {
            trace_stats_interval: Duration::from_millis(50),
        },
    })
}

fn fix(handle: &Handle, latitude: f64) -> PositionFix {
    PositionFix {
        observed_at: handle.clock_time(),
        position: LatLon::from_degrees(latitude, 6.),
        altitude: Some(MslAltitude::new(Length::from_meters(1000.))),
        track: None,
        ground_speed: None,
    }
}

fn submit_fix(handle: &Handle, latitude: f64) -> PositionFix {
    let fix = fix(handle, latitude);
    let observation = Observation::new(
        fix.observed_at,
        GnssUpdate {
            position: fix.position,
            altitude: fix.altitude,
            track: fix.track,
            ground_speed: fix.ground_speed,
        },
    );
    assert_ok!(
        handle.submit(Input::Flight(FlightInput::Gnss(Sourced::simulator(
            observation,
        )))),
        "runtime is running"
    );
    fix
}

fn trace_stats_kind() -> ComputeKind {
    ComputeKind::Flight(FlightComputeKind::TraceStats)
}

#[test]
fn atomic_subscribe_and_fifo_ordering() {
    let runtime = runtime_builder()
        .worker(trace_stats_kind(), PureWorker)
        .start();
    let handle = runtime.handle();

    let first = submit_fix(&handle, 50.);

    // The subscription request is ordered behind the first fix on the
    // same queue, so its snapshot must already contain it.
    let subscription = assert_ok!(handle.subscribe(ChangeFilter::all()));
    assert_some_eq!(subscription.snapshot.flight.position, first);

    let second = submit_fix(&handle, 50.01);
    let third = submit_fix(&handle, 50.02);

    let mut positions = Vec::new();
    let deadline = Instant::now() + TIMEOUT;
    while positions.len() < 2 {
        let remaining = deadline.saturating_duration_since(Instant::now());
        let changes = assert_ok!(subscription.changes.recv_timeout(remaining));
        positions.extend(changes.into_iter().filter_map(|change| match change {
            Change::Flight(FlightChange::Position(fix)) => Some(fix),
            Change::Flight(FlightChange::TraceStats(_)) => None,
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
    let runtime = runtime_builder()
        .worker(trace_stats_kind(), PureWorker)
        .subscriber_buffer_capacity(1)
        .start();
    let handle = runtime.handle();

    let subscription = assert_ok!(handle.subscribe(ChangeFilter::all()));
    submit_fix(&handle, 50.);
    let latest = submit_fix(&handle, 50.01);

    // Resubscribe is ordered behind both fixes, so its snapshot proves that
    // the one-batch buffer overflowed while unread.
    let replacement = assert_ok!(handle.subscribe(ChangeFilter::all()));
    assert_some_eq!(replacement.snapshot.flight.position, latest);
    assert_ok!(subscription.changes.try_recv());
    assert_err_eq!(
        subscription.changes.try_recv(),
        std::sync::mpsc::TryRecvError::Disconnected
    );
    assert_eq!(handle.metrics().slow_subscriber_drops(), 1);

    // Reconnect is resubscribe: a fresh subscription works and starts
    // from a fresh snapshot.
    assert_some!(replacement.snapshot.flight.position);

    runtime.shutdown();
}

#[test]
fn worker_computes_trace_stats() {
    let runtime = runtime_builder()
        .worker(trace_stats_kind(), PureWorker)
        .start();
    let handle = runtime.handle();

    let subscription = assert_ok!(handle.subscribe(ChangeFilter::all()));
    submit_fix(&handle, 50.);
    submit_fix(&handle, 50.1);

    let stats = assert_some!(wait_for_stats(&subscription), "worker returns statistics");
    assert_ge!(stats.fix_count, 1);

    runtime.shutdown();
}

struct FailsOnce {
    failed: bool,
}

impl Worker for FailsOnce {
    fn run(&mut self, job: ComputeJob, _cancellation: &CancellationToken) -> WorkerResult {
        if !self.failed {
            self.failed = true;
            return WorkerResult::Failed("intentional test failure".into());
        }
        WorkerResult::Completed(job.run())
    }
}

#[test]
fn worker_failure_becomes_typed_failure_and_recovers() {
    let runtime = runtime_builder()
        .worker(trace_stats_kind(), FailsOnce { failed: false })
        .start();
    let handle = runtime.handle();
    let subscription = assert_ok!(handle.subscribe(ChangeFilter::all()));

    submit_fix(&handle, 50.);
    submit_fix(&handle, 50.1);

    let stats = assert_some!(
        wait_for_stats(&subscription),
        "runtime recovers from the failure"
    );
    assert_eq!(stats.fix_count, 2);
    assert_eq!(handle.metrics().worker_failures(), 1);

    runtime.shutdown();
}

/// A worker whose first job panics. Later jobs succeed.
struct PanicsOnce {
    panicked: bool,
}

impl Worker for PanicsOnce {
    fn run(&mut self, job: ComputeJob, _cancellation: &CancellationToken) -> WorkerResult {
        if !self.panicked {
            self.panicked = true;
            panic!("intentional test panic");
        }
        WorkerResult::Completed(job.run())
    }
}

#[test]
fn worker_panic_becomes_typed_failure_and_recovers() {
    let runtime = runtime_builder()
        .worker(trace_stats_kind(), PanicsOnce { panicked: false })
        .start();
    let handle = runtime.handle();

    let subscription = assert_ok!(handle.subscribe(ChangeFilter::all()));
    // The first fix starts the job that panics. The second marks the slot
    // pending again, so the core schedules a retry that must succeed.
    submit_fix(&handle, 50.);
    submit_fix(&handle, 50.1);

    let stats = assert_some!(
        wait_for_stats(&subscription),
        "runtime recovers from the panic"
    );
    assert_eq!(stats.fix_count, 2);
    assert_eq!(handle.metrics().worker_failures(), 1);

    runtime.shutdown();
}

/// A worker whose first `reset` panics. Later resets and runs succeed.
struct ResetPanicsOnce {
    panicked: bool,
}

impl Worker for ResetPanicsOnce {
    fn run(&mut self, job: ComputeJob, _cancellation: &CancellationToken) -> WorkerResult {
        WorkerResult::Completed(job.run())
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
    let runtime = runtime_builder()
        .worker(trace_stats_kind(), ResetPanicsOnce { panicked: false })
        .start();
    let handle = runtime.handle();

    let subscription = assert_ok!(handle.subscribe(ChangeFilter::all()));
    // The first job resets the worker before running, and that reset
    // panics, which must free the job slot instead of stalling the kind.
    submit_fix(&handle, 50.);
    submit_fix(&handle, 50.1);

    let stats = assert_some!(
        wait_for_stats(&subscription),
        "runtime recovers from the reset panic"
    );
    assert_eq!(stats.fix_count, 2);
    assert_eq!(handle.metrics().worker_failures(), 1);

    runtime.shutdown();
}

/// A worker that counts how often the runtime resets it.
struct CountingResets {
    resets: Arc<AtomicUsize>,
}

impl Worker for CountingResets {
    fn run(&mut self, job: ComputeJob, _cancellation: &CancellationToken) -> WorkerResult {
        WorkerResult::Completed(job.run())
    }

    fn reset(&mut self) {
        self.resets.fetch_add(1, Ordering::SeqCst);
    }
}

#[test]
fn worker_is_reset_when_the_revision_changes() {
    let resets = Arc::new(AtomicUsize::new(0));
    let runtime = runtime_builder()
        .worker(
            trace_stats_kind(),
            CountingResets {
                resets: Arc::clone(&resets),
            },
        )
        .start();
    let handle = runtime.handle();

    let subscription = assert_ok!(handle.subscribe(ChangeFilter::all()));
    submit_fix(&handle, 50.);
    assert_some!(wait_for_stats(&subscription), "first job completes");
    let before_clear = resets.load(Ordering::SeqCst);

    // Clearing the trace changes the compute revision. The next job runs
    // under the new revision and must reset the worker's cache before it does.
    assert_ok!(
        handle.submit(Input::Flight(FlightInput::ClearTrace)),
        "runtime is running"
    );
    submit_fix(&handle, 50.1);

    let deadline = Instant::now() + TIMEOUT;
    while resets.load(Ordering::SeqCst) <= before_clear {
        assert_lt!(
            Instant::now(),
            deadline,
            "worker was not reset on revision change"
        );
        std::thread::yield_now();
    }

    runtime.shutdown();
}

#[test]
fn missing_worker_fails_the_job_without_stalling() {
    // No worker registered: every job fails immediately, and the core
    // keeps running.
    let runtime = runtime_builder().start();
    let handle = runtime.handle();

    submit_fix(&handle, 50.);

    let deadline = Instant::now() + TIMEOUT;
    while handle.metrics().worker_failures() == 0 {
        assert_lt!(Instant::now(), deadline, "job failure was never recorded");
        std::thread::yield_now();
    }

    let subscription = assert_ok!(handle.subscribe(ChangeFilter::all()));
    assert_some!(subscription.snapshot.flight.position);

    runtime.shutdown();
}

#[test]
#[should_panic(expected = "already registered for Flight(TraceStats)")]
fn duplicate_worker_kind_is_rejected() {
    let _ = runtime_builder()
        .worker(trace_stats_kind(), PureWorker)
        .worker(trace_stats_kind(), PureWorker);
}

#[test]
fn handle_reports_runtime_stopped_after_shutdown() {
    let runtime = runtime_builder().start();
    let handle = runtime.handle();
    runtime.shutdown();

    let input = Input::Clock {
        clock_time: handle.clock_time(),
    };
    assert_err!(handle.submit(input));
    assert_err!(handle.query(GetTraceStats));
    assert_err!(handle.subscribe(ChangeFilter::all()));
}

#[test]
fn dropping_the_runtime_stops_the_core() {
    let runtime = runtime_builder()
        .worker(trace_stats_kind(), PureWorker)
        .start();
    let handle = runtime.handle();
    submit_fix(&handle, 50.);

    // Dropping the runtime without an explicit shutdown must still stop
    // the core loop and join its threads, not leave them running.
    drop(runtime);

    let input = Input::Clock {
        clock_time: handle.clock_time(),
    };
    assert_err!(handle.submit(input));
}

fn wait_for_stats(subscription: &updraft_runtime::Subscription) -> Option<TraceStats> {
    let deadline = Instant::now() + TIMEOUT;
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        let changes = subscription.changes.recv_timeout(remaining).ok()?;
        for change in changes {
            if let Change::Flight(FlightChange::TraceStats(Some(stats))) = change {
                return Some(stats);
            }
        }
    }
}
