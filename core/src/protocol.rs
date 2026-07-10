use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::flight::{FlightChange, FlightInput, OwnshipPosition};

/// One message entering the core, the unit of recording and replay.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Input {
    Flight(FlightInput),
}

/// A client-visible state change, published on the state stream.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum Change {
    Flight(FlightChange),
}

/// A request to the outside world, executed by the runtime. Uninhabited
/// until the first effect lands; the runtime's exhaustive `match` will
/// force handling as variants appear.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Effect {}

/// The full client-visible state, delivered at the start of every state
/// stream so that reconnecting is just resubscribing.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct Snapshot {
    pub position: Option<OwnshipPosition>,
}

/// The result of handling one input.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Update {
    pub changes: Vec<Change>,
    pub effects: Vec<Effect>,
}
