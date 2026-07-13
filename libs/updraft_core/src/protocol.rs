use std::time::Duration;

/// A recorded event or request that may change shared state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Input {
    /// Advances the core's clock.
    Clock {
        /// Time since the runtime started.
        clock_time: Duration,
    },
}

/// Everything produced by handling one input.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Update {}
