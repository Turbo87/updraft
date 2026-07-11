use std::collections::BTreeMap;
use std::ops::Add;
use std::time::Duration;

use serde::{Deserialize, Serialize};

/// A point on the process-wide monotonic timeline, in microseconds since
/// the runtime's time origin.
///
/// The core never reads a clock. Adapters stamp observations with
/// monotonic timestamps from one process-wide origin, and clock
/// advancement enters the core as an [`Input`](crate::Input). Monotonic
/// time is used only for scheduling, delay rules, freshness, and
/// lookahead. GPS time stays in flight data and IGC records.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct MonotonicTime {
    micros: u64,
}

impl MonotonicTime {
    /// The process-wide time origin.
    pub const ORIGIN: Self = Self { micros: 0 };

    pub const fn from_micros(micros: u64) -> Self {
        Self { micros }
    }

    pub const fn as_micros(self) -> u64 {
        self.micros
    }

    /// The timestamp `since_origin` after the time origin, truncated to
    /// microsecond resolution.
    pub const fn from_duration(since_origin: Duration) -> Self {
        // u64 microseconds cover ~584,000 years of process uptime.
        let micros = since_origin.as_micros();
        let micros = if micros > u64::MAX as u128 {
            u64::MAX
        } else {
            micros as u64
        };
        Self { micros }
    }

    /// The duration from `earlier` to `self`, or zero if `earlier` is later.
    pub const fn saturating_duration_since(self, earlier: Self) -> Duration {
        Duration::from_micros(self.micros.saturating_sub(earlier.micros))
    }
}

impl Add<Duration> for MonotonicTime {
    type Output = Self;

    fn add(self, rhs: Duration) -> Self {
        Self::from_micros(self.micros.saturating_add(Self::from_duration(rhs).micros))
    }
}

/// Identity of a scheduled core timer.
///
/// Each timer fires at most once per scheduling. A domain reschedules it
/// when it needs another wake-up.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Timer {
    /// Throttles trace-statistics compute jobs.
    TraceStats,
}

/// The deterministic timer queue.
///
/// Timers are authoritative state: identity, deadlines, and firing order
/// live here, and [`Update::next_deadline`](crate::Update::next_deadline)
/// tells the runtime when to deliver the next clock input. The runtime
/// only sleeps and reports that time advanced, so tests and replay use
/// the same scheduling logic as production.
#[derive(Debug, Default)]
pub(crate) struct Timers {
    deadlines: BTreeMap<Timer, MonotonicTime>,
}

impl Timers {
    /// Schedules `timer` to fire at `at`, replacing an earlier schedule.
    pub(crate) fn schedule(&mut self, timer: Timer, at: MonotonicTime) {
        self.deadlines.insert(timer, at);
    }

    pub(crate) fn cancel(&mut self, timer: Timer) {
        self.deadlines.remove(&timer);
    }

    pub(crate) fn is_scheduled(&self, timer: Timer) -> bool {
        self.deadlines.contains_key(&timer)
    }

    /// The earliest scheduled deadline, if any.
    pub(crate) fn next_deadline(&self) -> Option<MonotonicTime> {
        self.deadlines.values().copied().min()
    }

    /// Removes and returns all timers due at `now`, ordered by deadline
    /// first and timer identity second so firing order is deterministic.
    pub(crate) fn take_due(&mut self, now: MonotonicTime) -> Vec<Timer> {
        let mut due: Vec<(MonotonicTime, Timer)> = self
            .deadlines
            .iter()
            .filter(|(_, at)| **at <= now)
            .map(|(timer, at)| (*at, *timer))
            .collect();
        due.sort();
        let due: Vec<Timer> = due.into_iter().map(|(_, timer)| timer).collect();
        for timer in &due {
            self.deadlines.remove(timer);
        }
        due
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn monotonic_time_arithmetic() {
        let t = MonotonicTime::from_micros(1_500_000);
        assert_eq!(
            t + Duration::from_millis(500),
            MonotonicTime::from_micros(2_000_000)
        );
        assert_eq!(
            t.saturating_duration_since(MonotonicTime::from_micros(500_000)),
            Duration::from_secs(1)
        );
        assert_eq!(
            MonotonicTime::ORIGIN.saturating_duration_since(t),
            Duration::ZERO
        );
        assert_eq!(
            MonotonicTime::from_duration(Duration::from_nanos(1_500)),
            MonotonicTime::from_micros(1)
        );
    }

    #[test]
    fn timer_queue() {
        let mut timers = Timers::default();
        assert_eq!(timers.next_deadline(), None);

        timers.schedule(Timer::TraceStats, MonotonicTime::from_micros(100));
        assert!(timers.is_scheduled(Timer::TraceStats));
        assert_eq!(
            timers.next_deadline(),
            Some(MonotonicTime::from_micros(100))
        );

        assert_eq!(timers.take_due(MonotonicTime::from_micros(99)), vec![]);
        assert_eq!(
            timers.take_due(MonotonicTime::from_micros(100)),
            vec![Timer::TraceStats]
        );
        assert!(!timers.is_scheduled(Timer::TraceStats));
        assert_eq!(timers.next_deadline(), None);

        timers.schedule(Timer::TraceStats, MonotonicTime::from_micros(200));
        timers.cancel(Timer::TraceStats);
        assert_eq!(timers.next_deadline(), None);
    }
}
