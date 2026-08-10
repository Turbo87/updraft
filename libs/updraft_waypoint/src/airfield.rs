use crate::{Frequency, FrequencyType, OperatingHours};
use updraft_geo::LatLon;
use updraft_units::{Angle, Length, Mass, MslAltitude};

/// One landing site with its runways and radio frequencies.
///
/// CUP supplies at most one runway and one frequency. OpenAIP supplies
/// ordered lists.
#[derive(Clone, Debug, PartialEq)]
pub struct Airfield {
    pub airfield_type: AirfieldType,
    /// Whether the site has civil use. A joint site also has military use.
    pub civil: Option<bool>,
    /// Whether the site has military use. A joint site also has civil use.
    pub military: Option<bool>,
    /// Whether the site is closed.
    pub closed: Option<bool>,
    pub icao_code: Option<Box<str>>,
    pub iata_code: Option<Box<str>>,
    /// A further source identifier for airfields without an ICAO code.
    pub alt_identifier: Option<Box<str>>,
    pub traffic_types: Vec<TrafficType>,
    pub magnetic_declination: Option<Angle>,
    /// Whether a landing needs prior permission.
    pub prior_permission_required: Option<bool>,
    pub private_use: Option<bool>,
    pub skydive_activity: Option<bool>,
    /// Whether the site permits winch launches only.
    pub winch_only: Option<bool>,
    pub services: Option<AirfieldServices>,
    pub frequencies: Vec<AirfieldFrequency>,
    pub runways: Vec<Runway>,
    pub hours_of_operation: Option<OperatingHours>,
}

/// The form of a landing site.
///
/// The form says what a pilot lands on. Who operates the site and which
/// aircraft may use it are separate attributes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AirfieldType {
    /// A prepared aerodrome. CUP styles 2, 4, and 5, and OpenAIP airport
    /// types 0, 1, 2, 3, 5, 6, 8, and 9.
    Aerodrome,
    /// CUP style 3. A field for an unplanned landing.
    Outlanding,
    /// OpenAIP airport type 11. A simple strip.
    LandingStrip,
    /// OpenAIP airport type 12.
    AgriculturalLandingStrip,
    /// OpenAIP airport type 13. A mountain aerodrome with a sloped runway.
    Altiport,
    /// OpenAIP airport type 10.
    WaterAirfield,
    /// OpenAIP airport types 4 and 7.
    Heliport,
}

/// The flight rules that an airfield accepts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrafficType {
    Vfr,
    Ifr,
}

/// One airfield radio frequency.
#[derive(Clone, Debug, PartialEq)]
pub struct AirfieldFrequency {
    pub frequency: Frequency,
    /// The frequency purpose. CUP supplies no purpose for its single value.
    pub frequency_type: Option<FrequencyType>,
    pub name: Option<Box<str>>,
    pub primary: Option<bool>,
    pub public_use: Option<bool>,
    pub remarks: Option<Box<str>>,
}

