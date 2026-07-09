//! Garmin proprietary sentences.

use crate::field::{f64_field, field, parsed_field};
use updraft_units::Length;

/// Garmin barometric altitude (`$PGRMZ`). The altitude unit is taken from
/// the second field (`f` for feet, `m` for meters), defaulting to feet.
#[derive(Clone, Debug, PartialEq)]
pub struct Pgrmz {
    pub altitude: Option<Length>,
    pub fix_dimension: PgrmzFixDimension,
}

impl Pgrmz {
    pub fn parse(fields: &[&[u8]]) -> Self {
        let altitude = f64_field(fields, 0).map(|value| match field(fields, 1) {
            Some(b"m") | Some(b"M") => Length::from_meters(value),
            _ => Length::from_feet(value),
        });
        Self {
            altitude,
            fix_dimension: PgrmzFixDimension::from_field(parsed_field(fields, 2)),
        }
    }
}

/// The fix dimensionality reported in the third `$PGRMZ` field.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PgrmzFixDimension {
    NoFix,
    TwoDimensional,
    ThreeDimensional,
    Other(u8),
}

impl PgrmzFixDimension {
    fn from_field(value: Option<u8>) -> Self {
        match value {
            None | Some(1) => Self::NoFix,
            Some(2) => Self::TwoDimensional,
            Some(3) => Self::ThreeDimensional,
            Some(other) => Self::Other(other),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use claims::assert_some_eq;

    #[test]
    fn reads_altitude_in_feet_by_default() {
        let fields: [&[u8]; 3] = [b"4395", b"f", b"3"];
        let pgrmz = Pgrmz::parse(&fields);
        assert_some_eq!(pgrmz.altitude, Length::from_feet(4395.0));
    }

    #[test]
    fn defaults_to_feet_when_the_unit_is_absent() {
        let fields: [&[u8]; 1] = [b"4395"];
        let pgrmz = Pgrmz::parse(&fields);
        assert_some_eq!(pgrmz.altitude, Length::from_feet(4395.0));
    }

    #[test]
    fn reads_altitude_in_meters() {
        for unit in [b"m".as_slice(), b"M".as_slice()] {
            let fields: [&[u8]; 3] = [b"1340", unit, b"3"];
            let pgrmz = Pgrmz::parse(&fields);
            assert_some_eq!(pgrmz.altitude, Length::from_meters(1340.0));
        }
    }

    #[test]
    fn maps_fix_dimension() {
        assert_eq!(
            PgrmzFixDimension::from_field(None),
            PgrmzFixDimension::NoFix
        );
        assert_eq!(
            PgrmzFixDimension::from_field(Some(1)),
            PgrmzFixDimension::NoFix
        );
        assert_eq!(
            PgrmzFixDimension::from_field(Some(2)),
            PgrmzFixDimension::TwoDimensional
        );
        assert_eq!(
            PgrmzFixDimension::from_field(Some(3)),
            PgrmzFixDimension::ThreeDimensional
        );
        assert_eq!(
            PgrmzFixDimension::from_field(Some(9)),
            PgrmzFixDimension::Other(9)
        );
    }
}
