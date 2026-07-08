//! `!w` — Cambridge CAI302 proprietary air-data record.
//!
//! Unlike the `$P…` sentences this record uses the `!` line delimiter, so
//! its address (the text before the first comma) is simply `w`. It packs
//! wind, true altitude, QNH, true airspeed, three variometer readings, and
//! the instrument's MacCready/ballast/bugs settings, several of them
//! encoded with a scale factor and an additive offset (see the field
//! comments). Fields are read tolerantly: a missing or empty field becomes
//! `None` rather than an error, matching how the CAI302 firmware omits
//! values it has no data for.

use updraft_units::{Angle, Length, Pressure, Speed};

use crate::error::ParseError;
use crate::fields::Fields;
use crate::scalars;

/// A parsed `!w` record.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CaiW {
    /// Vector wind direction (the direction the wind blows *towards*, as
    /// reported by the instrument).
    pub wind_direction: Option<Angle>,
    /// Vector wind speed.
    pub wind_speed: Option<Speed>,
    /// Age of the wind estimate, in seconds.
    pub wind_age: Option<f64>,
    /// Along-track wind component, positive for a headwind.
    pub component_wind: Option<Speed>,
    /// True altitude.
    pub true_altitude: Option<Length>,
    /// Instrument QNH setting.
    pub qnh: Option<Pressure>,
    /// True airspeed.
    pub true_airspeed: Option<Speed>,
    /// Total-energy variometer reading.
    pub vario: Option<Speed>,
    /// Averaged variometer reading.
    pub averager: Option<Speed>,
    /// Relative (netto) variometer reading.
    pub relative_vario: Option<Speed>,
    /// Instrument MacCready setting.
    pub mac_cready: Option<Speed>,
    /// Instrument ballast setting, as a fraction of capacity (`0.0..=1.0`).
    pub ballast_fraction: Option<f64>,
    /// Instrument bugs setting, as a fraction (`0.0..=1.0`).
    pub bugs_fraction: Option<f64>,
}

/// Variometer readings are sent in tenths of a knot, offset by 200
/// (so `200` is 0 kt).
fn vario_knots(raw: f64) -> Speed {
    Speed::from_knots((raw - 200.0) / 10.0)
}

pub(crate) fn parse(fields: Fields<'_>) -> Result<CaiW, ParseError> {
    let fields: Vec<&str> = fields.collect();
    let get = |index: usize| fields.get(index).copied().unwrap_or("");

    Ok(CaiW {
        wind_direction: scalars::opt_f64(get(0))?.map(Angle::from_degrees),
        // Tenths of a metre per second.
        wind_speed: scalars::opt_f64(get(1))?.map(|v| Speed::from_meters_per_second(v / 10.0)),
        wind_age: scalars::opt_f64(get(2))?,
        // Tenths of a metre per second, offset by 500 (500 = 0 m/s).
        component_wind: scalars::opt_f64(get(3))?
            .map(|v| Speed::from_meters_per_second((v - 500.0) / 10.0)),
        // Metres, offset by 1000.
        true_altitude: scalars::opt_f64(get(4))?.map(|v| Length::from_meters(v - 1000.0)),
        qnh: scalars::opt_f64(get(5))?.map(Pressure::from_hectopascals),
        // Hundredths of a metre per second.
        true_airspeed: scalars::opt_f64(get(6))?.map(|v| Speed::from_meters_per_second(v / 100.0)),
        vario: scalars::opt_f64(get(7))?.map(vario_knots),
        averager: scalars::opt_f64(get(8))?.map(vario_knots),
        relative_vario: scalars::opt_f64(get(9))?.map(vario_knots),
        // Tenths of a knot.
        mac_cready: scalars::opt_f64(get(10))?.map(|v| Speed::from_knots(v / 10.0)),
        ballast_fraction: scalars::opt_f64(get(11))?.map(|v| v / 100.0),
        bugs_fraction: scalars::opt_f64(get(12))?.map(|v| v / 100.0),
    })
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;

    use super::*;
    use crate::{ParseResult, parse};

    fn cai_w(line: &str) -> CaiW {
        match parse(line).unwrap() {
            ParseResult::CaiW(w) => w,
            other => panic!("expected !w, got {other:?}"),
        }
    }

    #[test]
    fn decodes_scales_and_offsets() {
        let w = cai_w("!w,235,26,3,505,1200,1013,3500,215,205,198,25,50,15*59");
        assert_relative_eq!(w.wind_direction.unwrap().as_degrees(), 235.0);
        assert_relative_eq!(w.wind_speed.unwrap().as_meters_per_second(), 2.6);
        assert_relative_eq!(w.wind_age.unwrap(), 3.0);
        // 505 -> +0.5 m/s headwind.
        assert_relative_eq!(w.component_wind.unwrap().as_meters_per_second(), 0.5);
        // 1200 -> 200 m true altitude.
        assert_relative_eq!(w.true_altitude.unwrap().as_meters(), 200.0);
        assert_relative_eq!(w.qnh.unwrap().as_hectopascals(), 1013.0);
        // 3500 -> 35.0 m/s TAS.
        assert_relative_eq!(w.true_airspeed.unwrap().as_meters_per_second(), 35.0);
        // 215 -> +1.5 kt, 205 -> +0.5 kt, 198 -> -0.2 kt.
        assert_relative_eq!(w.vario.unwrap().as_knots(), 1.5);
        assert_relative_eq!(w.averager.unwrap().as_knots(), 0.5);
        assert_relative_eq!(w.relative_vario.unwrap().as_knots(), -0.2);
        // 25 -> 2.5 kt MacCready.
        assert_relative_eq!(w.mac_cready.unwrap().as_knots(), 2.5);
        assert_relative_eq!(w.ballast_fraction.unwrap(), 0.50);
        assert_relative_eq!(w.bugs_fraction.unwrap(), 0.15);
    }

    #[test]
    fn tolerates_empty_fields() {
        // Only true altitude and QNH populated; everything else empty.
        let w = cai_w("!w,,,,,1050,1013,,,,,,,*5C");
        assert_eq!(w.wind_direction, None);
        assert_eq!(w.true_airspeed, None);
        assert_eq!(w.vario, None);
        assert_relative_eq!(w.true_altitude.unwrap().as_meters(), 50.0);
        assert_relative_eq!(w.qnh.unwrap().as_hectopascals(), 1013.0);
    }
}
