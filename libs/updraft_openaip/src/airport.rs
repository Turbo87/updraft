//! The `apt` dataset.

use crate::code::codes;
use crate::common::{
    Countries, Elevation, ElevationGeoid, FrequencyUnit, HoursOfOperation, Image, Point,
};
use serde::Deserialize;

/// One airport record.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Airport {
    #[serde(rename = "_id")]
    pub id: Box<str>,
    pub name: Box<str>,
    pub r#type: AirportType,
    pub icao_code: Option<Box<str>>,
    pub iata_code: Option<Box<str>>,
    /// A local identifier that no international registry defines.
    pub alt_identifier: Option<Box<str>>,
    pub country: Countries,
    pub geometry: Point,
    pub elevation: Elevation,
    pub elevation_geoid: Option<ElevationGeoid>,
    /// The magnetic declination at the airport in degrees.
    pub magnetic_declination: f64,
    #[serde(default)]
    pub traffic_type: Box<[TrafficType]>,
    /// Prior permission is required.
    pub ppr: bool,
    pub private: bool,
    pub skydive_activity: bool,
    /// Gliders can only start by winch.
    pub winch_only: bool,
    pub services: Option<Services>,
    #[serde(default)]
    pub frequencies: Box<[Frequency]>,
    #[serde(default)]
    pub runways: Box<[Runway]>,
    pub hours_of_operation: Option<HoursOfOperation>,
    pub contact: Option<Box<str>>,
    #[serde(default)]
    pub telephone_services: Box<[TelephoneService]>,
    #[serde(default)]
    pub images: Box<[Image]>,
    pub remarks: Option<Box<str>>,
    pub created_at: Box<str>,
    pub created_by: Box<str>,
    pub updated_at: Box<str>,
    pub updated_by: Box<str>,
}

/// The services of an airport.
#[derive(Clone, Debug, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Services {
    #[serde(default)]
    pub fuel_types: Box<[FuelType]>,
    #[serde(default)]
    pub charging_stations: Box<[ChargingStation]>,
    #[serde(default)]
    pub glider_towing: Box<[GliderTowing]>,
    #[serde(default)]
    pub handling_facilities: Box<[HandlingFacility]>,
    #[serde(default)]
    pub passenger_facilities: Box<[PassengerFacility]>,
}

/// A radio frequency of an airport.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Frequency {
    #[serde(rename = "_id")]
    pub id: Box<str>,
    /// The frequency in the given unit, for example `118.075`.
    pub value: Box<str>,
    pub unit: FrequencyUnit,
    pub r#type: FrequencyType,
    pub name: Option<Box<str>>,
    pub primary: bool,
    /// The frequency is available for public use.
    pub public_use: bool,
    pub remarks: Option<Box<str>>,
}

/// A telephone service of an airport.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TelephoneService {
    pub name: Box<str>,
    pub phone_number: Box<str>,
    pub remarks: Option<Box<str>>,
}

/// One runway of an airport.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Runway {
    #[serde(rename = "_id")]
    pub id: Box<str>,
    /// The runway designator, for example `07L`.
    pub designator: Box<str>,
    pub true_heading: i32,
    /// The designator refers to true north instead of magnetic north.
    pub aligned_true_north: bool,
    pub operations: Operations,
    pub main_runway: bool,
    pub turn_direction: Option<TurnDirection>,
    pub landing_only: Option<bool>,
    pub take_off_only: Option<bool>,
    pub surface: Surface,
    pub dimension: Dimension,
    pub declared_distance: DeclaredDistance,
    pub threshold_location: Option<ThresholdLocation>,
    /// If set, only these aircraft types may use the runway.
    #[serde(default)]
    pub exclusive_aircraft_type: Box<[AircraftType]>,
    /// The pilot can control the lighting from the aircraft.
    pub pilot_ctrl_lighting: Option<bool>,
    #[serde(default)]
    pub lighting_system: Box<[LightingSystem]>,
    #[serde(default)]
    pub visual_approach_aids: Box<[VisualApproachAid]>,
    #[serde(default)]
    pub instrument_approach_aids: Box<[InstrumentApproachAid]>,
    pub remarks: Option<Box<str>>,
}

/// The surface of a runway.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Surface {
    pub composition: Box<[SurfaceComposition]>,
    pub main_composite: SurfaceComposition,
    pub condition: SurfaceCondition,
    /// The maximum permitted take-off weight.
    pub mtow: Option<Weight>,
    /// The pavement classification number.
    pub pcn: Option<Box<str>>,
    pub remarks: Option<Box<str>>,
}

