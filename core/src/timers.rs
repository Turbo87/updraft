use crate::time::MonotonicTime;

/// The fixed set of timers the core can arm. The enum order is the
/// tie-break when several deadlines fire on the same clock advance.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Timer {
    PositionStaleness,
}

const ALL_TIMERS: [Timer; 1] = [Timer::PositionStaleness];

/// Deterministic timer state: one deadline slot per [`Timer`], armed by
/// the update loop and drained as injected clock inputs advance time.
/// Re-arming a timer replaces its deadline, which is exactly debounce.
#[derive(Default)]
pub(crate) struct Timers {
    position_staleness: Option<MonotonicTime>,
}

impl Timers {
    pub(crate) fn arm(&mut self, timer: Timer, deadline: MonotonicTime) {
        *self.slot(timer) = Some(deadline);
    }

    /// Fires every timer with a deadline at or before `now`, in the fixed
    /// [`ALL_TIMERS`] order.
    pub(crate) fn advance(&mut self, now: MonotonicTime) -> Vec<Timer> {
        ALL_TIMERS
            .into_iter()
            .filter(|&timer| {
                let slot = self.slot(timer);
                let due = slot.is_some_and(|deadline| deadline <= now);
                if due {
                    *slot = None;
                }
                due
            })
            .collect()
    }

    /// The earliest pending deadline, for the host to arm one sleep.
    pub(crate) fn next_deadline(&self) -> Option<MonotonicTime> {
        ALL_TIMERS
            .iter()
            .filter_map(|&timer| self.peek(timer))
            .min()
    }

    fn slot(&mut self, timer: Timer) -> &mut Option<MonotonicTime> {
        match timer {
            Timer::PositionStaleness => &mut self.position_staleness,
        }
    }

    fn peek(&self, timer: Timer) -> Option<MonotonicTime> {
        match timer {
            Timer::PositionStaleness => self.position_staleness,
        }
    }
}
