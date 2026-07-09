use crate::{FlightChange, FlightInput, OwnshipPosition};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Input {
    Flight(FlightInput),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Change {
    Flight(FlightChange),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Effect {}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Snapshot {
    pub position: Option<OwnshipPosition>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Update {
    pub changes: Vec<Change>,
    pub effects: Vec<Effect>,
}
