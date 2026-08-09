use crate::WindDirection;

/// One thermal hotspot.
#[derive(Clone, Debug, PartialEq)]
pub struct Hotspot {
    pub hotspot_type: HotspotType,
    pub reliability: Option<HotspotReliability>,
    pub occurrence: Option<HotspotOccurrence>,
    /// The aircraft categories that the hotspot suits. An empty list means
    /// that the source sets no restriction.
    pub aircraft_categories: Vec<HotspotAircraftCategory>,
    /// The times of day at which the hotspot can work.
    pub times_of_day: Vec<TimeOfDay>,
    /// The times of day at which the hotspot works best.
    pub favorable_times_of_day: Vec<TimeOfDay>,
    /// The wind directions in which the hotspot works best.
    pub favorable_wind_directions: Vec<WindDirection>,
    /// The wind directions that the hotspot needs to work.
    pub required_wind_directions: Vec<WindDirection>,
}

/// The origin of the lift.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HotspotType {
    Natural,
    Artificial,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HotspotReliability {
    Poor,
    Fair,
    High,
    VeryHigh,
}

/// How regularly the lift occurs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HotspotOccurrence {
    IrregularIntervals,
    ScheduledInterval,
    NearlyConstant,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HotspotAircraftCategory {
    Glider,
    HangGlider,
    Paraglider,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TimeOfDay {
    EarlyMorning,
    Morning,
    Noon,
    Afternoon,
    Evening,
}
