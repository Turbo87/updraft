//! `$LXWP2` — `LXNav` glide-computer settings (used for bidirectional
//! settings sync).

use updraft_units::Speed;

use crate::error::ParseError;
use crate::fields::Fields;
use crate::scalars;

/// A parsed `$LXWP2` sentence.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Lxwp2 {
    /// MacCready setting.
    pub mac_cready: Option<Speed>,
    /// Ballast, expressed as an overload factor (`1.0..=1.5`).
    pub ballast_overload: Option<f64>,
    /// Bugs, expressed as a cleanliness factor where `1.0` is clean.
    pub bugs: Option<f64>,
    /// Polar coefficient `a`.
    pub polar_a: Option<f64>,
    /// Polar coefficient `b`.
    pub polar_b: Option<f64>,
    /// Polar coefficient `c`.
    pub polar_c: Option<f64>,
    /// Audio volume.
    pub volume: Option<u32>,
}

/// Decode the bugs field into a cleanliness factor (`1.0` = clean).
///
/// Older LX160 firmware reports `1.00`/`1.05`/`1.10`, while other devices
/// report a percentage (`0`, `5`, `10`, …); both are normalized here the
/// way `XCSoar` does it.
fn decode_bugs(raw: f64) -> f64 {
    if (1.0..=1.5).contains(&raw) {
        2.0 - raw
    } else {
        (100.0 - raw) / 100.0
    }
}

pub(crate) fn parse(fields: Fields<'_>) -> Result<Lxwp2, ParseError> {
    let fields: Vec<&str> = fields.collect();
    let get = |index: usize| fields.get(index).copied().unwrap_or("");
    Ok(Lxwp2 {
        mac_cready: scalars::opt_f64(get(0))?.map(Speed::from_meters_per_second),
        ballast_overload: scalars::opt_f64(get(1))?,
        bugs: scalars::opt_f64(get(2))?.map(decode_bugs),
        polar_a: scalars::opt_f64(get(3))?,
        polar_b: scalars::opt_f64(get(4))?,
        polar_c: scalars::opt_f64(get(5))?,
        volume: scalars::opt(get(6), scalars::u32)?,
    })
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;

    use super::*;
    use crate::{ParseResult, parse};

    fn lxwp2(line: &str) -> Lxwp2 {
        match parse(line).unwrap() {
            ParseResult::Lxwp2(lxwp2) => lxwp2,
            other => panic!("expected LXWP2, got {other:?}"),
        }
    }

    #[test]
    fn settings_with_percentage_bugs() {
        let lxwp2 = lxwp2("$LXWP2,1.5,1.10,20,2.0,-3.5,0.5,7*0E");
        assert_relative_eq!(lxwp2.mac_cready.unwrap().as_meters_per_second(), 1.5);
        assert_relative_eq!(lxwp2.ballast_overload.unwrap(), 1.10);
        // 20% bugs -> 0.80 clean.
        assert_relative_eq!(lxwp2.bugs.unwrap(), 0.80);
        assert_relative_eq!(lxwp2.polar_a.unwrap(), 2.0);
        assert_relative_eq!(lxwp2.polar_b.unwrap(), -3.5);
        assert_relative_eq!(lxwp2.polar_c.unwrap(), 0.5);
        assert_eq!(lxwp2.volume, Some(7));
    }

    #[test]
    fn lx160_style_bugs() {
        // LX160 firmware reports bugs as 1.00/1.05/1.10; 1.10 -> 0.90.
        let lxwp2 = lxwp2("$LXWP2,2.0,1.05,1.10,2.0,-3.5,0.5,5*12");
        assert_relative_eq!(lxwp2.bugs.unwrap(), 0.90);
    }
}
