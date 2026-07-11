//! The deterministic Updraft core.
//!
//! The core owns authoritative state and the decisions based on it. It
//! performs no I/O, spawns no threads, and reads no clocks. All state
//! changes flow through a single entry point, [`App::handle`], which takes
//! one [`Input`] and returns an [`Update`] of client-visible [`Change`]
//! values, [`Effect`] requests for outside work, and the next timer
//! deadline. A shared runtime owns the clock, queues, workers, and
//! subscriptions and drives this loop; see the `updraft_runtime` crate.
//!
//! This crate currently implements a small illustrative domain (see
//! [`demo`]) that demonstrates the async-computation path end to end: cheap
//! per-input work stays synchronous, while an expensive reduction is handed
//! to a worker through [`Effect::Compute`] and returns as an input. Real
//! domains (flight, navigation, traffic) will follow the same shape.

pub mod compute;
pub mod demo;
mod protocol;
pub mod time;

pub use compute::{ComputeFailure, ComputeJob, ComputeKind, ComputeResult, Reduction};
pub use demo::Demo;
pub use protocol::{Change, Effect, Input, Query, QueryResult, Sample, Snapshot, Update};
pub use time::{MonotonicTime, TimerId, TimerQueue};

/// The default demo re-evaluation cadence, in milliseconds.
const DEFAULT_INTERVAL_MILLIS: u64 = 1000;

/// The application: authoritative state plus the single mutation entry
/// point.
///
/// Construct one with [`App::new`], feed it [`Input`] values through
/// [`handle`](App::handle), and read it with [`query`](App::query) and
/// [`snapshot`](App::snapshot). The core never mutates except through
/// `handle`, and holds no shared mutable state across threads.
#[derive(Clone, Debug)]
pub struct App {
    now: MonotonicTime,
    timers: TimerQueue,
    demo: Demo,
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl App {
    /// Creates an app with the default cadence.
    pub fn new() -> Self {
        Self::with_interval_millis(DEFAULT_INTERVAL_MILLIS)
    }

    /// Creates an app whose demo domain re-evaluates at most once per
    /// `interval_millis`. Tests use a short interval to keep runs fast.
    pub fn with_interval_millis(interval_millis: u64) -> Self {
        Self {
            now: MonotonicTime::ZERO,
            timers: TimerQueue::new(),
            demo: Demo::new(interval_millis),
        }
    }

    /// Applies one input; the only mutation entry point.
    pub fn handle(&mut self, input: Input) -> Update {
        let mut changes = Vec::new();
        let mut effects = Vec::new();

        match input {
            Input::Observe(sample) => self.on_observe(sample, &mut changes),
            Input::Tick(now) => self.on_tick(now, &mut effects),
            Input::ComputeCompleted(result) => {
                self.on_completed(result, &mut changes, &mut effects);
            }
            Input::ComputeFailed(failure) => self.on_failed(failure, &mut effects),
        }

        Update {
            changes,
            effects,
            next_deadline: self.timers.next_deadline(),
        }
    }

    /// Answers a read-only query against current state.
    pub fn query(&self, query: Query) -> QueryResult {
        match query {
            Query::SampleCount => QueryResult::SampleCount(self.demo.sample_count()),
            Query::LatestReduction => QueryResult::LatestReduction(self.demo.latest()),
        }
    }

    /// The shared current state for a newly subscribing client.
    pub fn snapshot(&self) -> Snapshot {
        Snapshot {
            sample_count: self.demo.sample_count(),
            latest: self.demo.latest(),
        }
    }

    fn on_observe(&mut self, sample: Sample, changes: &mut Vec<Change>) {
        self.now = self.now.max(sample.observed_at);
        let count = self.demo.record(sample.value);
        changes.push(Change::Samples(count));

        // Arm the cadence timer on the first pending work. If one is
        // already scheduled we leave it, so a burst of samples does not
        // keep pushing the deadline out.
        if !self.timers.is_scheduled(TimerId::Demo) {
            self.timers
                .schedule(TimerId::Demo, self.next_demo_deadline());
        }
    }

    fn on_tick(&mut self, now: MonotonicTime, effects: &mut Vec<Effect>) {
        self.now = self.now.max(now);
        if !self.timers.take_due(self.now).contains(&TimerId::Demo) {
            return;
        }

        match self.demo.start_job() {
            Some(job) => effects.push(Effect::Compute(job)),
            None => {
                // The slot was busy. If work is still pending, try again
                // after another interval; otherwise leave the timer idle.
                if self.demo.has_pending_work() {
                    self.timers
                        .schedule(TimerId::Demo, self.next_demo_deadline());
                }
            }
        }
    }

    fn on_completed(
        &mut self,
        result: ComputeResult,
        changes: &mut Vec<Change>,
        effects: &mut Vec<Effect>,
    ) {
        match result {
            ComputeResult::Demo { reduction } => {
                self.demo.accept(reduction);
                changes.push(Change::Computed(reduction));
            }
        }
        self.maybe_rerun(effects);
    }

    fn on_failed(&mut self, failure: ComputeFailure, effects: &mut Vec<Effect>) {
        match failure.kind {
            ComputeKind::Demo => self.demo.record_failure(),
        }
        self.maybe_rerun(effects);
    }

    /// After a job frees the slot, launch another immediately if samples
    /// arrived while it ran.
    fn maybe_rerun(&mut self, effects: &mut Vec<Effect>) {
        if let Some(job) = self.demo.start_job() {
            effects.push(Effect::Compute(job));
        }
    }

    fn next_demo_deadline(&self) -> MonotonicTime {
        self.now.saturating_add_millis(self.demo.interval_millis())
    }
}
