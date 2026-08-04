use super::geometry::normalize_geometry;
use crate::{
    Airspace, AirspaceAltitude, AirspaceClass, AirspaceDataset, AirspaceId, AirspaceImportError,
    AirspaceParseError, AirspaceType,
};
use ::openair::{Airspace as ParsedAirspace, Class as ParsedClass};
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
    parsed: ParsedAirspace,
) -> Result<Airspace, AirspaceImportError> {
    let (class, type_code) = normalize_classification(parsed.class, parsed.type_.as_deref());
    let lower_limit = AirspaceAltitude::try_from(parsed.lower_bound)
        .map_err(|kind| AirspaceImportError::parse(id, kind))?;
    let upper_limit = AirspaceAltitude::try_from(parsed.upper_bound)
        .map_err(|kind| AirspaceImportError::parse(id, kind))?;
    let polygon =
        normalize_geometry(parsed.geom).map_err(|kind| AirspaceImportError::geometry(id, kind))?;

    Ok(Airspace {
        id,
        name: parsed.name.map(String::into_boxed_str),
        class,
        type_code,
        activity: None,
        on_demand: None,
        on_request: None,
        lower_limit,
        lower_limit_min: None,
        upper_limit,
        upper_limit_max: None,
        polygon,
    })
}

/// Converts a parsed class and optional type to a canonical classification.
fn normalize_classification(
    parsed_class: ParsedClass,
    parsed_type: Option<&str>,
) -> (AirspaceClass, AirspaceType) {
    let (class, legacy_type) = match parsed_class {
        ParsedClass::A => (AirspaceClass::A, None),
        ParsedClass::B => (AirspaceClass::B, None),
        ParsedClass::C => (AirspaceClass::C, None),
        ParsedClass::D => (AirspaceClass::D, None),
        ParsedClass::E => (AirspaceClass::E, None),
        ParsedClass::F => (AirspaceClass::F, None),
        ParsedClass::G => (AirspaceClass::G, None),
        ParsedClass::Unclassified => (AirspaceClass::Unclassified, None),
        ParsedClass::Ctr => (
            AirspaceClass::Unclassified,
            Some(AirspaceType::ControlledTowerRegion),
        ),
        ParsedClass::Restricted => (AirspaceClass::Unclassified, Some(AirspaceType::Restricted)),
        ParsedClass::Danger => (AirspaceClass::Unclassified, Some(AirspaceType::Danger)),
        ParsedClass::Prohibited => (AirspaceClass::Unclassified, Some(AirspaceType::Prohibited)),
        ParsedClass::GliderProhibited => (AirspaceClass::Unclassified, Some(AirspaceType::Other)),
        ParsedClass::WaveWindow => (
            AirspaceClass::Unclassified,
            Some(AirspaceType::GlidingSector),
        ),
        ParsedClass::RadioMandatoryZone => (
            AirspaceClass::Unclassified,
            Some(AirspaceType::RadioMandatoryZone),
        ),
        ParsedClass::TransponderMandatoryZone => (
            AirspaceClass::Unclassified,
            Some(AirspaceType::TransponderMandatoryZone),
        ),
    };

    let type_code = parsed_type
        .and_then(airspace_type_from_openair_code)
        .or(legacy_type)
        .unwrap_or(AirspaceType::Other);
    (class, type_code)
}

/// Maps a supported OpenAir type code to its OpenAIP airspace type.
fn airspace_type_from_openair_code(code: &str) -> Option<AirspaceType> {
    match code.trim().to_ascii_uppercase().as_str() {
        "ACCSEC" => Some(AirspaceType::AccSector),
        "ADIZ" => Some(AirspaceType::AirDefenseIdentificationZone),
        "ALERT" => Some(AirspaceType::AlertArea),
        "ASRA" => Some(AirspaceType::AerialSportingOrRecreationalActivity),
        "ATZ" => Some(AirspaceType::AirportTrafficZone),
        "AWY" => Some(AirspaceType::Airway),
        "CTA" => Some(AirspaceType::ControlArea),
        "CTR" => Some(AirspaceType::ControlledTowerRegion),
        "FIR" => Some(AirspaceType::FlightInformationRegion),
        "FIS" => Some(AirspaceType::FisSector),
        "GSEC" => Some(AirspaceType::GlidingSector),
        "HTZ" => Some(AirspaceType::HelicopterTrafficZone),
        "LTA" => Some(AirspaceType::LowerTrafficArea),
        "MATZ" => Some(AirspaceType::MilitaryAirportTrafficZone),
        "MTA" => Some(AirspaceType::MilitaryTrainingArea),
        "MTR" => Some(AirspaceType::MilitaryTrainingRoute),
        "OFR" => Some(AirspaceType::LowAltitudeOverflightRestriction),
        "P" => Some(AirspaceType::Prohibited),
        "Q" => Some(AirspaceType::Danger),
        "R" => Some(AirspaceType::Restricted),
        "RMZ" => Some(AirspaceType::RadioMandatoryZone),
        "TIA" => Some(AirspaceType::TrafficInformationArea),
        "TIZ" => Some(AirspaceType::TrafficInformationZone),
        "TMA" => Some(AirspaceType::TerminalManeuveringArea),
        "TMZ" => Some(AirspaceType::TransponderMandatoryZone),
        "TRA" => Some(AirspaceType::TemporaryReservedArea),
        "TRAFR" => Some(AirspaceType::TsaOrTraFeedingRoute),
        "TSA" => Some(AirspaceType::TemporarySegregatedArea),
        "UIR" => Some(AirspaceType::UpperFlightInformationRegion),
        "UTA" => Some(AirspaceType::UpperTrafficArea),
        "VFRSEC" => Some(AirspaceType::VfrSector),
        "WARNING" => Some(AirspaceType::WarningArea),
        _ => None,
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
