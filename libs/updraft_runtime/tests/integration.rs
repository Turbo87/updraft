//! End-to-end runtime tests exercising the async worker path over real
//! threads. Intervals and delays are tiny; the tests wait on conditions
//! with generous timeouts rather than fixed sleeps.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::Receiver;
use std::time::{Duration, Instant};

use updraft_runtime::{ComputeFn, Reduction, Runtime, RuntimeConfig};

use updraft_core::Change;

const TIMEOUT: Duration = Duration::from_secs(5);

fn fast_config() -> RuntimeConfig {
    RuntimeConfig {
        interval_millis: 20,
        worker_delay: Duration::from_millis(2),
        ..Default::default()
    }
}

/// Waits for a `Computed` change whose reduction has `count` samples.
fn wait_for_reduction(changes: &Receiver<Vec<Change>>, count: usize) -> Reduction {
    let deadline = Instant::now() + TIMEOUT;
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        assert!(!remaining.is_zero(), "timed out waiting for reduction");
        match changes.recv_timeout(remaining) {
            Ok(batch) => {
                for change in batch {
                    if let Change::Computed(reduction) = change
                        && reduction.count == count
                    {
                        return reduction;
                    }
                }
            }
            Err(_) => panic!("change stream closed unexpectedly"),
        }
    }
}

/// Drains a receiver until it disconnects, returning whether that happened
/// before the timeout.
fn drained_to_disconnect(changes: &Receiver<Vec<Change>>) -> bool {
    let deadline = Instant::now() + TIMEOUT;
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return false;
        }
        match changes.recv_timeout(remaining) {
            Ok(_) => {}
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => return true,
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => return false,
        }
    }
}

/// Polls `condition` until it holds or the timeout elapses.
fn wait_until(mut condition: impl FnMut() -> bool) -> bool {
    let deadline = Instant::now() + TIMEOUT;
    while Instant::now() < deadline {
        if condition() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    condition()
}

#[test]
fn samples_flow_through_the_worker_and_back_as_a_change() {
    let runtime = Runtime::with_config(fast_config());
    let subscription = runtime.subscribe();
    assert_eq!(subscription.snapshot.sample_count, 0);
    assert_eq!(subscription.snapshot.latest, None);

    runtime.submit_observe(1);
    runtime.submit_observe(2);
    runtime.submit_observe(3);

    // A batch covering all three samples is eventually reduced off the loop.
    let reduction = wait_for_reduction(&subscription.changes, 3);
    assert_eq!(reduction, Reduction { count: 3, sum: 6 });

    runtime.shutdown();
}

#[test]
fn a_slow_subscriber_is_dropped_without_stalling_the_loop() {
    let config = RuntimeConfig {
        subscriber_capacity: 4,
        ..fast_config()
    };
    let runtime = Runtime::with_config(config);

    // This subscriber is never drained, so its bounded buffer overflows.
    let slow = runtime.subscribe();

    for value in 0..100 {
        runtime.submit_observe(value);
    }

    // A subscription registered after the flood is FIFO-ordered behind every
    // observe, so once it returns the loop has handled all 100 (proving it
    // never stalled) and dropped the slow subscriber much earlier. We sync
    // on this *before* draining `slow`, so draining cannot keep it alive.
    let healthy = runtime.subscribe();
    assert_eq!(healthy.snapshot.sample_count, 100);

    // The slow subscriber's sender was dropped, so draining its buffer ends
    // in a disconnect.
    assert!(
        drained_to_disconnect(&slow.changes),
        "slow subscriber should be dropped"
    );

    runtime.shutdown();
}

#[test]
fn a_worker_panic_becomes_a_failure_and_the_runtime_survives() {
    let calls = Arc::new(AtomicUsize::new(0));
    let observed = Arc::clone(&calls);
    let compute: ComputeFn = Arc::new(move |_job| {
        observed.fetch_add(1, Ordering::SeqCst);
        panic!("simulated worker crash");
    });

    let config = RuntimeConfig {
        compute,
        ..fast_config()
    };
    let runtime = Runtime::with_config(config);

    runtime.submit_observe(1);
    runtime.submit_observe(2);

    // The worker is invoked and panics; the runtime catches it rather than
    // dying.
    assert!(
        wait_until(|| calls.load(Ordering::SeqCst) >= 1),
        "worker should have been invoked"
    );

    // The loop is still alive and handled both observations, but no result
    // was ever produced because every job panicked.
    let healthy = runtime.subscribe();
    assert_eq!(healthy.snapshot.sample_count, 2);
    assert_eq!(healthy.snapshot.latest, None);

    runtime.shutdown();
}
