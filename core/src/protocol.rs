use serde::Serialize;
use ts_rs::TS;

use crate::{FlightChange, FlightInput, OwnshipPosition};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Input {
    Flight(FlightInput),
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum Change {
    Flight(FlightChange),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Effect {}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct Snapshot {
    pub position: Option<OwnshipPosition>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Update {
    pub changes: Vec<Change>,
    pub effects: Vec<Effect>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum StateMessage {
    Snapshot(Snapshot),
    Changes(Vec<Change>),
}
