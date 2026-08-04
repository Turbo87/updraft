use serde_json::json;
use updraft_geo::Polygon;
use updraft_units::{Length, MslAltitude, PressureAltitude};

/// A stable sequence number within one parsed airspace dataset.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AirspaceId(pub u32);

/// An ICAO airspace class with its OpenAIP numeric value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum AirspaceClass {
    A = 0,
    B = 1,
    C = 2,
    D = 3,
    E = 4,
    F = 5,
    G = 6,
    Unclassified = 8,
}

impl AirspaceClass {
    /// Returns the numeric ICAO class value from the OpenAIP schema.
    pub const fn openaip_code(self) -> u8 {
        self as u8
    }
}

/// An airspace type with its OpenAIP numeric value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum AirspaceType {
    Other = 0,
    Restricted = 1,
    Danger = 2,
    Prohibited = 3,
    ControlledTowerRegion = 4,
    TransponderMandatoryZone = 5,
    RadioMandatoryZone = 6,
    TerminalManeuveringArea = 7,
    TemporaryReservedArea = 8,
    TemporarySegregatedArea = 9,
    FlightInformationRegion = 10,
    UpperFlightInformationRegion = 11,
    AirDefenseIdentificationZone = 12,
    AirportTrafficZone = 13,
    MilitaryAirportTrafficZone = 14,
    Airway = 15,
    MilitaryTrainingRoute = 16,
    AlertArea = 17,
    WarningArea = 18,
    ProtectedArea = 19,
    HelicopterTrafficZone = 20,
    GlidingSector = 21,
    TransponderSetting = 22,
    TrafficInformationZone = 23,
    TrafficInformationArea = 24,
    MilitaryTrainingArea = 25,
    ControlArea = 26,
    AccSector = 27,
    AerialSportingOrRecreationalActivity = 28,
    LowAltitudeOverflightRestriction = 29,
    MilitaryRoute = 30,
    TsaOrTraFeedingRoute = 31,
    VfrSector = 32,
    FisSector = 33,
    LowerTrafficArea = 34,
    UpperTrafficArea = 35,
    MilitaryControlledTowerRegion = 36,
}

impl AirspaceType {
    /// Returns the numeric airspace type value from the OpenAIP schema.
    pub const fn openaip_code(self) -> u8 {
        self as u8
    }
}

/// An intended airspace activity with its documented OpenAIP numeric value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum AirspaceActivity {
    NoSpecificActivity = 0,
    Parachuting = 1,
    Aerobatics = 2,
    AeroclubAndAerialWork = 3,
    UltraLightMachine = 4,
    HangGlidingOrParagliding = 5,
}

impl AirspaceActivity {
    /// Returns the numeric activity value from the OpenAIP schema.
    pub const fn openaip_code(self) -> u8 {
        self as u8
    }
}

/// One supported OpenAir altitude limit in typed physical units.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AirspaceAltitude {
    Ground,
    Msl(MslAltitude),
    Agl(Length),
    FlightLevel(PressureAltitude),
    Unlimited,
}

/// One canonical polygon-only airspace.
#[derive(Clone, Debug, PartialEq)]
pub struct Airspace {
    /// The stable sequence number within this dataset.
    pub id: AirspaceId,
    /// The source airspace name when it is present.
    pub name: Option<Box<str>>,
    /// The required OpenAIP ICAO class.
    pub class: AirspaceClass,
    /// The required OpenAIP airspace type.
    pub type_code: AirspaceType,
    /// The intended activity when the source defines one.
    pub activity: Option<AirspaceActivity>,
    /// Whether the airspace is activated on demand.
    pub on_demand: Option<bool>,
    /// Whether the airspace is activated on request.
    pub on_request: Option<bool>,
    /// Whether a NOTAM announces when the airspace is active.
    pub by_notam: Option<bool>,
    /// Whether the airspace is subject to a special agreement.
    pub special_agreement: Option<bool>,
    /// The lower altitude limit.
    pub lower_limit: AirspaceAltitude,
    /// An optional hard minimum for the lower altitude limit.
    pub lower_limit_min: Option<AirspaceAltitude>,
    /// The upper altitude limit.
    pub upper_limit: AirspaceAltitude,
    /// An optional hard maximum for the upper altitude limit.
    pub upper_limit_max: Option<AirspaceAltitude>,
    /// The canonical polygon exterior ring.
    pub polygon: Polygon,
}

impl Airspace {
    /// Converts this airspace to a GeoJSON rendering subset.
    ///
    /// The `type` and `class` properties use numeric values from the
    /// [OpenAIP airspace schema](https://api.core.openaip.net/api/schemas/response/airspace/airspace-schema.json).
    pub fn to_geojson(&self) -> serde_json::Value {
        json!({
            "type": "Feature",
            "id": self.id.0,
            "properties": {
                "type": self.type_code.openaip_code(),
                "class": self.class.openaip_code(),
            },
            "geometry": {
                "type": "Polygon",
                "coordinates": [self.polygon.to_geojson_coordinates()],
            },
        })
    }
}

/// A complete canonical dataset parsed from one OpenAir source.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct AirspaceDataset {
    airspaces: Vec<Airspace>,
}

impl AirspaceDataset {
    /// Creates a canonical dataset from airspaces in parser order.
    pub fn from_airspaces(airspaces: Vec<Airspace>) -> Self {
        Self { airspaces }
    }

    /// Returns every airspace in parser order.
    pub fn airspaces(&self) -> &[Airspace] {
        &self.airspaces
    }
}

/// A safe OpenAir parsing or semantic conversion error kind.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum AirspaceParseError {
    #[error("the source parser rejected the input: {0}")]
    SourceParser(String),
    #[error("the source contains too many airspaces")]
    TooManyAirspaces,
    #[error("the airspace altitude is not supported")]
    UnsupportedAltitude,
}

/// A safe OpenAir polygon conversion error kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum AirspaceGeometryError {
    #[error("a coordinate is invalid")]
    InvalidCoordinate,
    #[error("a radius is invalid")]
    InvalidRadius,
    #[error("the polygon ring has fewer than three distinct vertices")]
    InvalidRing,
}

/// A dependency-independent failure from OpenAir parsing or polygon conversion.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum AirspaceImportError {
    #[error("OpenAir parsing failed for {airspace_id:?}: {kind}")]
    Parse {
        airspace_id: Option<AirspaceId>,
        kind: AirspaceParseError,
    },
    #[error("OpenAir geometry conversion failed for {airspace_id:?}: {kind}")]
    Geometry {
        airspace_id: AirspaceId,
        kind: AirspaceGeometryError,
    },
}

impl AirspaceImportError {
    /// Creates a parse import error for one dataset-local airspace.
    pub fn parse(airspace_id: AirspaceId, kind: AirspaceParseError) -> Self {
        Self::Parse {
            airspace_id: Some(airspace_id),
            kind,
        }
    }

    /// Creates a geometry import error for one dataset-local airspace.
    pub fn geometry(airspace_id: AirspaceId, kind: AirspaceGeometryError) -> Self {
        Self::Geometry { airspace_id, kind }
    }
}
