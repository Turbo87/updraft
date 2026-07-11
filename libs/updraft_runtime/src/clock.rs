use std::time::Instant;

use updraft_core::MonotonicTime;

/// The process-wide monotonic time origin.
///
/// The runtime creates one clock when it starts. Every adapter stamps its
/// observations with timestamps from this same timeline, and the core
/// itself never reads a clock: time advances only through inputs.
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

    /// The current time on the process-wide monotonic timeline.
    pub fn now(&self) -> MonotonicTime {
        MonotonicTime::from_duration(self.origin.elapsed())
    }
}
