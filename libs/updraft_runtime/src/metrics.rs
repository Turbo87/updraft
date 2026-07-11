use std::sync::atomic::{AtomicU64, Ordering};

/// Counters the runtime maintains while it runs.
///
/// These counters are deliberately coarse. Queue-depth and handler-time
/// measurements are intentionally out of scope here rather than relying on
/// unit-test timing.
#[derive(Debug, Default)]
pub struct Metrics {
    inputs_handled: AtomicU64,
    slow_subscriber_drops: AtomicU64,
    worker_failures: AtomicU64,
}

impl Metrics {
    /// How many inputs the core has handled.
    pub fn inputs_handled(&self) -> u64 {
        self.inputs_handled.load(Ordering::Relaxed)
    }

    /// How many subscriptions were dropped because their buffer was full.
    pub fn slow_subscriber_drops(&self) -> u64 {
        self.slow_subscriber_drops.load(Ordering::Relaxed)
    }

    /// How many compute jobs failed (worker errors and panics).
    pub fn worker_failures(&self) -> u64 {
        self.worker_failures.load(Ordering::Relaxed)
    }

    pub(crate) fn record_input(&self) {
        self.inputs_handled.fetch_add(1, Ordering::Relaxed);
    }

    pub(crate) fn record_slow_subscriber_drop(&self) {
        self.slow_subscriber_drops.fetch_add(1, Ordering::Relaxed);
    }

    pub(crate) fn record_worker_failure(&self) {
        self.worker_failures.fetch_add(1, Ordering::Relaxed);
    }
}
