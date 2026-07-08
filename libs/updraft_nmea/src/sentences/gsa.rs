//! `GSA` — GNSS DOP and active satellites: fix dimensionality, the
//! satellites used, and the dilution-of-precision values.

use crate::error::ParseError;
use crate::fields::Fields;
use crate::scalars;

/// A parsed `GSA` sentence.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Gsa {
    /// Whether 2D/3D selection is automatic or manual.
    pub selection_mode: SelectionMode,
    /// The fix dimensionality.
    pub fix_type: FixType,
    /// PRNs of the satellites used in the fix (empty slots omitted).
    pub satellite_prns: Vec<u8>,
    /// Positional (3D) dilution of precision.
    pub pdop: Option<f64>,
    /// Horizontal dilution of precision.
    pub hdop: Option<f64>,
    /// Vertical dilution of precision.
    pub vdop: Option<f64>,
}

/// The GSA 2D/3D selection mode (field 1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum SelectionMode {
    /// `A` — switching between 2D and 3D done automatically.
    Automatic,
    /// `M` — forced to operate in 2D or 3D.
    Manual,
}

impl SelectionMode {
    fn parse(field: &str) -> Result<Self, ParseError> {
        match field {
            "A" => Ok(Self::Automatic),
            "M" => Ok(Self::Manual),
            _ => Err(ParseError::InvalidField),
        }
    }
}

/// The GSA fix dimensionality (field 2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum FixType {
    /// `1` — no fix.
    NoFix,
    /// `2` — 2D fix.
    TwoDimensional,
    /// `3` — 3D fix.
    ThreeDimensional,
}

impl FixType {
    /// Parse a `1`/`2`/`3` fix-dimension code. Shared with `PGRMZ`, whose
    /// third field uses the same encoding.
    pub(crate) fn parse(field: &str) -> Result<Self, ParseError> {
        match field {
            "1" => Ok(Self::NoFix),
            "2" => Ok(Self::TwoDimensional),
            "3" => Ok(Self::ThreeDimensional),
            _ => Err(ParseError::InvalidField),
        }
    }
}

/// The 12 satellite slots always precede the DOP triplet, so the tail is a
/// fixed layout regardless of how many slots are filled.
const SATELLITE_SLOTS: usize = 12;

pub(crate) fn parse(mut fields: Fields<'_>) -> Result<Gsa, ParseError> {
    let selection_mode = SelectionMode::parse(fields.next_required()?)?;
    let fix_type = FixType::parse(fields.next_required()?)?;

    // The remaining fields are 12 satellite slots, the PDOP/HDOP/VDOP
    // triplet, and (NMEA 4.10) an optional trailing system id. Anchoring
    // from the start keeps the DOP values at fixed offsets whether or not
    // the system id is present.
    let rest: Vec<&str> = fields.collect();
    let dop = rest.get(SATELLITE_SLOTS..SATELLITE_SLOTS + 3);
    let Some(&[pdop, hdop, vdop]) = dop else {
        return Err(ParseError::MissingField);
    };

    let mut satellite_prns = Vec::new();
    for slot in &rest[..SATELLITE_SLOTS] {
        if let Some(prn) = scalars::opt_u8(slot)? {
            satellite_prns.push(prn);
        }
    }

    Ok(Gsa {
        selection_mode,
        fix_type,
        satellite_prns,
        pdop: scalars::opt_f64(pdop)?,
        hdop: scalars::opt_f64(hdop)?,
        vdop: scalars::opt_f64(vdop)?,
    })
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;

    use super::*;
    use crate::{ParseResult, parse};

    fn gsa(line: &str) -> Gsa {
        match parse(line).unwrap() {
            ParseResult::Gsa(gsa) => gsa,
            other => panic!("expected GSA, got {other:?}"),
        }
    }

    #[test]
    fn three_d_fix_no_listed_satellites_from_corpus() {
        let gsa = gsa("$GPGSA,A,3,,,,,,,,,,,,,1.0,1.0,1.0*33");
        assert_eq!(gsa.selection_mode, SelectionMode::Automatic);
        assert_eq!(gsa.fix_type, FixType::ThreeDimensional);
        assert!(gsa.satellite_prns.is_empty());
        assert_relative_eq!(gsa.pdop.unwrap(), 1.0);
        assert_relative_eq!(gsa.hdop.unwrap(), 1.0);
        assert_relative_eq!(gsa.vdop.unwrap(), 1.0);
    }

    /// Sparse satellite slots, cross-checked against the `nmea` crate's
    /// test corpus (checksum `3C`).
    #[test]
    fn collects_only_filled_satellite_slots() {
        let gsa = gsa("$GPGSA,A,3,,,,,,16,18,,22,24,,,3.6,2.1,2.2*3C");
        assert_eq!(gsa.satellite_prns, [16, 18, 22, 24]);
        assert_relative_eq!(gsa.pdop.unwrap(), 3.6);
        assert_relative_eq!(gsa.hdop.unwrap(), 2.1);
        assert_relative_eq!(gsa.vdop.unwrap(), 2.2);
    }

    /// NMEA 4.10 form with a trailing system-id field, cross-checked
    /// against the `nmea` crate's corpus (checksum `01`).
    #[test]
    fn tolerates_trailing_system_id() {
        let gsa = gsa("$GNGSA,A,3,23,02,27,10,08,,,,,,,,3.45,1.87,2.89,1*01");
        assert_eq!(gsa.satellite_prns, [23, 2, 27, 10, 8]);
        assert_relative_eq!(gsa.pdop.unwrap(), 3.45);
        assert_relative_eq!(gsa.vdop.unwrap(), 2.89);
    }

    #[test]
    fn rejects_truncated_sentence() {
        // A broken form some receivers emit with no fix and only a handful
        // of fields cannot yield a DOP triplet.
        assert_eq!(parse("$GPGSA,A,1,,,,*32"), Err(ParseError::MissingField));
    }
}
