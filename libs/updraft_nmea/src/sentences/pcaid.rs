//! `$PCAID` — Cambridge Aero Instrument Data: barometric altitude and
//! engine-noise level.

use updraft_units::Length;

use crate::error::ParseError;
use crate::fields::Fields;
use crate::scalars;

/// A parsed `$PCAID` sentence.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Pcaid {
    /// Whether the last point was logged (`L`) or not (`N`).
    pub logged: Option<bool>,
    /// Barometric (pressure) altitude.
    pub pressure_altitude: Option<Length>,
    /// Engine-noise level, used for engine-run detection.
    pub engine_noise_level: Option<u32>,
}

pub(crate) fn parse(fields: Fields<'_>) -> Result<Pcaid, ParseError> {
    let fields: Vec<&str> = fields.collect();
    let get = |index: usize| fields.get(index).copied().unwrap_or("");

    let logged = match get(0) {
        "L" => Some(true),
        "N" => Some(false),
        "" => None,
        _ => return Err(ParseError::InvalidField),
    };
    // Field 4 (log flags) is left unparsed, matching XCSoar.
    Ok(Pcaid {
        logged,
        pressure_altitude: scalars::opt_f64(get(1))?.map(Length::from_meters),
        engine_noise_level: scalars::opt(get(2), scalars::u32)?,
    })
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;

    use super::*;
    use crate::{ParseResult, parse};

    fn pcaid(line: &str) -> Pcaid {
        match parse(line).unwrap() {
            ParseResult::Pcaid(pcaid) => pcaid,
            other => panic!("expected PCAID, got {other:?}"),
        }
    }

    #[test]
    fn not_logged_with_altitude() {
        let pcaid = pcaid("$PCAID,N,500,0,0*24");
        assert_eq!(pcaid.logged, Some(false));
        assert_relative_eq!(pcaid.pressure_altitude.unwrap().as_meters(), 500.0);
        assert_eq!(pcaid.engine_noise_level, Some(0));
    }

    #[test]
    fn logged_with_engine_noise() {
        let pcaid = pcaid("$PCAID,L,1234,50,0*22");
        assert_eq!(pcaid.logged, Some(true));
        assert_eq!(pcaid.engine_noise_level, Some(50));
    }
}
