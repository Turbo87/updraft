use std::time::Duration;

use crate::flight::Flight;
use crate::protocol::{Command, Input, Observation, Query, QueryResult, Snapshot, Update};
use crate::time::{MonotonicTime, Timers};

/// Tuning knobs for the core's scheduling behavior.
///
/// The defaults are the production values. Tests may shorten the
/// intervals to keep wall-clock time down when they run against the
/// real runtime instead of simulated time.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AppConfig {
    /// Minimum spacing between two trace-statistics compute jobs.
    pub trace_stats_interval: Duration,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            trace_stats_interval: Duration::from_secs(5),
        }
    }
}

/// The deterministic application core.
///
/// One `App` owns all authoritative flight state. It is mutated only
/// through [`handle()`](Self::handle), so there is no shared mutable state
/// across threads, and the same ordered inputs always produce the same
/// state changes.
#[derive(Debug)]
pub struct App {
    now: MonotonicTime,
    timers: Timers,
    flight: Flight,
}

impl App {
    /// An app with the default (production) configuration.
    pub fn new() -> Self {
        Self::with_config(AppConfig::default())
    }

    pub fn with_config(config: AppConfig) -> Self {
        Self {
            now: MonotonicTime::ORIGIN,
            timers: Timers::default(),
            flight: Flight::new(config.trace_stats_interval),
        }
    }

    /// Applies one [`Input`].
    ///
    /// After the input is handled, timers that became due fire in
    /// deterministic order, and [`Update::next_deadline`] reports
    /// when the runtime must deliver the next [`Input::Clock`].
    pub fn handle(&mut self, input: Input) -> Update {
        let mut update = Update::default();
        match input {
            Input::Clock { now } => self.advance(now),
            Input::Command(Command::ClearTrace) => {
                self.flight.clear_trace(&mut self.timers, &mut update);
            }
            Input::Observation(Observation::Position(fix)) => {
                self.advance(fix.observed_at);
                self.flight
                    .observe_position(fix, self.now, &mut self.timers, &mut update);
            }
            Input::ComputeResult(result) => {
                self.flight
                    .compute_result(result, self.now, &mut self.timers, &mut update);
            }
            Input::ComputeFailed(failure) => {
                self.flight
                    .compute_failed(&failure, self.now, &mut self.timers);
            }
        }
        for timer in self.timers.take_due(self.now) {
            self.flight.timer(timer, self.now, &mut update);
        }
        update.next_deadline = self.timers.next_deadline();
        update
    }

    /// Answers a read-only query against the current state.
    pub fn query(&self, query: Query) -> QueryResult {
        match query {
            Query::Position => QueryResult::Position(self.flight.position()),
            Query::TraceStats => QueryResult::TraceStats(self.flight.trace_stats()),
        }
    }

    /// Captures the shared current state for a newly subscribing client.
    pub fn snapshot(&self) -> Snapshot {
        Snapshot {
            position: self.flight.position(),
            trace_stats: self.flight.trace_stats(),
        }
    }

    /// Advances the core's idea of current time. Time never goes backward.
    fn advance(&mut self, now: MonotonicTime) {
        self.now = self.now.max(now);
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}