/// The length and width of a runway.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
pub struct Dimension {
    pub length: Distance,
    pub width: Distance,
}

/// The declared distances of a runway.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeclaredDistance {
    /// Take-off run available.
    pub tora: Option<Distance>,
    /// Take-off distance available.
    pub toda: Option<Distance>,
    /// Accelerate-stop distance available.
    pub asda: Option<Distance>,
    /// Landing distance available.
    pub lda: Option<Distance>,
}

/// The threshold position of a runway.
#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
pub struct ThresholdLocation {
    pub geometry: Point,
    pub elevation: Elevation,
}

/// An instrument approach aid of a runway.
///
/// The response schema also documents `_id`. No export record carries it.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstrumentApproachAid {
    pub identifier: Option<Box<str>>,
    pub r#type: InstrumentApproachType,
    pub frequency: NavaidFrequency,
    /// The paired VHF channel.
    pub channel: Option<Box<str>>,
    /// The aid is aligned with true north instead of magnetic north.
    pub aligned_true_north: bool,
    pub hours_of_operation: Option<HoursOfOperation>,
    pub remarks: Option<Box<str>>,
}

/// The frequency of an instrument approach aid.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct NavaidFrequency {
    pub value: Box<str>,
    pub unit: FrequencyUnit,
}

/// A distance in the given unit.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
pub struct Distance {
    pub value: i32,
    pub unit: DistanceUnit,
}

/// A weight in the given unit.
#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
pub struct Weight {
    pub value: f64,
    pub unit: WeightUnit,
}

codes! {
    /// The unit of a distance value.
    pub enum DistanceUnit {
        0 => Meter,
    }
}

codes! {
    /// The unit of a weight value.
    pub enum WeightUnit {
        9 => Ton,
    }
}

codes! {
    /// The airport type.
    pub enum AirportType {
        /// Civil or military airport.
        0 => Airport,
        1 => GliderSite,
        2 => AirfieldCivil,
        3 => InternationalAirport,
        4 => HeliportMilitary,
        5 => MilitaryAerodrome,
        6 => UltraLightFlyingSite,
        7 => HeliportCivil,
        8 => AerodromeClosed,
        /// Airport or airfield that permits instrument flight rules.
        9 => AirportIfr,
        10 => AirfieldWater,
        11 => LandingStrip,
        12 => AgriculturalLandingStrip,
        13 => Altiport,
    }
}

codes! {
    /// The permitted flight rules of the airport traffic.
    pub enum TrafficType {
        /// Visual flight rules.
        0 => Vfr,
        /// Instrument flight rules.
        1 => Ifr,
    }
}

codes! {
    /// An available fuel type.
    pub enum FuelType {
        0 => SuperPlus,
        1 => Avgas,
        2 => JetA,
        3 => JetA1,
        4 => JetB,
        5 => Diesel,
        6 => AvgasUl91,
    }
}

codes! {
    /// An available charging station type.
    pub enum ChargingStation {
        0 => CcsE,
        1 => Ccs1,
        2 => Ccs2,
    }
}

codes! {
    /// An available glider launch method.
    pub enum GliderTowing {
        0 => SelfLaunch,
        1 => Winch,
        2 => Tow,
        3 => AutoTow,
        4 => Bungee,
        5 => GravityPowered,
    }
}

codes! {
    /// An available handling facility.
    pub enum HandlingFacility {
        0 => CargoHandling,
        1 => DeIcing,
        2 => Maintenance,
        3 => Security,
        4 => Shelter,
    }
}

codes! {
    /// An available passenger facility.
    pub enum PassengerFacility {
        0 => BankOffice,
        1 => PostOffice,
        2 => Customs,
        3 => Lodging,
        4 => MedicalFacility,
        5 => Restaurant,
        6 => Sanitation,
        7 => Transportation,
        8 => LaundryService,
        9 => Camping,
    }
}

