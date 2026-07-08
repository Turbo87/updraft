//! Small field-parsing helpers shared by the sentence parsers.
//!
//! All indexing goes through [`str::get`] rather than slicing so that a
//! multi-byte field (which arbitrary input can contain) yields a
//! [`ParseError`] instead of panicking on a char boundary.

use updraft_geo::LatLon;
use updraft_units::Angle;

use crate::error::ParseError;
use crate::fields::Fields;

/// Apply `parse` to a field, treating an empty field as `None`.
pub(crate) fn opt<T>(
    field: &str,
    parse: impl FnOnce(&str) -> Result<T, ParseError>,
) -> Result<Option<T>, ParseError> {
    if field.is_empty() {
        Ok(None)
    } else {
        parse(field).map(Some)
    }
}

pub(crate) fn f64(field: &str) -> Result<f64, ParseError> {
    field.parse().map_err(|_| ParseError::InvalidNumber)
}

pub(crate) fn u8(field: &str) -> Result<u8, ParseError> {
    field.parse().map_err(|_| ParseError::InvalidNumber)
}

pub(crate) fn opt_f64(field: &str) -> Result<Option<f64>, ParseError> {
    opt(field, f64)
}

pub(crate) fn opt_u8(field: &str) -> Result<Option<u8>, ParseError> {
    opt(field, u8)
}

/// Parse one `ddmm.mmmm`/`dddmm.mmmm` magnitude plus its hemisphere letter
/// into a signed [`Angle`]. `degree_digits` is 2 for latitude, 3 for
/// longitude.
pub(crate) fn coordinate(
    value: &str,
    hemisphere: &str,
    degree_digits: usize,
) -> Result<Angle, ParseError> {
    let degrees = value
        .get(..degree_digits)
        .ok_or(ParseError::InvalidField)?
        .parse::<f64>()
        .map_err(|_| ParseError::InvalidNumber)?;
    let minutes = value
        .get(degree_digits..)
        .ok_or(ParseError::InvalidField)?
        .parse::<f64>()
        .map_err(|_| ParseError::InvalidNumber)?;
    let sign = match hemisphere {
        "N" | "E" => 1.0,
        "S" | "W" => -1.0,
        _ => return Err(ParseError::InvalidField),
    };
    Ok(Angle::from_degrees(sign * (degrees + minutes / 60.0)))
}

/// Consume the four consecutive `lat, N/S, lon, E/W` fields that both GGA
/// and RMC carry. Yields `None` when the latitude or longitude field is
/// empty (no fix).
pub(crate) fn position(fields: &mut Fields<'_>) -> Result<Option<LatLon>, ParseError> {
    let latitude = fields.next_required()?;
    let north_south = fields.next_required()?;
    let longitude = fields.next_required()?;
    let east_west = fields.next_required()?;
    if latitude.is_empty() || longitude.is_empty() {
        return Ok(None);
    }
    Ok(Some(LatLon::new(
        coordinate(latitude, north_south, 2)?,
        coordinate(longitude, east_west, 3)?,
    )))
}
