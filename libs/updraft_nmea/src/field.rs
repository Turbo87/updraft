//! Field extraction shared by the sentence parsers: a cursor over the
//! comma-separated fields of a sentence body, consumed in wire order.
//! Every helper treats an empty or unparseable field as absent, so a
//! malformed optional field never fails a whole sentence.

use std::str::FromStr;
use updraft_geo::LatLon;

/// The comma-separated fields of a sentence body, consumed left to right.
///
/// Fields are split off lazily as the parsers ask for them, so a sentence
/// is scanned exactly once and nothing is buffered. The typed helpers
/// (`bytes`, `f64`, ...) each consume one field.
#[derive(Clone, Debug)]
pub struct FieldsIter<'a> {
    /// The not-yet-consumed tail of the argument list, or `None` once the
    /// last field has been yielded.
    rest: Option<&'a [u8]>,
}

impl<'a> FieldsIter<'a> {
    pub fn new(args: &'a [u8]) -> Self {
        Self { rest: Some(args) }
    }

    /// The next field, with an empty or missing field read as absent.
    pub fn bytes(&mut self) -> Option<&'a [u8]> {
        self.next().filter(|field| !field.is_empty())
    }

    /// The next field as owned text, with invalid UTF-8 replaced by the
    /// Unicode replacement character. NMEA carries ASCII, so this only
    /// matters for a corrupt field or sentence.
    pub fn text(&mut self) -> Option<Box<str>> {
        self.bytes().map(text)
    }

    /// The next field parsed as `T`.
    pub fn parsed<T: FromStr>(&mut self) -> Option<T> {
        parse_from_utf8(self.bytes()?)
    }

    /// The next field as a finite floating-point value. `nan`/`inf` parse
    /// as `f64` but are treated as absent so a non-finite value never
    /// reaches downstream calculations.
    pub fn f64(&mut self) -> Option<f64> {
        self.parsed::<f64>().filter(|value| value.is_finite())
    }

    /// A latitude/longitude pair from the next four fields (`ddmm.mmmm`,
    /// hemisphere, `dddmm.mmmm`, hemisphere). All four fields are consumed
    /// even when some are absent or corrupt.
    pub fn lat_lon(&mut self) -> Option<LatLon> {
        let latitude = (self.bytes(), self.bytes());
        let longitude = (self.bytes(), self.bytes());
        let latitude = coordinate(latitude.0?, latitude.1?)?;
        let longitude = coordinate(longitude.0?, longitude.1?)?;
        Some(LatLon::from_degrees(latitude, longitude))
    }
}

/// Yields every field's raw bytes, including empty ones. An empty argument
/// list is a single empty field, like `split` on the separator.
impl<'a> Iterator for FieldsIter<'a> {
    type Item = &'a [u8];

    fn next(&mut self) -> Option<&'a [u8]> {
        let rest = self.rest?;
        match rest.iter().position(|&byte| byte == b',') {
            Some(comma) => {
                self.rest = Some(&rest[comma + 1..]);
                Some(&rest[..comma])
            }
            None => {
                self.rest = None;
                Some(rest)
            }
        }
    }
}

/// The bytes as owned text, with invalid UTF-8 replaced by the Unicode
/// replacement character.
pub fn text(bytes: &[u8]) -> Box<str> {
    String::from_utf8_lossy(bytes).into_owned().into_boxed_str()
}

/// Parses bytes as `T`, or `None` if they are not valid UTF-8 or do not
/// parse. NMEA numbers are ASCII, so a non-UTF-8 field simply reads as absent.
pub fn parse_from_utf8<T: FromStr>(bytes: &[u8]) -> Option<T> {
    std::str::from_utf8(bytes).ok()?.parse().ok()
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
        let mut fields = FieldsIter::new(b",12");
        assert_none!(fields.bytes());
        assert_some_eq!(fields.bytes(), b"12".as_slice());
        assert_none!(fields.bytes());

        let mut fields = FieldsIter::new(b",12");
        assert_none!(fields.parsed::<u8>());
        assert_some_eq!(fields.parsed::<u8>(), 12);
        assert_none!(fields.parsed::<u8>());
    }

    #[test]
    fn yields_fields_like_a_split() {
        let fields: Vec<&[u8]> = FieldsIter::new(b"a,,b").collect();
        assert_eq!(fields, [b"a".as_slice(), b"", b"b"]);

        // An empty argument list is a single empty field.
        let fields: Vec<&[u8]> = FieldsIter::new(b"").collect();
        assert_eq!(fields, [b"".as_slice()]);
    }

    proptest::proptest! {
        /// The cursor must yield exactly what a naive `split` yields, on
        /// arbitrary input.
        #[test]
        fn fields_match_naive_split(
            args in proptest::collection::vec(
                proptest::prop_oneof![
                    proptest::prelude::Just(b','),
                    proptest::prelude::any::<u8>(),
                ],
                0..96,
            ),
        ) {
            let expected: Vec<&[u8]> = args.split(|&byte| byte == b',').collect();
            let fields: Vec<&[u8]> = FieldsIter::new(&args).collect();
            proptest::prop_assert_eq!(fields, expected);
        }
    }

    #[test]
    fn non_finite_numbers_are_absent() {
        assert_none!(FieldsIter::new(b"nan").f64());
        assert_none!(FieldsIter::new(b"inf").f64());
        assert_none!(FieldsIter::new(b"-inf").f64());
        assert_some_eq!(FieldsIter::new(b"1.5").f64(), 1.5);
    }

    #[test]
    fn converts_northeast_coordinates() {
        // 48°57.88170' N, 007°05.83929' E.
        let mut fields = FieldsIter::new(b"4857.88170,N,00705.83929,E");
        let position = fields.lat_lon().unwrap();
        assert_abs_diff_eq!(
            position,
            LatLon::from_degrees(48.964_695, 7.097_321_5),
            epsilon = 1e-9
        );
    }

    #[test]
    fn applies_southern_and_western_signs() {
        let mut fields = FieldsIter::new(b"4857.88170,S,00705.83929,W");
        let position = fields.lat_lon().unwrap();
        assert_abs_diff_eq!(
            position,
            LatLon::from_degrees(-48.964_695, -7.097_321_5),
            epsilon = 1e-9
        );
    }

    #[test]
    fn rejects_an_unknown_hemisphere() {
        let mut fields = FieldsIter::new(b"4857.88170,X,00705.83929,E");
        assert_none!(fields.lat_lon());
    }

    #[test]
    fn lat_lon_consumes_all_four_fields_even_when_corrupt() {
        // The hemisphere is invalid, but the cursor must still stand after
        // the fourth field so later fields keep their positions.
        let mut fields = FieldsIter::new(b"4857.88170,X,00705.83929,E,42");
        assert_none!(fields.lat_lon());
        assert_some_eq!(fields.parsed::<u8>(), 42);
    }

    #[test]
    fn rejects_a_negative_or_non_finite_magnitude() {
        // A signed magnitude would silently cancel the hemisphere's sign, so
        // treat it as corrupt instead.
        assert_none!(coordinate(b"-4857.88170", b"S"));
        assert_none!(coordinate(b"inf", b"N"));
    }
}
