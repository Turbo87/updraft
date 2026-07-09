//! `$LXWP0` — `LXNav` air-data telemetry: airspeed, barometric altitude, a
//! burst of variometer samples, heading, and wind.

use updraft_units::{Angle, Length, Speed};

use crate::error::ParseError;
use crate::fields::Fields;
use crate::scalars;

/// Number of total-energy variometer samples in one `$LXWP0` (six per
/// second).
pub const VARIO_SAMPLES: usize = 6;

/// A parsed `$LXWP0` sentence.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Lxwp0 {
    /// Whether the last fix was stored by the flight logger (`Y`/`N`).
    pub logger_stored: Option<bool>,
    /// True airspeed (`LXNav` sends it in km/h in this field).
    pub true_airspeed: Option<Speed>,
    /// Barometric altitude, uncorrected (referenced to 1013.25 hPa).
    pub pressure_altitude: Option<Length>,
    /// The six total-energy variometer samples from the last second.
    pub vario: [Option<Speed>; VARIO_SAMPLES],
    /// Aircraft heading, when a compass is connected.
    pub heading: Option<Angle>,
    /// Wind direction.
    pub wind_direction: Option<Angle>,
    /// Wind speed (sent in km/h).
    pub wind_speed: Option<Speed>,
}

pub(crate) fn parse(fields: Fields<'_>) -> Result<Lxwp0, ParseError> {
    let fields: Vec<&str> = fields.collect();
    let get = |index: usize| fields.get(index).copied().unwrap_or("");

    let logger_stored = match get(0) {
        "Y" => Some(true),
        "N" => Some(false),
        "" => None,
        _ => return Err(ParseError::InvalidField),
    };

    let mut vario = [None; VARIO_SAMPLES];
    for (offset, sample) in vario.iter_mut().enumerate() {
        *sample = scalars::opt_f64(get(3 + offset))?.map(Speed::from_meters_per_second);
    }

    Ok(Lxwp0 {
        logger_stored,
        true_airspeed: scalars::opt_f64(get(1))?.map(Speed::from_kilometers_per_hour),
        pressure_altitude: scalars::opt_f64(get(2))?.map(Length::from_meters),
        vario,
        heading: scalars::opt_f64(get(9))?.map(Angle::from_degrees),
        wind_direction: scalars::opt_f64(get(10))?.map(Angle::from_degrees),
        wind_speed: scalars::opt_f64(get(11))?.map(Speed::from_kilometers_per_hour),
    })
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;

    use super::*;
    use crate::{ParseResult, parse};

    fn lxwp0(line: &str) -> Lxwp0 {
        match parse(line).unwrap() {
            ParseResult::Lxwp0(lxwp0) => lxwp0,
            other => panic!("expected LXWP0, got {other:?}"),
        }
    }

    #[test]
    fn telemetry_with_single_vario_sample() {
        let lxwp0 = lxwp0("$LXWP0,Y,222.3,1665.5,1.71,,,,,,239,174,10.1*47");
        assert_eq!(lxwp0.logger_stored, Some(true));
        assert_relative_eq!(lxwp0.true_airspeed.unwrap().as_kilometers_per_hour(), 222.3);
        assert_relative_eq!(lxwp0.pressure_altitude.unwrap().as_meters(), 1665.5);
        assert_relative_eq!(lxwp0.vario[0].unwrap().as_meters_per_second(), 1.71);
        assert_eq!(lxwp0.vario[1], None);
        assert_relative_eq!(lxwp0.heading.unwrap().as_degrees(), 239.0);
        assert_relative_eq!(lxwp0.wind_direction.unwrap().as_degrees(), 174.0);
        assert_relative_eq!(lxwp0.wind_speed.unwrap().as_kilometers_per_hour(), 10.1);
    }

    #[test]
    fn all_six_vario_samples() {
        let lxwp0 = lxwp0("$LXWP0,N,180.0,1200.0,0.1,0.2,0.3,0.4,0.5,0.6,,,*50");
        assert_eq!(lxwp0.logger_stored, Some(false));
        for (offset, sample) in lxwp0.vario.iter().enumerate() {
            assert_relative_eq!(
                sample.unwrap().as_meters_per_second(),
                0.1 * (offset as f64 + 1.0)
            );
        }
        assert_eq!(lxwp0.heading, None);
    }
}
