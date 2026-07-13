use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

/// Measurements collected by the runtime.
#[derive(Debug, Default)]
pub struct Metrics {
    inputs_handled: AtomicU64,
    pending_messages: AtomicU64,
    max_pending_messages: AtomicU64,
    queue_wait_samples: AtomicU64,
    total_queue_wait_nanos: AtomicU64,
    total_handler_nanos: AtomicU64,
    max_handler_nanos: AtomicU64,
    slow_subscriber_drops: AtomicU64,
    worker_failures: AtomicU64,
}

impl Metrics {
    /// How many inputs the core has handled.
    pub fn inputs_handled(&self) -> u64 {
        self.inputs_handled.load(Ordering::Relaxed)
    }

    /// Largest number of messages waiting for the core loop.
    pub fn max_pending_messages(&self) -> u64 {
        self.max_pending_messages.load(Ordering::Relaxed)
    }

    /// Number of queue-wait durations recorded.
    pub fn queue_wait_samples(&self) -> u64 {
        self.queue_wait_samples.load(Ordering::Relaxed)
    }

    /// Total time messages spent waiting to be handled by the core loop.
    pub fn total_queue_wait(&self) -> Duration {
        Duration::from_nanos(self.total_queue_wait_nanos.load(Ordering::Relaxed))
    }

    /// Total time spent in `App::handle()`.
    pub fn total_handler_time(&self) -> Duration {
        Duration::from_nanos(self.total_handler_nanos.load(Ordering::Relaxed))
    }

    /// Longest observed `App::handle()` call.
    pub fn max_handler_time(&self) -> Duration {
        Duration::from_nanos(self.max_handler_nanos.load(Ordering::Relaxed))
    }

    /// How many subscriptions were dropped because their buffer was full.
    pub fn slow_subscriber_drops(&self) -> u64 {
        self.slow_subscriber_drops.load(Ordering::Relaxed)
    }

    /// How many compute jobs ended in failure rather than completion.
    pub fn worker_failures(&self) -> u64 {
        self.worker_failures.load(Ordering::Relaxed)
    }

    pub(crate) fn record_input(&self) {
        self.inputs_handled.fetch_add(1, Ordering::Relaxed);
    }

    pub(crate) fn record_enqueued(&self) {
        let pending = self.pending_messages.fetch_add(1, Ordering::Relaxed) + 1;
        self.max_pending_messages
            .fetch_max(pending, Ordering::Relaxed);
    }

    pub(crate) fn record_dequeued(&self, waited: Duration) {
        self.pending_messages.fetch_sub(1, Ordering::Relaxed);
        self.queue_wait_samples.fetch_add(1, Ordering::Relaxed);
        self.total_queue_wait_nanos
            .fetch_add(duration_nanos(waited), Ordering::Relaxed);
    }

    pub(crate) fn record_handler_time(&self, elapsed: Duration) {
        let nanos = duration_nanos(elapsed);
        self.total_handler_nanos.fetch_add(nanos, Ordering::Relaxed);
        self.max_handler_nanos.fetch_max(nanos, Ordering::Relaxed);
    }

    pub(crate) fn record_send_failure(&self) {
        self.pending_messages.fetch_sub(1, Ordering::Relaxed);
    }

    pub(crate) fn record_slow_subscriber_drop(&self) {
        self.slow_subscriber_drops.fetch_add(1, Ordering::Relaxed);
    }

    pub(crate) fn record_worker_failure(&self) {
        self.worker_failures.fetch_add(1, Ordering::Relaxed);
    }
}

fn duration_nanos(duration: Duration) -> u64 {
    u64::try_from(duration.as_nanos()).unwrap_or(u64::MAX)
}
