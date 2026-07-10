use std::time::Duration;

use serde::{Deserialize, Serialize};

/// A point on the monotonic timeline that drives scheduling.
///
/// The core never reads a clock: adapters stamp every input with the time
/// elapsed since a single process-wide epoch chosen by the host, and tests
/// and replay provide timestamps directly. Never conflated with GPS time,
/// which travels inside the observations themselves.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct MonotonicTime(Duration);

impl MonotonicTime {
    pub const fn from_duration(duration: Duration) -> Self {
        Self(duration)
    }

    pub const fn as_duration(self) -> Duration {
        self.0
    }
}

impl std::ops::Add<Duration> for MonotonicTime {
    type Output = Self;

    fn add(self, duration: Duration) -> Self {
        Self(self.0 + duration)
    }
}
