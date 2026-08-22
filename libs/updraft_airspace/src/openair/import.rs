use super::geometry::normalize_geometry;
use crate::{
    Airspace, AirspaceAltitude, AirspaceClass, AirspaceDataset, AirspaceFrequency,
    AirspaceFrequencyUnit, AirspaceFrequencyValue, AirspaceId, AirspaceImportError,
    AirspaceParseError, AirspaceTransponderCode, AirspaceTransponderSetting, AirspaceType,
};
use ::openair::{
    Airspace as ParsedAirspace, AirspaceType as ParsedAirspaceType, Class as ParsedClass,
};
use std::io::Cursor;
use updraft_units::{Length, MslAltitude, PressureAltitude};

impl AirspaceDataset {
    /// Parses complete OpenAir bytes and converts all shapes to polygons.
    ///
    /// # Errors
    ///
    /// Returns an error if the source or one canonical airspace is invalid.
    pub fn from_openair(bytes: &[u8]) -> Result<Self, AirspaceImportError> {
        let parsed = ::openair::parse(Cursor::new(bytes))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|source| AirspaceImportError::Parse {
                airspace_id: None,
                kind: AirspaceParseError::SourceParser(source),
            })?;

        let airspaces = parsed
            .into_iter()
            .enumerate()
            .map(|(index, airspace)| {
                let id = u32::try_from(index).map(AirspaceId).map_err(|_| {
                    AirspaceImportError::Parse {
                        airspace_id: None,
                        kind: AirspaceParseError::TooManyAirspaces,
                    }
                })?;
                normalize_airspace(id, airspace)
            })
            .collect::<Result<_, _>>()?;

        Ok(Self::from_airspaces(airspaces))
    }
}

/// Converts one parsed airspace to canonical form and assigns its dataset ID.
fn normalize_airspace(
    id: AirspaceId,
    mut parsed: ParsedAirspace,
) -> Result<Airspace, AirspaceImportError> {
    let (class, type_code) = normalize_classification(&mut parsed)
        .map_err(|kind| AirspaceImportError::parse(id, kind))?;
    let lower_limit = AirspaceAltitude::try_from(parsed.lower_bound)
        .map_err(|kind| AirspaceImportError::parse(id, kind))?;
    let upper_limit = AirspaceAltitude::try_from(parsed.upper_bound)
        .map_err(|kind| AirspaceImportError::parse(id, kind))?;
    let polygon =
        normalize_geometry(parsed.geom).map_err(|kind| AirspaceImportError::geometry(id, kind))?;
    let frequencies = normalize_frequency(parsed.frequency, parsed.call_sign)
        .map_err(|kind| AirspaceImportError::parse(id, kind))?;
    let transponder_settings = normalize_transponder_code(parsed.transponder_code)
        .map_err(|kind| AirspaceImportError::parse(id, kind))?;

    Ok(Airspace {
        id,
        name: parsed.name.map(String::into_boxed_str),
        class,
        type_code,
        activity: None,
        on_demand: None,
        on_request: None,
        by_notam: None,
        special_agreement: None,
        request_compliance: None,
        country_codes: Vec::new(),
        frequencies,
        transponder_settings,
        hours_of_operation: None,
        active_from: None,
        active_until: None,
        remarks: None,
        lower_limit,
        lower_limit_min: None,
        upper_limit,
        upper_limit_max: None,
        polygon,
    })
}

/// Converts one optional OpenAir transponder code to canonical form.
fn normalize_transponder_code(
    code: Option<u16>,
) -> Result<Vec<AirspaceTransponderSetting>, AirspaceParseError> {
    let Some(code) = code else {
        return Ok(Vec::new());
    };
    let code = AirspaceTransponderCode::from_octal_digits(code)
        .ok_or(AirspaceParseError::InvalidTransponderCode)?;

    Ok(vec![AirspaceTransponderSetting {
        code,
        primary: true,
        remarks: None,
    }])
}

/// Converts one optional OpenAir frequency and call sign to canonical form.
fn normalize_frequency(
    frequency: Option<String>,
    call_sign: Option<String>,
) -> Result<Vec<AirspaceFrequency>, AirspaceParseError> {
    let Some(frequency) = frequency else {
        return Ok(Vec::new());
    };
    let value = AirspaceFrequencyValue::from_megahertz(&frequency)
        .ok_or(AirspaceParseError::InvalidFrequency)?;

    Ok(vec![AirspaceFrequency {
        value,
        unit: AirspaceFrequencyUnit::Megahertz,
        name: call_sign.map(String::into_boxed_str),
        primary: Some(true),
        remarks: None,
    }])
}

/// Converts a parsed class and optional type to a canonical classification.
fn normalize_classification(
    parsed: &mut ParsedAirspace,
) -> Result<(AirspaceClass, AirspaceType), AirspaceParseError> {
    parsed
        .normalize_legacy_class()
        .map_err(|_| AirspaceParseError::ConflictingClassification)?;
    normalize_legacy_gsec_class(parsed)?;

    let class = match parsed.class {
        ParsedClass::A => AirspaceClass::A,
        ParsedClass::B => AirspaceClass::B,
        ParsedClass::C => AirspaceClass::C,
        ParsedClass::D => AirspaceClass::D,
        ParsedClass::E => AirspaceClass::E,
        ParsedClass::F => AirspaceClass::F,
        ParsedClass::G => AirspaceClass::G,
        ParsedClass::Unclassified => AirspaceClass::Unclassified,
        ParsedClass::Unknown(_) => return Err(AirspaceParseError::UnsupportedClass),
    };
    let type_code = parsed
        .type_
        .as_ref()
        .map(airspace_type_from_openair)
        .unwrap_or(AirspaceType::Other);
    Ok((class, type_code))
}

