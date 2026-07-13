//! Garmin proprietary sentences.

use crate::field::FieldsIter;
use updraft_units::Length;

/// Garmin barometric altitude (`$PGRMZ`). The altitude unit is taken from
/// the second field (`f` for feet, `m` for meters), defaulting to feet.
#[derive(Clone, Debug, PartialEq)]
pub struct Pgrmz {
    pub altitude: Option<Length>,
    pub fix_dimension: PgrmzFixDimension,
}

impl Pgrmz {
    pub fn parse(mut fields: FieldsIter<'_>) -> Self {
        let value = fields.f64();
        let unit = fields.bytes();
        let altitude = value.map(|value| match unit {
            Some(b"m") | Some(b"M") => Length::from_meters(value),
            _ => Length::from_feet(value),
        });
        Self {
            altitude,
            fix_dimension: fields
                .bytes()
                .map(PgrmzFixDimension::from_field)
                .unwrap_or_default(),
        }
    }
}

/// The fix dimensionality reported in the third `$PGRMZ` field.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum PgrmzFixDimension {
    #[default]
    NoFix,
    TwoDimensional,
    ThreeDimensional,
    Other(u8),
}

impl PgrmzFixDimension {
    fn from_field(field: &[u8]) -> Self {
        match field {
            b"1" => Self::NoFix,
            b"2" => Self::TwoDimensional,
            b"3" => Self::ThreeDimensional,
            field => btoi::btou(field).ok().map(Self::Other).unwrap_or_default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use claims::assert_some_eq;

    #[test]
    fn reads_altitude_in_feet_by_default() {
        let pgrmz = Pgrmz::parse(FieldsIter::new(b"4395,f,3"));
        assert_some_eq!(pgrmz.altitude, Length::from_feet(4395.0));
    }

    #[test]
    fn defaults_to_feet_when_the_unit_is_absent() {
        let pgrmz = Pgrmz::parse(FieldsIter::new(b"4395"));
        assert_some_eq!(pgrmz.altitude, Length::from_feet(4395.0));
    }

    #[test]
    fn reads_altitude_in_meters() {
        for sentence in [b"1340,m,3".as_slice(), b"1340,M,3".as_slice()] {
            let pgrmz = Pgrmz::parse(FieldsIter::new(sentence));
            assert_some_eq!(pgrmz.altitude, Length::from_meters(1340.0));
        }
    }

    #[test]
    fn maps_fix_dimension() {
        assert_eq!(PgrmzFixDimension::default(), PgrmzFixDimension::NoFix);
        assert_eq!(
            PgrmzFixDimension::from_field(b"1"),
            PgrmzFixDimension::NoFix
        );
        assert_eq!(
            PgrmzFixDimension::from_field(b"2"),
            PgrmzFixDimension::TwoDimensional
        );
        assert_eq!(
            PgrmzFixDimension::from_field(b"3"),
            PgrmzFixDimension::ThreeDimensional
        );
        assert_eq!(
            PgrmzFixDimension::from_field(b"9"),
            PgrmzFixDimension::Other(9)
        );
    }
}
