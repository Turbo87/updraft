//! `PGRMZ` — Garmin proprietary altitude sentence.
//!
//! Garmin GPS units report GPS altitude here; in a glider panel a vario or
//! FLARM emitting `PGRMZ` reports **barometric pressure altitude**, which
//! is why this sentence is part of the air-data set rather than the GNSS
//! set.

use updraft_units::Length;

use crate::error::ParseError;
use crate::fields::Fields;
use crate::scalars;
use crate::sentences::gsa::FixType;

/// A parsed `PGRMZ` sentence.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Pgrmz {
    /// The reported altitude (barometric pressure altitude when the source
    /// is a vario or FLARM). The wire unit is normalized away.
    pub altitude: Length,
    /// Dimension of the position fix, if reported (`1`/`2`/`3`).
    pub fix_dimension: Option<FixType>,
}

pub(crate) fn parse(mut fields: Fields<'_>) -> Result<Pgrmz, ParseError> {
    let value = scalars::f64(fields.next_required()?)?;
    let altitude = match fields.next_required()? {
        "f" | "F" => Length::from_feet(value),
        "m" | "M" => Length::from_meters(value),
        _ => return Err(ParseError::InvalidField),
    };
    // The fix-dimension field is sometimes empty or omitted entirely.
    let fix_dimension = scalars::opt(fields.next_field().unwrap_or(""), FixType::parse)?;
    Ok(Pgrmz {
        altitude,
        fix_dimension,
    })
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;

    use super::*;
    use crate::{ParseResult, parse};

    fn pgrmz(line: &str) -> Pgrmz {
        match parse(line).unwrap() {
            ParseResult::Pgrmz(pgrmz) => pgrmz,
            other => panic!("expected PGRMZ, got {other:?}"),
        }
    }

    #[test]
    fn feet_altitude_from_flight_corpus() {
        let pgrmz = pgrmz("$PGRMZ,4395,f,3*20");
        assert_relative_eq!(pgrmz.altitude.as_feet(), 4395.0);
        assert_eq!(pgrmz.fix_dimension, Some(FixType::ThreeDimensional));
    }

    #[test]
    fn meters_unit_is_normalized() {
        let pgrmz = pgrmz("$PGRMZ,1339,m,2*29");
        assert_relative_eq!(pgrmz.altitude.as_meters(), 1339.0);
        assert_eq!(pgrmz.fix_dimension, Some(FixType::TwoDimensional));
    }

    #[test]
    fn missing_fix_dimension() {
        // Empty third field, and the field omitted entirely.
        assert_eq!(pgrmz("$PGRMZ,93,f,*12").fix_dimension, None);
        assert_eq!(pgrmz("$PGRMZ,1000,f*35").fix_dimension, None);
    }

    #[test]
    fn rejects_unknown_unit() {
        assert_eq!(parse("$PGRMZ,100,x,3*04"), Err(ParseError::InvalidField));
    }
}
