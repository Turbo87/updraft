//! Garmin proprietary sentences.

use crate::encode::{EncodeError, SentenceEncoder};
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

impl TryFrom<&Pgrmz> for Vec<u8> {
    type Error = EncodeError;

    fn try_from(pgrmz: &Pgrmz) -> Result<Self, Self::Error> {
        let mut sentence = SentenceEncoder::new("PGRMZ");
        match pgrmz.altitude {
            Some(altitude) => {
                sentence.field(&altitude.as_feet().to_string());
                sentence.field("f");
            }
            None => {
                sentence.field("");
                sentence.field("");
            }
        }
        sentence.field(&pgrmz.fix_dimension.to_nmea_field());
        Ok(sentence.finish())
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

    fn to_nmea_field(self) -> String {
        match self {
            Self::NoFix => "1".to_owned(),
            Self::TwoDimensional => "2".to_owned(),
            Self::ThreeDimensional => "3".to_owned(),
            Self::Other(value) => value.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Message, Step, parse};
    use claims::{assert_ok, assert_some_eq};

    #[test]
    fn encodes_pgrmz_sentence_in_default_feet() {
        let pgrmz = Pgrmz {
            altitude: Some(Length::from_feet(4395.0)),
            fix_dimension: PgrmzFixDimension::ThreeDimensional,
        };

        insta::assert_snapshot!(encode_pgrmz_sentence(&pgrmz));
    }

    #[test]
    fn encodes_every_pgrmz_fix_dimension() {
        let sentences = [
            PgrmzFixDimension::NoFix,
            PgrmzFixDimension::TwoDimensional,
            PgrmzFixDimension::ThreeDimensional,
            PgrmzFixDimension::Other(9),
        ]
        .map(|fix_dimension| {
            encode_pgrmz_sentence(&Pgrmz {
                altitude: Some(Length::from_feet(4395.0)),
                fix_dimension,
            })
        })
        .concat();

        insta::assert_snapshot!(sentences);
    }

    #[test]
    fn parses_encoded_pgrmz_sentence() {
        let expected = Pgrmz {
            altitude: Some(Length::from_feet(4395.0)),
            fix_dimension: PgrmzFixDimension::ThreeDimensional,
        };
        let sentence = assert_ok!(Vec::<u8>::try_from(&expected));
        let mut input = sentence.as_slice();

        let actual = match parse(&mut input) {
            Step::Frame(Message::Pgrmz(pgrmz)) => pgrmz,
            step => panic!("expected encoded PGRMZ frame, got {step:?}"),
        };

        assert_eq!(actual, expected);
    }

    fn encode_pgrmz_sentence(pgrmz: &Pgrmz) -> String {
        let sentence = assert_ok!(Vec::<u8>::try_from(pgrmz));
        let sentence = assert_ok!(String::from_utf8(sentence));
        assert!(sentence.ends_with("\r\n"));
        sentence
    }

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
