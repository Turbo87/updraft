use crate::{
    AirspaceFrequency, AirspaceOperatingHours, AirspaceOperatingPeriod, AirspaceOperatingSchedule,
    AirspaceTransponderSetting,
};
use serde_json::json;
use time::{OffsetDateTime, Time, format_description::well_known::Rfc3339};
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

#[derive(Clone, Copy)]
#[repr(u8)]
enum OpenAipLimitUnit {
    Meters = 0,
    Feet = 1,
    FlightLevel = 6,
}

#[derive(Clone, Copy)]
#[repr(u8)]
enum OpenAipReferenceDatum {
    Ground = 0,
    MeanSeaLevel = 1,
    StandardPressure = 2,
}

impl AirspaceAltitude {
    fn to_openaip_limit(self) -> serde_json::Value {
        match self {
            Self::Ground => {
                openaip_limit(0, OpenAipLimitUnit::Meters, OpenAipReferenceDatum::Ground)
            }
            Self::Msl(altitude) => openaip_limit(
                whole_feet(altitude.into_inner()),
                OpenAipLimitUnit::Feet,
                OpenAipReferenceDatum::MeanSeaLevel,
            ),
            Self::Agl(height) => openaip_limit(
                whole_feet(height),
                OpenAipLimitUnit::Feet,
                OpenAipReferenceDatum::Ground,
            ),
            Self::FlightLevel(altitude) => openaip_limit(
                whole_feet(altitude.into_inner()) / 100,
                OpenAipLimitUnit::FlightLevel,
                OpenAipReferenceDatum::StandardPressure,
            ),
            Self::Unlimited => json!({ "unlimited": true }),
        }
    }
}

fn whole_feet(length: Length) -> i64 {
    length.as_feet().round() as i64
}

fn openaip_limit(
    value: i64,
    unit: OpenAipLimitUnit,
    reference_datum: OpenAipReferenceDatum,
) -> serde_json::Value {
    json!({
        "value": value,
        "unit": unit as u8,
        "referenceDatum": reference_datum as u8,
    })
}

fn format_openaip_time(time: Time) -> String {
    format!(
        "{:02}:{:02}:{:02}",
        time.hour(),
        time.minute(),
        time.second()
    )
}

fn format_openaip_datetime(datetime: OffsetDateTime) -> String {
    datetime
        .format(&Rfc3339)
        .expect("airspace activation date must support RFC 3339")
}

fn operating_period_to_openaip_json(period: &AirspaceOperatingPeriod) -> serde_json::Value {
    let mut value = json!({
        "dayOfWeek": period.day_of_week.number_days_from_monday(),
        "sunrise": false,
        "sunset": false,
        "byNotam": false,
        "publicHolidaysExcluded": period.public_holidays_excluded,
    });
    match period.schedule {
        AirspaceOperatingSchedule::Fixed {
            start_time,
            end_time,
        } => {
            value["startTime"] = json!(format_openaip_time(start_time));
            value["endTime"] = json!(format_openaip_time(end_time));
        }
        AirspaceOperatingSchedule::FixedStartUntilSunset { start_time } => {
            value["startTime"] = json!(format_openaip_time(start_time));
            value["sunset"] = json!(true);
        }
        AirspaceOperatingSchedule::SunriseUntilFixedEnd { end_time } => {
            value["endTime"] = json!(format_openaip_time(end_time));
            value["sunrise"] = json!(true);
        }
        AirspaceOperatingSchedule::SunriseUntilSunset => {
            value["sunrise"] = json!(true);
            value["sunset"] = json!(true);
        }
        AirspaceOperatingSchedule::NoSpecifiedTime => {}
        AirspaceOperatingSchedule::ByNotam => value["byNotam"] = json!(true),
    }
    if let Some(remarks) = period.remarks.as_deref() {
        value["remarks"] = json!(remarks);
    }
    value
}

