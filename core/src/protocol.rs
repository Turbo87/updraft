use serde::Serialize;

use crate::{FlightChange, FlightInput, OwnshipPosition};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Input {
    Flight(FlightInput),
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Change {
    Flight(FlightChange),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Effect {}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize)]
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
