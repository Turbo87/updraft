use std::time::Duration;

use crate::flight::Flight;
use crate::protocol::{Input, Snapshot, Update};

/// A typed read-only request against current state.
pub trait Query {
    type Output;

    fn execute(self, app: &App) -> Self::Output;
}

/// The deterministic application core.
///
/// One `App` owns shared state such as the current flight state and settings
/// that affect flight behavior. State changes only through
/// [`handle()`](Self::handle), so the same ordered inputs always produce the
/// same results.
#[derive(Debug, Default)]
pub struct App {
    /// Latest clock time accepted by the core.
    clock_time: Duration,
    pub(crate) flight: Flight,
}

impl App {
    pub fn new() -> Self {
        Self::default()
    }

    /// Applies one [`Input`].
    pub fn handle(&mut self, input: Input) -> Update {
        let mut update = Update::default();
        match input {
            Input::Clock { clock_time } => self.advance(clock_time),
            Input::Flight(input) => {
                if let Some(observed_at) = input.observed_at() {
                    self.advance(observed_at);
                }
                self.flight.handle(input, &mut update);
            }
        }
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
