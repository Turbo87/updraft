//! Positional field extraction shared by the sentence parsers. Every
//! helper treats an empty or unparseable field as absent, so a malformed
//! optional field never fails a whole sentence.

use std::str::FromStr;
use updraft_geo::LatLon;

/// The raw bytes of the `index`-th field, or `None` if it is absent or empty.
pub fn field<'a>(fields: &[&'a [u8]], index: usize) -> Option<&'a [u8]> {
    fields.get(index).copied().filter(|field| !field.is_empty())
}

/// Parses bytes as `T`, or `None` if they are not valid UTF-8 or do not
/// parse. NMEA numbers are ASCII, so a non-UTF-8 field simply reads as absent.
pub fn parse_from_utf8<T: FromStr>(bytes: &[u8]) -> Option<T> {
    std::str::from_utf8(bytes).ok()?.parse().ok()
}

/// The `index`-th field parsed as `T`, or `None` if it is absent, empty, or
/// does not parse.
pub fn parsed_field<T: FromStr>(fields: &[&[u8]], index: usize) -> Option<T> {
    parse_from_utf8(field(fields, index)?)
}

/// A finite floating-point field. `nan`/`inf` parse as `f64` but are treated
/// as absent so a non-finite value never reaches downstream calculations.
pub fn f64_field(fields: &[&[u8]], index: usize) -> Option<f64> {
    parsed_field::<f64>(fields, index).filter(|value| value.is_finite())
}

/// A latitude/longitude pair from four consecutive NMEA fields
/// (`ddmm.mmmm`, hemisphere, `dddmm.mmmm`, hemisphere).
pub fn lat_lon(
    fields: &[&[u8]],
    latitude: usize,
    latitude_hemisphere: usize,
    longitude: usize,
    longitude_hemisphere: usize,
) -> Option<LatLon> {
    let latitude = coordinate(
        field(fields, latitude)?,
        field(fields, latitude_hemisphere)?,
    )?;
    let longitude = coordinate(
        field(fields, longitude)?,
        field(fields, longitude_hemisphere)?,
    )?;
    Some(LatLon::from_degrees(latitude, longitude))
}

/// Converts an NMEA `[d]ddmm.mmmm` magnitude plus a hemisphere letter into
/// signed decimal degrees.
fn coordinate(value: &[u8], hemisphere: &[u8]) -> Option<f64> {
    let value: f64 = parse_from_utf8(value)?;
    // A `ddmm.mmmm` magnitude is non-negative and the hemisphere carries the
    // sign. A negative or non-finite value is corrupt, not a real position.
    if !value.is_finite() || value < 0.0 {
        return None;
    }
    let degrees = (value / 100.0).trunc();
    let minutes = value - degrees * 100.0;
    let magnitude = degrees + minutes / 60.0;
    match hemisphere {
        b"N" | b"E" => Some(magnitude),
        b"S" | b"W" => Some(-magnitude),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_abs_diff_eq;
    use claims::{assert_none, assert_some_eq};

    #[test]
    fn empty_and_missing_fields_are_absent() {
        let fields: [&[u8]; 2] = [b"", b"12"];
        assert_none!(field(&fields, 0));
        assert_some_eq!(field(&fields, 1), b"12".as_slice());
        assert_none!(field(&fields, 2));
        assert_none!(parsed_field::<u8>(&fields, 0));
        assert_some_eq!(parsed_field::<u8>(&fields, 1), 12);
    }

    #[test]
    fn non_finite_numbers_are_absent() {
        assert_none!(f64_field(&[b"nan".as_slice()], 0));
        assert_none!(f64_field(&[b"inf".as_slice()], 0));
        assert_none!(f64_field(&[b"-inf".as_slice()], 0));
        assert_some_eq!(f64_field(&[b"1.5".as_slice()], 0), 1.5);
    }

    #[test]
    fn converts_northeast_coordinates() {
        // 48°57.88170' N, 007°05.83929' E.
        let fields: [&[u8]; 4] = [b"4857.88170", b"N", b"00705.83929", b"E"];
        let position = lat_lon(&fields, 0, 1, 2, 3).unwrap();
        assert_abs_diff_eq!(
            position,
            LatLon::from_degrees(48.964_695, 7.097_321_5),
            epsilon = 1e-9
        );
    }

    #[test]
    fn applies_southern_and_western_signs() {
        let fields: [&[u8]; 4] = [b"4857.88170", b"S", b"00705.83929", b"W"];
        let position = lat_lon(&fields, 0, 1, 2, 3).unwrap();
        assert_abs_diff_eq!(
            position,
            LatLon::from_degrees(-48.964_695, -7.097_321_5),
            epsilon = 1e-9
        );
    }

    #[test]
    fn rejects_an_unknown_hemisphere() {
        let fields: [&[u8]; 4] = [b"4857.88170", b"X", b"00705.83929", b"E"];
        assert_none!(lat_lon(&fields, 0, 1, 2, 3));
    }

    #[test]
    fn rejects_a_negative_or_non_finite_magnitude() {
        // A signed magnitude would silently cancel the hemisphere's sign, so
        // treat it as corrupt instead.
        assert_none!(coordinate(b"-4857.88170", b"S"));
        assert_none!(coordinate(b"inf", b"N"));
    }
}
