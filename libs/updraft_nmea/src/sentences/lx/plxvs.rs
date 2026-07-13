use crate::field::FieldsIter;
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
    pub fn parse(mut fields: FieldsIter<'_>) -> Self {
        Self {
            outside_air_temperature: fields.f64(),
            mode: fields.bytes().and_then(PlxvsMode::from_field),
            supply_voltage: fields.f64(),
            igc_pressure_altitude: fields.f64().map(Length::from_meters),
            flap_position: fields.text(),
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
    fn from_field(field: &[u8]) -> Option<Self> {
        match field {
            b"0" => Some(Self::Vario),
            b"1" => Some(Self::SpeedCommand),
            field => btoi::btou(field).ok().map(Self::Other),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use claims::{assert_none, assert_some_eq};

    #[test]
    fn parses_the_short_form() {
        let plxvs = Plxvs::parse(FieldsIter::new(b"23.1,0,12.3,"));
        assert_some_eq!(plxvs.outside_air_temperature, 23.1);
        assert_some_eq!(plxvs.mode, PlxvsMode::Vario);
        assert_some_eq!(plxvs.supply_voltage, 12.3);
        assert_none!(plxvs.igc_pressure_altitude);
        assert_none!(plxvs.flap_position);
    }

    #[test]
    fn parses_the_recorder_altitude_and_flap() {
        let plxvs = Plxvs::parse(FieldsIter::new(b"18.4,1,12.1,1543.2,L,"));
        assert_some_eq!(plxvs.mode, PlxvsMode::SpeedCommand);
        assert_some_eq!(plxvs.igc_pressure_altitude, Length::from_meters(1543.2));
        assert_some_eq!(plxvs.flap_position, "L".into());
    }

    #[test]
    fn maps_mode_values() {
        assert_eq!(PlxvsMode::from_field(b"0"), Some(PlxvsMode::Vario));
        assert_eq!(PlxvsMode::from_field(b"1"), Some(PlxvsMode::SpeedCommand));
        assert_eq!(PlxvsMode::from_field(b"5"), Some(PlxvsMode::Other(5)));
    }
}
