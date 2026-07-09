//! `$LXWP3` — `LXNav` instrument settings.
//!
//! The sentence carries a long list of instrument-tuning fields
//! (speed-command mode, filter periods, speed-to-fly tab, and so on). Only
//! the fields with a clear, portable meaning are modelled; the rest are
//! device-specific tuning knobs and are left unparsed, matching how
//! `XCSoar` uses this sentence.

use updraft_units::Length;

use crate::error::ParseError;
use crate::fields::Fields;
use crate::scalars;

/// A parsed `$LXWP3` sentence (the modelled subset of its fields).
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Lxwp3 {
    /// Altitude offset used to derive the QNH from the pressure altitude
    /// (field 0, reported in feet).
    pub altitude_offset: Option<Length>,
    /// Variometer filter period, in seconds (field 2).
    pub vario_filter: Option<f64>,
    /// Glider name / polar name configured on the instrument (field 11).
    pub glider_name: Option<String>,
}

pub(crate) fn parse(fields: Fields<'_>) -> Result<Lxwp3, ParseError> {
    let fields: Vec<&str> = fields.collect();
    let get = |index: usize| fields.get(index).copied().unwrap_or("");
    Ok(Lxwp3 {
        altitude_offset: scalars::opt_f64(get(0))?.map(Length::from_feet),
        vario_filter: scalars::opt_f64(get(2))?,
        glider_name: (!get(11).is_empty()).then(|| get(11).to_owned()),
    })
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;

    use crate::{ParseResult, parse};

    #[test]
    fn modelled_fields() {
        let ParseResult::Lxwp3(lxwp3) =
            parse("$LXWP3,100,0,3,1,0,0,0,0,0,0,0,MyGlider,0*0A").unwrap()
        else {
            panic!("expected LXWP3");
        };
        assert_relative_eq!(lxwp3.altitude_offset.unwrap().as_feet(), 100.0);
        assert_relative_eq!(lxwp3.vario_filter.unwrap(), 3.0);
        assert_eq!(lxwp3.glider_name.as_deref(), Some("MyGlider"));
    }
}
