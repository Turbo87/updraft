use crate::field::{f64_field, field, parsed_field, text};
use updraft_units::Length;

/// `$PLXVS`: the vario "slow data" sentence from LXNAV varios, sent every
/// few seconds.
///
/// Carries temperature, the current flight mode, supply voltage, and, on
/// newer firmware, the recorder's altitude and flap position.
#[derive(Clone, Debug, PartialEq)]
pub struct Plxvs {
    /// Outside air temperature, in °C.
    pub outside_air_temperature: Option<f64>,
    /// The current vario / speed-command flight mode.
    pub mode: Option<PlxvsMode>,
    /// Supply voltage, in volts.
    pub supply_voltage: Option<f64>,
    /// Barometric altitude the built-in IGC recorder is using.
    pub igc_pressure_altitude: Option<Length>,
    /// Flap position, e.g. `L` for landing.
    pub flap_position: Option<Box<str>>,
}

impl Plxvs {
    pub fn parse(fields: &[&[u8]]) -> Self {
        Self {
            outside_air_temperature: f64_field(fields, 0),
            mode: parsed_field(fields, 1).map(PlxvsMode::from_value),
            supply_voltage: f64_field(fields, 2),
            igc_pressure_altitude: f64_field(fields, 3).map(Length::from_meters),
            flap_position: field(fields, 4).map(text),
        }
    }
}

/// The flight mode reported in a `PLXVS` sentence.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PlxvsMode {
    /// `0`: vario mode (circling).
    Vario,
    /// `1`: speed-command mode (cruise).
    SpeedCommand,
    Other(u8),
}

impl PlxvsMode {
    fn from_value(value: u8) -> Self {
        match value {
            0 => Self::Vario,
            1 => Self::SpeedCommand,
            other => Self::Other(other),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use claims::{assert_none, assert_some_eq};

    #[test]
    fn parses_the_short_form() {
        let fields: [&[u8]; 4] = [b"23.1", b"0", b"12.3", b""];
        let plxvs = Plxvs::parse(&fields);
        assert_some_eq!(plxvs.outside_air_temperature, 23.1);
        assert_some_eq!(plxvs.mode, PlxvsMode::Vario);
        assert_some_eq!(plxvs.supply_voltage, 12.3);
        assert_none!(plxvs.igc_pressure_altitude);
        assert_none!(plxvs.flap_position);
    }

    #[test]
    fn parses_the_recorder_altitude_and_flap() {
        let fields: [&[u8]; 6] = [b"18.4", b"1", b"12.1", b"1543.2", b"L", b""];
        let plxvs = Plxvs::parse(&fields);
        assert_some_eq!(plxvs.mode, PlxvsMode::SpeedCommand);
        assert_some_eq!(plxvs.igc_pressure_altitude, Length::from_meters(1543.2));
        assert_some_eq!(plxvs.flap_position, "L".into());
    }

    #[test]
    fn maps_mode_values() {
        assert_eq!(PlxvsMode::from_value(0), PlxvsMode::Vario);
        assert_eq!(PlxvsMode::from_value(1), PlxvsMode::SpeedCommand);
        assert_eq!(PlxvsMode::from_value(5), PlxvsMode::Other(5));
    }
}
