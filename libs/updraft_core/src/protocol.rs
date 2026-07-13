use std::time::Duration;

use crate::flight;

/// A recorded event or request that may change shared state.
#[derive(Clone, Debug, PartialEq)]
pub enum Input {
    /// Advances the core's clock.
    Clock {
        /// Time since the runtime started.
        clock_time: Duration,
    },
    /// An input owned by the flight domain.
    Flight(flight::Input),
}

/// A client-visible state update produced after handling an input.
#[derive(Clone, Debug, PartialEq)]
pub enum Change {
    Flight(flight::Change),
}

/// The shared current state for a newly subscribing client.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Snapshot {
    pub flight: flight::Snapshot,
}

/// Everything produced by handling one input.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Update {
    /// Client-visible state updates, published in input order.
    pub changes: Vec<Change>,
}
