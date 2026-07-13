use std::time::{Duration, Instant};

/// A monotonic clock for one runtime.
#[derive(Clone, Debug)]
pub struct Clock {
    origin: Instant,
}

impl Clock {
    pub(crate) fn new() -> Self {
        Self {
            origin: Instant::now(),
        }
    }

    /// Time since the runtime started.
    pub fn clock_time(&self) -> Duration {
        self.origin.elapsed()
    }
}
