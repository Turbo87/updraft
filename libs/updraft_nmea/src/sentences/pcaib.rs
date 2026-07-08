//! `$PCAIB` — Cambridge proprietary destination-navpoint echo.
//!
//! The instrument reports the elevation and attribute word of its current
//! destination navpoint. `XCSoar` ignores it; the fields are modelled here
//! for completeness.

use updraft_units::Length;

use crate::error::ParseError;
use crate::fields::Fields;
use crate::scalars;

/// A parsed `$PCAIB` sentence.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Pcaib {
    /// Destination navpoint elevation.
    pub destination_elevation: Option<Length>,
    /// Destination navpoint attribute word (a device-specific bitfield).
    pub destination_attribute: Option<u32>,
}

pub(crate) fn parse(fields: Fields<'_>) -> Result<Pcaib, ParseError> {
    let fields: Vec<&str> = fields.collect();
    let get = |index: usize| fields.get(index).copied().unwrap_or("");
    Ok(Pcaib {
        destination_elevation: scalars::opt_f64(get(0))?.map(Length::from_meters),
        destination_attribute: scalars::opt(get(1), scalars::u32)?,
    })
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;

    use crate::{ParseResult, parse};

    #[test]
    fn elevation_and_attribute() {
        let ParseResult::Pcaib(pcaib) = parse("$PCAIB,01234,00056*5E").unwrap() else {
            panic!("expected PCAIB");
        };
        assert_relative_eq!(pcaib.destination_elevation.unwrap().as_meters(), 1234.0);
        assert_eq!(pcaib.destination_attribute, Some(56));
    }
}
