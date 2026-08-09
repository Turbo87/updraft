use crate::WindDirection;
use updraft_units::Angle;

/// One hang gliding or paragliding site.
#[derive(Clone, Debug, PartialEq)]
pub struct HangGlidingSite {
    pub site_type: HangGlidingType,
    /// The wing categories that the site suits.
    pub categories: Vec<HangGlidingCategory>,
    /// The ways to reach the site.
    pub access: Vec<HangGlidingAccess>,
    pub certified: Option<bool>,
    /// The suitable wind directions. OpenAIP treats every other direction as
    /// unfavorable or dangerous.
    pub suitable_wind_directions: Vec<WindDirection>,
    /// The middle direction of the first take-off sector. The CUP `rwdir`
    /// column supplies it for styles 20 and 21.
    pub take_off_direction: Option<Angle>,
}

/// The role of a hang gliding site.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HangGlidingType {
    /// CUP style 20 and OpenAIP hang gliding type 0.
    TakeOff,
    /// CUP style 21 and OpenAIP hang gliding type 1.
    Landing,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HangGlidingCategory {
    Paraglider,
    FixedWing,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HangGlidingAccess {
    Hike,
    CableCar,
    ChairLift,
    Car,
    Shuttle,
}
