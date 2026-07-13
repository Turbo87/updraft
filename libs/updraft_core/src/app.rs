use std::time::Duration;

use crate::protocol::{Input, Update};

/// The deterministic application core.
#[derive(Debug, Default)]
pub struct App {
    /// Latest clock time accepted by the core.
    clock_time: Duration,
}

impl App {
    pub fn new() -> Self {
        Self::default()
    }

    /// Applies one [`Input`].
    pub fn handle(&mut self, input: Input) -> Update {
        match input {
            Input::Clock { clock_time } => self.advance(clock_time),
        }
        Update::default()
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
