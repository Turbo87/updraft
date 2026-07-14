use std::time::Duration;

use crate::flight::{self, Flight};
use crate::protocol::{ComputeKind, ComputeResult, Input, Snapshot, Update};
use crate::time::Timers;

/// A typed read-only request against current state.
pub trait Query {
    type Output;

    fn execute(self, app: &App) -> Self::Output;
}

/// Scheduling configuration for the application core.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AppConfig {
    pub flight: flight::Config,
}

/// The deterministic application core.
///
/// One `App` owns shared state such as the current flight state and settings
/// that affect flight behavior. State changes only through
/// [`handle()`](Self::handle), so the same ordered inputs always produce the
/// same results.
#[derive(Debug)]
pub struct App {
    /// Latest clock time accepted by the core.
    clock_time: Duration,
    timers: Timers,
    pub(crate) flight: Flight,
}

impl App {
    /// An app with the default (production) configuration.
    pub fn new() -> Self {
        Self::with_config(AppConfig::default())
    }

    pub fn with_config(config: AppConfig) -> Self {
        Self {
            clock_time: Duration::ZERO,
            timers: Timers::default(),
            flight: Flight::new(config.flight),
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
            Input::Clock { clock_time } => self.advance(clock_time),
            Input::Flight(input) => {
                if let Some(observed_at) = input.observed_at() {
                    self.advance(observed_at);
                }
                self.flight
                    .handle(input, self.clock_time, &mut self.timers, &mut update);
            }
            Input::ComputeResult(ComputeResult::Flight(result)) => {
                self.flight
                    .compute_result(result, self.clock_time, &mut self.timers, &mut update);
            }
            Input::ComputeFailed(failure) => {
                let ComputeKind::Flight(kind) = failure.kind;
                self.flight.compute_failed(
                    kind,
                    failure.revision,
                    self.clock_time,
                    &mut self.timers,
                );
            }
            Input::ComputeCancelled(cancellation) => {
                let ComputeKind::Flight(kind) = cancellation.kind;
                self.flight.compute_cancelled(
                    kind,
                    cancellation.revision,
                    self.clock_time,
                    &mut self.timers,
                );
            }
        }
        for timer in self.timers.take_due(self.clock_time) {
            self.flight.timer(timer, self.clock_time, &mut update);
        }
        update.next_deadline = self.timers.next_deadline();
        update
    }

    /// Answers a typed read-only query against the current state.
    pub fn query<Q: Query>(&self, query: Q) -> Q::Output {
        query.execute(self)
    }

    /// Captures the shared current state for a newly subscribing client.
    pub fn snapshot(&self) -> Snapshot {
        Snapshot {
            flight: self.flight.snapshot(),
        }
    }

    /// Advances the core's idea of current time. Time never goes backward.
    fn advance(&mut self, clock_time: Duration) {
        self.clock_time = self.clock_time.max(clock_time);
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clock_never_goes_backward() {
        let mut app = App::new();

        app.advance(Duration::from_secs(10));
        app.advance(Duration::from_secs(1));

        assert_eq!(app.clock_time, Duration::from_secs(10));
    }
}