/// The services that an airfield supplies.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AirfieldServices {
    pub fuel_types: Vec<FuelType>,
    pub charging_stations: Vec<ChargingStation>,
    pub glider_towing: Vec<GliderTowing>,
    pub handling_facilities: Vec<HandlingFacility>,
    pub passenger_facilities: Vec<PassengerFacility>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FuelType {
    SuperPlus,
    Avgas,
    JetA,
    JetA1,
    JetB,
    Diesel,
    AvgasUl91,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChargingStation {
    CcsE,
    Ccs1,
    Ccs2,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GliderTowing {
    SelfLaunch,
    Winch,
    Tow,
    AutoTow,
    Bungee,
    GravityPowered,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HandlingFacility {
    CargoHandling,
    DeIcing,
    Maintenance,
    Security,
    Shelter,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PassengerFacility {
    BankOffice,
    PostOffice,
    Customs,
    Lodging,
    MedicalFacility,
    Restaurant,
    Sanitation,
    Transportation,
    LaundryService,
    Camping,
}

/// One runway.
///
/// A CUP airfield supplies only the direction and the dimensions. OpenAIP
/// supplies one entry for each runway direction.
#[derive(Clone, Debug, PartialEq)]
pub struct Runway {
    /// The runway designator, for example `07L`.
    pub designator: Option<Box<str>>,
    /// The runway direction. The CUP column declares no reference datum.
    pub true_heading: Option<Angle>,
    pub aligned_true_north: Option<bool>,
    pub operations: Option<RunwayOperations>,
    pub main_runway: Option<bool>,
    pub turn_direction: Option<TurnDirection>,
    pub landing_only: Option<bool>,
    pub take_off_only: Option<bool>,
    pub surface: Option<RunwaySurface>,
    pub dimension: Option<RunwayDimension>,
    pub declared_distances: Option<DeclaredDistances>,
    pub threshold_location: Option<ThresholdLocation>,
    /// The aircraft types that may use this runway. An empty list means that
    /// the runway has no aircraft restriction. A glider site and an
    /// ultralight site state their aircraft type here.
    pub exclusive_aircraft_types: Vec<AircraftType>,
    pub pilot_controlled_lighting: Option<bool>,
    pub lighting_systems: Vec<LightingSystem>,
    pub visual_approach_aids: Vec<VisualApproachAid>,
    pub instrument_approach_aids: Vec<InstrumentApproachAid>,
    pub remarks: Option<Box<str>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RunwayOperations {
    Active,
    TemporarilyClosed,
    Closed,
}

/// The permitted take-off and landing turn directions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TurnDirection {
    Right,
    Left,
    Both,
}

/// The runway surface.
#[derive(Clone, Debug, PartialEq)]
pub struct RunwaySurface {
    pub compositions: Vec<RunwayComposition>,
    pub main_composition: Option<RunwayComposition>,
    pub condition: Option<RunwayCondition>,
    /// The maximum take-off weight that the runway permits.
    pub max_take_off_weight: Option<Mass>,
    /// The unvalidated pavement classification number.
    pub pcn: Option<Box<str>>,
    pub remarks: Option<Box<str>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RunwayComposition {
    Asphalt,
    Concrete,
    Grass,
    Sand,
    Water,
    BituminousTar,
    Brick,
    Macadam,
    Stone,
    Coral,
    Clay,
    Laterite,
    Gravel,
    Earth,
    Ice,
    Snow,
    ProtectiveLaminate,
    Metal,
    LandingMat,
    PiercedSteelPlanking,
    Wood,
    NonBituminousMix,
    Unknown,
    /// A solid surface without further classification. CUP style 5 supplies
    /// it, because CUP names no material.
    Solid,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RunwayCondition {
    Good,
    Fair,
    Poor,
    Unsafe,
    Deformed,
    Unknown,
}

/// The physical runway dimensions.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RunwayDimension {
    pub length: Option<Length>,
    pub width: Option<Length>,
}

/// The declared distances of a runway.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DeclaredDistances {
    /// Take-off run available.
    pub tora: Option<Length>,
    /// Take-off distance available.
    pub toda: Option<Length>,
    /// Accelerate-stop distance available.
    pub asda: Option<Length>,
    /// Landing distance available.
    pub lda: Option<Length>,
}

/// The runway threshold position.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ThresholdLocation {
    pub position: LatLon,
    pub elevation: Option<MslAltitude>,
}

/// An aircraft type that a runway permits.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AircraftType {
    SingleEnginePiston,
    SingleEngineTurbine,
    MultiEnginePiston,
    MultiEngine,
    HighPerformanceAircraft,
    TouringMotorGlider,
    Experimental,
    VeryLightAircraft,
    Glider,
    LightSportAircraft,
    UltralightAircraft,
    HangGlider,
    Paraglider,
    Balloon,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LightingSystem {
    RunwayEndIdentifierLights,
    RunwayEndLights,
    RunwayEdgeLights,
    RunwayCenterLineLights,
    TouchdownZoneLights,
    TaxiwayCenterlineLeadOffLights,
    TaxiwayCenterlineLeadOnLights,
    LandAndHoldShortLights,
    ApproachLightingSystem,
    ThresholdLights,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VisualApproachAid {
    VisualApproachSlopeIndicator,
    PrecisionApproachPathIndicator,
    TriColorVisualApproachSlopeIndicator,
    PulsatingVisualApproachSlopeIndicator,
    AlignmentOfElementsSystem,
}

/// One instrument approach aid of a runway.
#[derive(Clone, Debug, PartialEq)]
pub struct InstrumentApproachAid {
    pub approach_type: InstrumentApproachType,
    pub identifier: Option<Box<str>>,
    pub frequency: Option<Frequency>,
    pub channel: Option<Box<str>>,
    pub aligned_true_north: Option<bool>,
    pub hours_of_operation: Option<OperatingHours>,
    pub remarks: Option<Box<str>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InstrumentApproachType {
    /// Instrument landing system.
    Ils,
    /// Localizer approach.
    Loc,
    /// Localizer type directional aid approach.
    Lda,
    /// Compass locator.
    Locator,
    /// Distance measuring equipment.
    Dme,
    /// Glide path.
    GlidePath,
}
