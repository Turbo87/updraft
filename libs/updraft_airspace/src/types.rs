use serde_json::json;
use updraft_geo::Polygon;
use updraft_units::{Length, MslAltitude, PressureAltitude};

/// A stable sequence number within one parsed airspace dataset.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AirspaceId(pub u32);

/// A modern OpenAir airspace class.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AirspaceClass {
    A,
    B,
    C,
    D,
    E,
    F,
    G,
    Unclassified,
}

impl AirspaceClass {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::A => "A",
            Self::B => "B",
            Self::C => "C",
            Self::D => "D",
            Self::E => "E",
            Self::F => "F",
            Self::G => "G",
            Self::Unclassified => "UNC",
        }
    }
}

/// A known OpenAir type or an unknown normalized type code.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AirspaceType {
    RemoteCommunicationArea,
    AirDefenceIdentificationZone,
    AlertArea,
    AerialSportingOrRecreationalActivity,
    AerodromeTrafficZone,
    Airway,
    ControlledTrafficArea,
    ControlZone,
    Custom,
    FlightInformationRegion,
    FlightInformationServiceSector,
    GlidingSector,
    HelicopterTrafficZone,
    LowerTrafficArea,
    MilitaryAirportTrafficZone,
    MilitaryTrainingArea,
    MilitaryTrainingRoute,
    NotamAffectedArea,
    OverflightRestriction,
    ProhibitedArea,
    DangerArea,
    RestrictedArea,
    RadioMandatoryZone,
    TemporaryFlightRestriction,
    TrafficInformationArea,
    TrafficInformationZone,
    TerminalManoeuvringArea,
    TransponderMandatoryZone,
    TemporaryReservedArea,
    TemporaryReservedOrSegregatedAreaFeedingRoute,
    TransponderRecommendedZone,
    TemporarySegregatedArea,
    UpperFlightInformationRegion,
    UpperTrafficArea,
    DesignatedVisualFlightRulesRoute,
    VisualFlightRulesSector,
    WarningArea,
    Unknown(Box<str>),
}

impl AirspaceType {
    /// Creates an airspace type from a normalized OpenAir type code.
    pub fn from_code(code: &str) -> Self {
        match code {
            "ACCSEC" => Self::RemoteCommunicationArea,
            "ADIZ" => Self::AirDefenceIdentificationZone,
            "ALERT" => Self::AlertArea,
            "ASRA" => Self::AerialSportingOrRecreationalActivity,
            "ATZ" => Self::AerodromeTrafficZone,
            "AWY" => Self::Airway,
            "CTA" => Self::ControlledTrafficArea,
            "CTR" => Self::ControlZone,
            "CUSTOM" => Self::Custom,
            "FIR" => Self::FlightInformationRegion,
            "FIS" => Self::FlightInformationServiceSector,
            "GSEC" => Self::GlidingSector,
            "HTZ" => Self::HelicopterTrafficZone,
            "LTA" => Self::LowerTrafficArea,
            "MATZ" => Self::MilitaryAirportTrafficZone,
            "MTA" => Self::MilitaryTrainingArea,
            "MTR" => Self::MilitaryTrainingRoute,
            "N" => Self::NotamAffectedArea,
            "OFR" => Self::OverflightRestriction,
            "P" => Self::ProhibitedArea,
            "Q" => Self::DangerArea,
            "R" => Self::RestrictedArea,
            "RMZ" => Self::RadioMandatoryZone,
            "TFR" => Self::TemporaryFlightRestriction,
            "TIA" => Self::TrafficInformationArea,
            "TIZ" => Self::TrafficInformationZone,
            "TMA" => Self::TerminalManoeuvringArea,
            "TMZ" => Self::TransponderMandatoryZone,
            "TRA" => Self::TemporaryReservedArea,
            "TRAFR" => Self::TemporaryReservedOrSegregatedAreaFeedingRoute,
            "TRZ" => Self::TransponderRecommendedZone,
            "TSA" => Self::TemporarySegregatedArea,
            "UIR" => Self::UpperFlightInformationRegion,
            "UTA" => Self::UpperTrafficArea,
            "VFRR" => Self::DesignatedVisualFlightRulesRoute,
            "VFRSEC" => Self::VisualFlightRulesSector,
            "WARNING" => Self::WarningArea,
            _ => Self::Unknown(code.into()),
        }
    }

    /// Returns the normalized OpenAir type code.
    pub fn as_str(&self) -> &str {
        match self {
            Self::RemoteCommunicationArea => "ACCSEC",
            Self::AirDefenceIdentificationZone => "ADIZ",
            Self::AlertArea => "ALERT",
            Self::AerialSportingOrRecreationalActivity => "ASRA",
            Self::AerodromeTrafficZone => "ATZ",
            Self::Airway => "AWY",
            Self::ControlledTrafficArea => "CTA",
            Self::ControlZone => "CTR",
            Self::Custom => "CUSTOM",
            Self::FlightInformationRegion => "FIR",
            Self::FlightInformationServiceSector => "FIS",
            Self::GlidingSector => "GSEC",
            Self::HelicopterTrafficZone => "HTZ",
            Self::LowerTrafficArea => "LTA",
            Self::MilitaryAirportTrafficZone => "MATZ",
            Self::MilitaryTrainingArea => "MTA",
            Self::MilitaryTrainingRoute => "MTR",
            Self::NotamAffectedArea => "N",
            Self::OverflightRestriction => "OFR",
            Self::ProhibitedArea => "P",
            Self::DangerArea => "Q",
            Self::RestrictedArea => "R",
            Self::RadioMandatoryZone => "RMZ",
            Self::TemporaryFlightRestriction => "TFR",
            Self::TrafficInformationArea => "TIA",
            Self::TrafficInformationZone => "TIZ",
            Self::TerminalManoeuvringArea => "TMA",
            Self::TransponderMandatoryZone => "TMZ",
            Self::TemporaryReservedArea => "TRA",
            Self::TemporaryReservedOrSegregatedAreaFeedingRoute => "TRAFR",
            Self::TransponderRecommendedZone => "TRZ",
            Self::TemporarySegregatedArea => "TSA",
            Self::UpperFlightInformationRegion => "UIR",
            Self::UpperTrafficArea => "UTA",
            Self::DesignatedVisualFlightRulesRoute => "VFRR",
            Self::VisualFlightRulesSector => "VFRSEC",
            Self::WarningArea => "WARNING",
            Self::Unknown(value) => value,
        }
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
    /// The modern OpenAir class when the source defines one.
    pub class: Option<AirspaceClass>,
    /// The OpenAir type when one applies.
    pub type_code: Option<AirspaceType>,
    /// The lower altitude limit.
    pub lower_bound: AirspaceAltitude,
    /// The upper altitude limit.
    pub upper_bound: AirspaceAltitude,
    /// The canonical polygon exterior ring.
    pub polygon: Polygon,
}

impl Airspace {
    /// Converts this airspace to a GeoJSON feature.
    pub fn to_geojson(&self) -> serde_json::Value {
        json!({
            "type": "Feature",
            "properties": {
                "id": self.id.0,
                "class": self.class.as_ref().map(AirspaceClass::as_str),
                "type": self.type_code.as_ref().map(AirspaceType::as_str),
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
    #[error("the airspace type code is empty")]
    EmptyTypeCode,
    #[error("the airspace has no class or type")]
    MissingClassOrType,
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
