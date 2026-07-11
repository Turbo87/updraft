//! Monotonic time and the authoritative timer queue.
//!
//! The core never reads a clock. Adapters stamp observations with
//! [`MonotonicTime`] values taken from one process-wide origin, and the
//! runtime delivers clock advancement as a
//! [`Input::Tick`](crate::Input::Tick) input. Timers live in core state so
//! that replay and tests schedule with the same logic as production.

use serde::{Deserialize, Serialize};

/// A point on the process-wide monotonic timeline, counted in nanoseconds
/// from a single origin chosen by the runtime.
///
/// Monotonic time is used only for scheduling, delay rules, and freshness.
/// Wall/GPS time is carried separately inside flight data.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct MonotonicTime {
    nanos: u64,
}

impl MonotonicTime {
    /// The origin of the timeline.
    pub const ZERO: Self = Self { nanos: 0 };

    /// A timestamp `nanos` nanoseconds after the origin.
    pub const fn from_nanos(nanos: u64) -> Self {
        Self { nanos }
    }

    /// A timestamp `millis` milliseconds after the origin.
    pub const fn from_millis(millis: u64) -> Self {
        Self {
            nanos: millis.saturating_mul(1_000_000),
        }
    }

    /// A timestamp `secs` seconds after the origin.
    pub const fn from_secs(secs: u64) -> Self {
        Self {
            nanos: secs.saturating_mul(1_000_000_000),
        }
    }

    /// The number of nanoseconds since the origin.
    pub const fn as_nanos(self) -> u64 {
        self.nanos
    }

    /// The number of seconds since the origin, as a float.
    pub fn as_secs_f64(self) -> f64 {
        self.nanos as f64 / 1_000_000_000.0
    }

    /// The timestamp `millis` milliseconds later, saturating at the
    /// representable maximum.
    pub const fn saturating_add_millis(self, millis: u64) -> Self {
        Self {
            nanos: self.nanos.saturating_add(millis.saturating_mul(1_000_000)),
        }
    }

    /// The later of two timestamps. Advancing `now` through this keeps the
    /// core's clock monotonic even if an input carries an earlier stamp.
    pub const fn max(self, other: Self) -> Self {
        if self.nanos >= other.nanos {
            self
        } else {
            other
        }
    }
}

/// Identifies a timer owned by a domain. One variant per recurring or
/// one-shot deadline the core schedules; each expensive computation type
/// that runs on a cadence gets its own id.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum TimerId {
    /// Drives the demo domain's re-evaluation cadence.
    Demo,
}

/// The authoritative set of pending timers.
///
/// The queue holds at most one deadline per [`TimerId`]; scheduling an id
/// that is already present replaces its deadline. Both `next_deadline` and
/// the ordering of `take_due` are core state, so replay advances time
/// through the same logic as production.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimerQueue {
    entries: Vec<(TimerId, MonotonicTime)>,
}

impl TimerQueue {
    /// An empty queue.
    pub fn new() -> Self {
        Self::default()
    }

    /// Schedules `id` to fire at `deadline`, replacing any existing
    /// deadline for the same id.
    pub fn schedule(&mut self, id: TimerId, deadline: MonotonicTime) {
        if let Some(entry) = self
            .entries
            .iter_mut()
            .find(|(existing, _)| *existing == id)
        {
            entry.1 = deadline;
        } else {
            self.entries.push((id, deadline));
        }
    }

    /// Cancels the timer for `id`, if any is scheduled.
    pub fn cancel(&mut self, id: TimerId) {
        self.entries.retain(|(existing, _)| *existing != id);
    }

    /// Whether a timer is currently scheduled for `id`.
    pub fn is_scheduled(&self, id: TimerId) -> bool {
        self.entries.iter().any(|(existing, _)| *existing == id)
    }

    /// The earliest scheduled deadline, or `None` when no timer is pending.
    pub fn next_deadline(&self) -> Option<MonotonicTime> {
        self.entries.iter().map(|(_, at)| *at).min()
    }

    /// Removes and returns every timer whose deadline is at or before
    /// `now`, ordered by deadline then id for determinism.
    pub fn take_due(&mut self, now: MonotonicTime) -> Vec<TimerId> {
        let mut due: Vec<(TimerId, MonotonicTime)> = self
            .entries
            .iter()
            .copied()
            .filter(|(_, at)| *at <= now)
            .collect();
        due.sort_by(|a, b| a.1.cmp(&b.1).then(a.0.cmp(&b.0)));
        self.entries.retain(|(_, at)| *at > now);
        due.into_iter().map(|(id, _)| id).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arithmetic_and_conversions() {
        let t = MonotonicTime::from_secs(2);
        assert_eq!(t.as_nanos(), 2_000_000_000);
        assert_eq!(t.as_secs_f64(), 2.0);
        assert_eq!(
            t.saturating_add_millis(500),
            MonotonicTime::from_millis(2500)
        );
        assert!(MonotonicTime::from_millis(1) > MonotonicTime::ZERO);
    }

    #[test]
    fn max_keeps_the_clock_monotonic() {
        let later = MonotonicTime::from_secs(5);
        let earlier = MonotonicTime::from_secs(3);
        assert_eq!(later.max(earlier), later);
        assert_eq!(earlier.max(later), later);
    }

    #[test]
    fn schedule_replaces_existing_deadline() {
        let mut queue = TimerQueue::new();
        queue.schedule(TimerId::Demo, MonotonicTime::from_secs(5));
        queue.schedule(TimerId::Demo, MonotonicTime::from_secs(1));
        assert_eq!(queue.next_deadline(), Some(MonotonicTime::from_secs(1)));
        assert!(queue.is_scheduled(TimerId::Demo));
    }

    #[test]
    fn take_due_returns_only_elapsed_timers() {
        let mut queue = TimerQueue::new();
        queue.schedule(TimerId::Demo, MonotonicTime::from_secs(3));

        assert!(queue.take_due(MonotonicTime::from_secs(2)).is_empty());
        assert_eq!(queue.next_deadline(), Some(MonotonicTime::from_secs(3)));

        assert_eq!(
            queue.take_due(MonotonicTime::from_secs(3)),
            vec![TimerId::Demo]
        );
        assert_eq!(queue.next_deadline(), None);
        assert!(!queue.is_scheduled(TimerId::Demo));
    }
}