fn operating_hours_to_openaip_json(hours: &AirspaceOperatingHours) -> serde_json::Value {
    let mut value = json!({
        "operatingHours": hours
            .operating_periods()
            .iter()
            .map(operating_period_to_openaip_json)
            .collect::<Vec<_>>(),
    });
    if let Some(remarks) = hours.remarks.as_deref() {
        value["remarks"] = json!(remarks);
    }
    value
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
    /// Whether the airspace represents a non-mandatory compliance request.
    pub request_compliance: Option<bool>,
    /// The unvalidated source country codes associated with this airspace.
    pub country_codes: Vec<Box<str>>,
    /// The source-defined radio frequencies.
    pub frequencies: Vec<AirspaceFrequency>,
    /// The source-defined transponder settings.
    pub transponder_settings: Vec<AirspaceTransponderSetting>,
    /// The operating hours when the source defines them.
    pub hours_of_operation: Option<AirspaceOperatingHours>,
    /// The activation start instant when the source defines it.
    pub active_from: Option<OffsetDateTime>,
    /// The activation end instant when the source defines it.
    pub active_until: Option<OffsetDateTime>,
    /// Additional source remarks when present.
    pub remarks: Option<Box<str>>,
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
    /// The `type` and `icaoClass` properties use numeric values from the
    /// [OpenAIP airspace schema](https://api.core.openaip.net/api/schemas/response/airspace/airspace-schema.json).
    pub fn to_geojson(&self) -> serde_json::Value {
        let mut properties = json!({
            "type": self.type_code.openaip_code(),
            "icaoClass": self.class.openaip_code(),
            "lowerLimit": self.lower_limit.to_openaip_limit(),
            "upperLimit": self.upper_limit.to_openaip_limit(),
        });
        if let Some(name) = self.name.as_deref() {
            properties["name"] = json!(name);
        }
        if let Some(activity) = self.activity {
            properties["activity"] = json!(activity.openaip_code());
        }
        match self.country_codes.as_slice() {
            [] => {}
            [country] => properties["country"] = json!(country),
            countries => properties["country"] = json!(countries),
        }
        for (property, value) in [
            ("onDemand", self.on_demand),
            ("onRequest", self.on_request),
            ("byNotam", self.by_notam),
            ("specialAgreement", self.special_agreement),
            ("requestCompliance", self.request_compliance),
        ] {
            if let Some(value) = value {
                properties[property] = json!(value);
            }
        }
        if let Some(lower_limit_min) = self.lower_limit_min {
            properties["lowerLimitMin"] = lower_limit_min.to_openaip_limit();
        }
        if let Some(upper_limit_max) = self.upper_limit_max {
            properties["upperLimitMax"] = upper_limit_max.to_openaip_limit();
        }
        if !self.frequencies.is_empty() {
            properties["frequencies"] = json!(
                self.frequencies
                    .iter()
                    .map(|frequency| {
                        let mut value = json!({
                            "value": frequency.value.to_string(),
                            "unit": frequency.unit.openaip_code(),
                        });
                        if let Some(name) = frequency.name.as_deref() {
                            value["name"] = json!(name);
                        }
                        if let Some(primary) = frequency.primary {
                            value["primary"] = json!(primary);
                        }
                        if let Some(remarks) = frequency.remarks.as_deref() {
                            value["remarks"] = json!(remarks);
                        }
                        value
                    })
                    .collect::<Vec<_>>()
            );
        }
        if !self.transponder_settings.is_empty() {
            properties["transponderSettings"] = json!(
                self.transponder_settings
                    .iter()
                    .map(|setting| {
                        let mut value = json!({
                            "code": setting.code.to_string(),
                            "primary": setting.primary,
                        });
                        if let Some(remarks) = setting.remarks.as_deref() {
                            value["remarks"] = json!(remarks);
                        }
                        value
                    })
                    .collect::<Vec<_>>()
            );
        }
        if let Some(hours_of_operation) = self.hours_of_operation.as_ref() {
            properties["hoursOfOperation"] = operating_hours_to_openaip_json(hours_of_operation);
        }
        if let Some(active_from) = self.active_from {
            properties["activeFrom"] = json!(format_openaip_datetime(active_from));
        }
        if let Some(active_until) = self.active_until {
            properties["activeUntil"] = json!(format_openaip_datetime(active_until));
        }

        json!({
            "type": "Feature",
            "id": self.id.0,
            "properties": properties,
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
    #[error("the airspace frequency is invalid")]
    InvalidFrequency,
    #[error("the airspace transponder code is invalid")]
    InvalidTransponderCode,
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
