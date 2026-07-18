use std::collections::BTreeMap;
use std::time::Duration;

/// Work that the core schedules for later.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Timer {
    /// Starts the next trace-statistics job.
    TraceStats,
}

/// Stores scheduled timers and their deadlines.
#[derive(Debug, Default)]
pub(crate) struct Timers {
    due_times: BTreeMap<Timer, Duration>,
}

impl Timers {
    /// Schedules `timer` to fire at `at`, replacing its existing due time.
    pub(crate) fn schedule(&mut self, timer: Timer, at: Duration) {
        self.due_times.insert(timer, at);
    }

    pub(crate) fn cancel(&mut self, timer: Timer) {
        self.due_times.remove(&timer);
    }

    pub(crate) fn is_scheduled(&self, timer: Timer) -> bool {
        self.due_times.contains_key(&timer)
    }

    /// The earliest scheduled deadline, if any.
    pub(crate) fn next_deadline(&self) -> Option<Duration> {
        self.due_times.values().copied().min()
    }

    /// Removes due timers, ordered by deadline and then timer identity.
    pub(crate) fn take_due(&mut self, clock_time: Duration) -> Vec<Timer> {
        let mut due: Vec<(Duration, Timer)> = self
            .due_times
            .iter()
            .filter(|(_, at)| **at <= clock_time)
            .map(|(timer, at)| (*at, *timer))
            .collect();
        due.sort();
        let due: Vec<Timer> = due.into_iter().map(|(_, timer)| timer).collect();
        for timer in &due {
            self.due_times.remove(timer);
        }
        due
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use claims::{assert_none, assert_some_eq};

    #[test]
    fn timer_queue() {
        let mut timers = Timers::default();
        assert_none!(timers.next_deadline());

        timers.schedule(Timer::TraceStats, Duration::from_micros(100));
        assert!(timers.is_scheduled(Timer::TraceStats));
        assert_some_eq!(timers.next_deadline(), Duration::from_micros(100));

        assert_eq!(timers.take_due(Duration::from_micros(99)), vec![]);
        assert_eq!(
            timers.take_due(Duration::from_micros(100)),
            vec![Timer::TraceStats]
        );
        assert!(!timers.is_scheduled(Timer::TraceStats));
        assert_none!(timers.next_deadline());

        timers.schedule(Timer::TraceStats, Duration::from_micros(200));
        timers.cancel(Timer::TraceStats);
        assert_none!(timers.next_deadline());
    }
}
