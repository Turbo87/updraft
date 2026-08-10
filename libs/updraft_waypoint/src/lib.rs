//! Canonical waypoint data.
//!
//! A waypoint is one named point of interest. The canonical [`Waypoint`]
//! holds the attributes that every source supplies: identity, position,
//! elevation, and kind. [`WaypointKind`] holds the attributes that only
//! one family of points supplies, for example runways or obstacle height.
//!
//! The model represents the SeeYou CUP format and the non-airspace OpenAIP
//! datasets: airports, navaids, obstacles, hotspots, reporting points, hang
//! gliding sites, and RC airfields. Every record of those sources is one
//! point, so one point model covers all of them.
//!
//! The model excludes service and provenance values: document identifiers,
//! audit stamps, `OpenStreetMap` references, images, contact values, and the
//! CUP `userdata` and `pics` columns. It also excludes the OpenAIP
//! `elevationGeoid` values, because `updraft_egm96` owns geoid separation.
//!
//! Numeric source codes are not part of this model. Each importer owns the
//! conversion from its own wire values.

mod airfield;
mod frequency;
mod hang_gliding;
mod hotspot;
mod navaid;
mod obstacle;
mod operating_hours;
mod rc_airfield;
mod reporting_point;
mod waypoint;
mod wind;

pub use airfield::{
    AircraftType, Airfield, AirfieldFrequency, AirfieldOperator, AirfieldServices, AirfieldType,
    ChargingStation, DeclaredDistances, FuelType, GliderTowing, HandlingFacility,
    InstrumentApproachAid, InstrumentApproachType, LightingSystem, PassengerFacility, Runway,
    RunwayComposition, RunwayCondition, RunwayDimension, RunwayOperations, RunwaySurface,
    ThresholdLocation, TrafficType, TurnDirection, VisualApproachAid,
};
pub use frequency::{Frequency, FrequencyType, FrequencyUnit, FrequencyValue};
pub use hang_gliding::{HangGlidingAccess, HangGlidingCategory, HangGlidingSite, HangGlidingType};
pub use hotspot::{
    Hotspot, HotspotAircraftCategory, HotspotOccurrence, HotspotReliability, HotspotType, TimeOfDay,
};
pub use navaid::{Navaid, NavaidType};
pub use obstacle::{Obstacle, ObstacleType};
pub use operating_hours::{OperatingHours, OperatingPeriod, OperatingSchedule};
pub use rc_airfield::{RcAirfield, VerticalLimit};
pub use reporting_point::ReportingPoint;
pub use waypoint::{Waypoint, WaypointDataset, WaypointId, WaypointKind};
pub use wind::WindDirection;
