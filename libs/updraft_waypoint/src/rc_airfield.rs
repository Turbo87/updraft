use crate::OperatingHours;
use updraft_units::{Length, MslAltitude, PressureAltitude};

/// One radio-controlled model aircraft airfield.
#[derive(Clone, Debug, PartialEq)]
pub struct RcAirfield {
    pub operator: Option<Box<str>>,
    pub electric: Option<bool>,
    pub combustion: Option<bool>,
    pub turbine: Option<bool>,
    /// The highest altitude that model flights may use.
    pub permitted_altitude: Option<VerticalLimit>,
    pub hours_of_operation: Option<OperatingHours>,
}

/// One vertical limit in typed physical units.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum VerticalLimit {
    /// A height above ground level.
    Agl(Length),
    /// An altitude above mean sea level.
    Msl(MslAltitude),
    /// A flight level above the standard pressure datum.
    FlightLevel(PressureAltitude),
}