/// Converts the nonstandard `AC GSEC` form used by real-world source files.
fn normalize_legacy_gsec_class(parsed: &mut ParsedAirspace) -> Result<(), AirspaceParseError> {
    let ParsedClass::Unknown(class) = &parsed.class else {
        return Ok(());
    };
    if class.as_ref() != "GSEC" {
        return Ok(());
    }

    let gliding_sector = ParsedAirspaceType::GlidingSector;
    if parsed
        .type_
        .as_ref()
        .is_some_and(|existing_type| existing_type != &gliding_sector)
    {
        return Err(AirspaceParseError::ConflictingClassification);
    }

    parsed.class = ParsedClass::Unclassified;
    parsed.type_ = Some(gliding_sector);
    Ok(())
}

/// Maps an OpenAir v2 airspace type to its OpenAIP airspace type.
fn airspace_type_from_openair(parsed_type: &ParsedAirspaceType) -> AirspaceType {
    match parsed_type {
        ParsedAirspaceType::RemoteCommunicationSector => AirspaceType::AccSector,
        ParsedAirspaceType::AirDefenceIdentZone => AirspaceType::AirDefenseIdentificationZone,
        ParsedAirspaceType::AlertArea => AirspaceType::AlertArea,
        ParsedAirspaceType::AerialSportingOrRecreationalActivity => {
            AirspaceType::AerialSportingOrRecreationalActivity
        }
        ParsedAirspaceType::AerodromeTrafficZone => AirspaceType::AirportTrafficZone,
        ParsedAirspaceType::Airway => AirspaceType::Airway,
        ParsedAirspaceType::ControlArea => AirspaceType::ControlArea,
        ParsedAirspaceType::ControlZone => AirspaceType::ControlledTowerRegion,
        ParsedAirspaceType::FlightInformationRegion => AirspaceType::FlightInformationRegion,
        ParsedAirspaceType::FlightInformationServiceSector => AirspaceType::FisSector,
        ParsedAirspaceType::GlidingSector => AirspaceType::GlidingSector,
        ParsedAirspaceType::HelicopterTrafficZone => AirspaceType::HelicopterTrafficZone,
        ParsedAirspaceType::LowerTrafficArea => AirspaceType::LowerTrafficArea,
        ParsedAirspaceType::MilitaryAerodromeTrafficZone => {
            AirspaceType::MilitaryAirportTrafficZone
        }
        ParsedAirspaceType::MilitaryTrainingArea => AirspaceType::MilitaryTrainingArea,
        ParsedAirspaceType::MilitaryTrainingRoute => AirspaceType::MilitaryTrainingRoute,
        ParsedAirspaceType::OverflightRestriction => AirspaceType::LowAltitudeOverflightRestriction,
        ParsedAirspaceType::ProhibitedArea => AirspaceType::Prohibited,
        ParsedAirspaceType::DangerArea => AirspaceType::Danger,
        ParsedAirspaceType::RestrictedArea => AirspaceType::Restricted,
        ParsedAirspaceType::RadioMandatoryZone => AirspaceType::RadioMandatoryZone,
        ParsedAirspaceType::TrafficInformationArea => AirspaceType::TrafficInformationArea,
        ParsedAirspaceType::TrafficInformationZone => AirspaceType::TrafficInformationZone,
        ParsedAirspaceType::TerminalManoeuvringArea => AirspaceType::TerminalManeuveringArea,
        ParsedAirspaceType::TransponderMandatoryZone => AirspaceType::TransponderMandatoryZone,
        ParsedAirspaceType::TemporaryReservedArea => AirspaceType::TemporaryReservedArea,
        ParsedAirspaceType::TemporaryReservedOrSegregatedAreaFeedingRoute => {
            AirspaceType::TsaOrTraFeedingRoute
        }
        ParsedAirspaceType::TemporarySegregatedArea => AirspaceType::TemporarySegregatedArea,
        ParsedAirspaceType::UpperFlightInformationRegion => {
            AirspaceType::UpperFlightInformationRegion
        }
        ParsedAirspaceType::UpperTrafficArea => AirspaceType::UpperTrafficArea,
        ParsedAirspaceType::VisualFlightRulesSector => AirspaceType::VfrSector,
        ParsedAirspaceType::WarningArea => AirspaceType::WarningArea,
        ParsedAirspaceType::Custom
        | ParsedAirspaceType::NotamAffectedArea
        | ParsedAirspaceType::NoType
        | ParsedAirspaceType::TemporaryFlightRestriction
        | ParsedAirspaceType::TransponderRecommendedZone
        | ParsedAirspaceType::VisualFlightRulesRoute
        | ParsedAirspaceType::Unknown(_) => AirspaceType::Other,
    }
}

impl TryFrom<::openair::Altitude> for AirspaceAltitude {
    type Error = AirspaceParseError;

    fn try_from(value: ::openair::Altitude) -> Result<Self, Self::Error> {
        match value {
            ::openair::Altitude::Gnd => Ok(AirspaceAltitude::Ground),
            ::openair::Altitude::FeetAmsl(feet) => Ok(AirspaceAltitude::Msl(MslAltitude::new(
                Length::from_feet(f64::from(feet)),
            ))),
            ::openair::Altitude::FeetAgl(feet) => {
                Ok(AirspaceAltitude::Agl(Length::from_feet(f64::from(feet))))
            }
            ::openair::Altitude::FlightLevel(level) => Ok(AirspaceAltitude::FlightLevel(
                PressureAltitude::new(Length::from_feet(f64::from(level) * 100.)),
            )),
            ::openair::Altitude::Unlimited => Ok(AirspaceAltitude::Unlimited),
            ::openair::Altitude::Other(_) => Err(AirspaceParseError::UnsupportedAltitude),
        }
    }
}
