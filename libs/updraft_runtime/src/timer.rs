//! The timer thread: turns the core's next deadline into `Tick` inputs.
//!
//! The core owns timer identity and deadlines; the runtime only sleeps and
//! reports that time advanced. After each `Update`, the loop arms this
//! thread with `Update.next_deadline`. When that deadline elapses the
//! thread submits a single [`Input::Tick`] stamped with the current
//! monotonic time; a new arm replaces the pending deadline, and `None`
//! cancels it.

use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::time::{Duration, Instant};

use updraft_core::{Input, MonotonicTime};

use crate::LoopSender;

/// A message to the timer thread.
pub(crate) enum TimerMsg {
    /// Wait until this deadline (or forever, when `None`).
    Arm(Option<MonotonicTime>),
    /// Stop the timer; the runtime is shutting down.
    Stop,
}

/// The timer thread body. `origin` is the shared monotonic clock origin.
pub(crate) fn run(rx: &Receiver<TimerMsg>, loop_tx: &LoopSender, origin: Instant) {
    let mut deadline: Option<MonotonicTime> = None;

    loop {
        let next = match deadline {
            // Nothing scheduled: block until the loop arms or stops us.
            None => match rx.recv() {
                Ok(msg) => msg,
                Err(_) => break,
            },
            Some(at) => {
                let wait = remaining(origin, at);
                if wait.is_zero() {
                    // Deadline reached: fire and disarm.
                    if loop_tx.send_input(Input::Tick(now(origin))).is_err() {
                        break;
                    }
                    deadline = None;
                    continue;
                }
                match rx.recv_timeout(wait) {
                    Ok(msg) => msg,
                    Err(RecvTimeoutError::Timeout) => {
                        if loop_tx.send_input(Input::Tick(now(origin))).is_err() {
                            break;
                        }
                        deadline = None;
                        continue;
                    }
                    Err(RecvTimeoutError::Disconnected) => break,
                }
            }
        };

        match next {
            TimerMsg::Arm(at) => deadline = at,
            TimerMsg::Stop => break,
        }
    }
}

/// The current time on the shared monotonic timeline.
fn now(origin: Instant) -> MonotonicTime {
    MonotonicTime::from_nanos(origin.elapsed().as_nanos() as u64)
}

/// How long until `deadline`, saturating at zero when it is already past.
fn remaining(origin: Instant, deadline: MonotonicTime) -> Duration {
    let target = origin + Duration::from_nanos(deadline.as_nanos());
    target.saturating_duration_since(Instant::now())
}