codes! {
    /// The purpose of an airport frequency.
    pub enum FrequencyType {
        0 => Approach,
        1 => Apron,
        2 => Arrival,
        3 => Center,
        /// Common traffic advisory frequency.
        4 => Ctaf,
        5 => Delivery,
        6 => Departure,
        /// Flight information service.
        7 => Fis,
        8 => Gliding,
        9 => Ground,
        10 => Information,
        11 => Multicom,
        12 => Unicom,
        13 => Radar,
        14 => Tower,
        /// Automatic terminal information service.
        15 => Atis,
        16 => Radio,
        17 => Other,
        18 => Airmet,
        /// Automated weather observing system.
        19 => Awos,
        20 => Lights,
        /// Meteorological information for aircraft in flight.
        21 => Volmet,
        /// Aerodrome flight information service.
        22 => Afis,
        /// Automated surface observing system.
        23 => Asos,
        /// Automated weather information service.
        24 => Awis,
        25 => Emergency,
        26 => ClearanceDelivery,
        27 => RemoteComOutlet,
        28 => GroundComOutlet,
        29 => FlightServiceStation,
        30 => ClassC,
        31 => ClassB,
        32 => VfrAdvisory,
        /// Terminal radar service area.
        33 => Trsa,
    }
}

codes! {
    /// The operational state of a runway.
    pub enum Operations {
        0 => Active,
        1 => TemporarilyClosed,
        2 => Closed,
    }
}

codes! {
    /// The permitted take-off and landing turn direction of a runway.
    pub enum TurnDirection {
        0 => Right,
        1 => Left,
        2 => Both,
    }
}

codes! {
    /// A runway surface material.
    pub enum SurfaceComposition {
        0 => Asphalt,
        1 => Concrete,
        2 => Grass,
        3 => Sand,
        4 => Water,
        /// Bituminous tar or asphalt, also called earth cement.
        5 => Bituminous,
        6 => Brick,
        /// Macadam or tarmac of water-bound crushed rock.
        7 => Macadam,
        8 => Stone,
        9 => Coral,
        10 => Clay,
        /// Laterite, a high iron clay of tropical areas.
        11 => Laterite,
        12 => Gravel,
        13 => Earth,
        14 => Ice,
        15 => Snow,
        /// Protective laminate, usually rubber.
        16 => ProtectiveLaminate,
        17 => Metal,
        /// Portable landing mat, usually aluminium.
        18 => LandingMat,
        19 => PiercedSteelPlanking,
        20 => Wood,
        21 => NonBituminousMix,
        22 => Unknown,
    }
}

codes! {
    /// The condition of a runway surface.
    pub enum SurfaceCondition {
        0 => Good,
        1 => Fair,
        2 => Poor,
        3 => Unsafe,
        4 => Deformed,
        5 => Unknown,
    }
}

codes! {
    /// An aircraft type that may use a runway.
    pub enum AircraftType {
        0 => SingleEnginePiston,
        1 => SingleEngineTurbine,
        2 => MultiEnginePiston,
        3 => MultiEngine,
        4 => HighPerformanceAircraft,
        5 => TouringMotorGlider,
        6 => Experimental,
        7 => VeryLightAircraft,
        8 => Glider,
        9 => LightSportAircraft,
        10 => UltraLightAircraft,
        11 => HangGlider,
        12 => Paraglider,
        13 => Balloon,
    }
}

codes! {
    /// A runway lighting system.
    pub enum LightingSystem {
        0 => RunwayEndIdentifierLights,
        1 => RunwayEndLights,
        2 => RunwayEdgeLights,
        3 => RunwayCenterLineLightingSystem,
        4 => TouchdownZoneLights,
        5 => TaxiwayCenterlineLeadOffLights,
        6 => TaxiwayCenterlineLeadOnLights,
        7 => LandAndHoldShortLights,
        8 => ApproachLightingSystem,
        9 => ThresholdLights,
    }
}

codes! {
    /// A visual approach aid of a runway.
    pub enum VisualApproachAid {
        /// Visual approach slope indicator.
        0 => Vasi,
        /// Precision approach path indicator.
        1 => Papi,
        /// Tri-colour visual approach slope indicator.
        2 => TriColorVasi,
        /// Pulsating visual approach slope indicator.
        3 => PulsatingVasi,
        /// Alignment of elements system.
        4 => AlignmentOfElements,
    }
}

codes! {
    /// The type of an instrument approach aid.
    pub enum InstrumentApproachType {
        /// Instrument landing system.
        0 => Ils,
        /// Localizer approach.
        1 => Loc,
        /// Localizer type directional aid approach.
        2 => Lda,
        /// Locator, also called compass locator.
        3 => Locator,
        /// Distance measuring equipment.
        4 => Dme,
        /// Glide path.
        5 => GlidePath,
    }
}
