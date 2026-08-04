use super::geometry::normalize_geometry;
use crate::airspace::{
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
    let (class, type_code) = normalize_classification(parsed.class, parsed.type_.as_deref())
        .map_err(|kind| AirspaceImportError::parse(id, kind))?;
    let lower_bound = AirspaceAltitude::try_from(parsed.lower_bound)
        .map_err(|kind| AirspaceImportError::parse(id, kind))?;
    let upper_bound = AirspaceAltitude::try_from(parsed.upper_bound)
        .map_err(|kind| AirspaceImportError::parse(id, kind))?;
    let polygon =
        normalize_geometry(parsed.geom).map_err(|kind| AirspaceImportError::geometry(id, kind))?;

    Ok(Airspace {
        id,
        name: parsed.name.map(String::into_boxed_str),
        class,
        type_code,
        lower_bound,
        upper_bound,
        polygon,
    })
}

/// Converts a parsed class and optional type to a canonical classification.
fn normalize_classification(
    parsed_class: ParsedClass,
    parsed_type: Option<&str>,
) -> Result<(Option<AirspaceClass>, Option<AirspaceType>), AirspaceParseError> {
    let (class, legacy_type) = match parsed_class {
        ParsedClass::A => (Some(AirspaceClass::A), None),
        ParsedClass::B => (Some(AirspaceClass::B), None),
        ParsedClass::C => (Some(AirspaceClass::C), None),
        ParsedClass::D => (Some(AirspaceClass::D), None),
        ParsedClass::E => (Some(AirspaceClass::E), None),
        ParsedClass::F => (Some(AirspaceClass::F), None),
        ParsedClass::G => (Some(AirspaceClass::G), None),
        ParsedClass::Unclassified => (Some(AirspaceClass::Unclassified), None),
        ParsedClass::Ctr => (None, Some(AirspaceType::ControlZone)),
        ParsedClass::Restricted => (None, Some(AirspaceType::RestrictedArea)),
        ParsedClass::Danger => (None, Some(AirspaceType::DangerArea)),
        ParsedClass::Prohibited => (None, Some(AirspaceType::ProhibitedArea)),
        ParsedClass::GliderProhibited => (None, Some(AirspaceType::Unknown("GP".into()))),
        ParsedClass::WaveWindow => (None, Some(AirspaceType::GlidingSector)),
        ParsedClass::RadioMandatoryZone => (None, Some(AirspaceType::RadioMandatoryZone)),
        ParsedClass::TransponderMandatoryZone => {
            (None, Some(AirspaceType::TransponderMandatoryZone))
        }
    };

    let normalized_type = parsed_type.map(|value| value.trim().to_ascii_uppercase());
    let type_code = match normalized_type.as_deref() {
        None => legacy_type,
        Some("NONE") => None,
        Some("") => {
            return Err(AirspaceParseError::EmptyTypeCode);
        }
        Some(code) => Some(AirspaceType::from_code(code)),
    };
    if class.is_none() && type_code.is_none() {
        return Err(AirspaceParseError::MissingClassOrType);
    }
    Ok((class, type_code))
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
