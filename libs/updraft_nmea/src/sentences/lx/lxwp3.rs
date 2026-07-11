use crate::field::FieldsIter;
use updraft_units::{Length, Speed};

/// `$LXWP3`: vario and speed-command configuration.
///
/// These settings rarely change and are mostly of interest for mirroring
/// the instrument's configuration. Each value is kept as sent.
#[derive(Clone, Debug, PartialEq)]
pub struct Lxwp3 {
    pub altitude_offset: Option<Length>,
    /// How the vario/speed-command changeover is driven.
    pub speed_command_mode: Option<Lxwp3SpeedCommandMode>,
    /// Vario needle filter time constant, in seconds.
    pub vario_filter: Option<f64>,
    /// Total-energy compensation filter, in seconds (0 disables it).
    pub te_filter: Option<f64>,
    /// Total-energy compensation level, in percent (0-250).
    pub te_level: Option<f64>,
    /// Integrator averaging time, in seconds.
    pub vario_average: Option<f64>,
    /// Vario scale range (2.5, 5, or 10).
    pub vario_range: Option<f64>,
    /// Speed-command dead band ("area of silence").
    pub speed_command_deadband: Option<Speed>,
    /// The external switch / taster behaviour.
    pub switch_mode: Option<Lxwp3SwitchMode>,
    /// Airspeed at which the auto changeover switches to speed command,
    /// used when [`speed_command_mode`](Self::speed_command_mode) is
    /// airspeed-driven.
    pub speed_command_switch_speed: Option<Speed>,
    /// "Smart" vario filtering coefficient, in m/s².
    pub smart_diff: Option<f64>,
    /// Glider name (up to 14 characters). Non-UTF-8 bytes are replaced
    /// with the Unicode replacement character.
    pub glider_name: Option<Box<str>>,
    /// Local time offset from UTC, in hours.
    pub time_offset: Option<f64>,
}

impl Lxwp3 {
    pub fn parse(mut fields: FieldsIter<'_>) -> Self {
        Self {
            altitude_offset: fields.f64().map(Length::from_feet),
            speed_command_mode: fields.u8().map(Lxwp3SpeedCommandMode::from_value),
            vario_filter: fields.f64(),
            te_filter: fields.f64(),
            te_level: fields.f64(),
            vario_average: fields.f64(),
            vario_range: fields.f64(),
            speed_command_deadband: fields.f64().map(Speed::from_meters_per_second),
            switch_mode: fields.u8().map(Lxwp3SwitchMode::from_value),
            speed_command_switch_speed: fields.f64().map(Speed::from_kilometers_per_hour),
            smart_diff: fields.f64(),
            glider_name: fields.text(),
            time_offset: fields.f64(),
        }
    }
}

/// How the automatic vario/speed-command changeover is driven, from the
/// `scmode` field.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Lxwp3SpeedCommandMode {
    /// `0`: driven by the external switch.
    External,
    /// `1`: speed command whenever not circling.
    Circling,
    /// `2`: driven by airspeed, crossing the configured switch speed.
    Airspeed,
    Other(u8),
}

impl Lxwp3SpeedCommandMode {
    fn from_value(value: u8) -> Self {
        match value {
            0 => Self::External,
            1 => Self::Circling,
            2 => Self::Airspeed,
            other => Self::Other(other),
        }
    }
}

/// The external switch / taster behaviour, from the `sclow` field.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Lxwp3SwitchMode {
    /// `0`: normal.
    Normal,
    /// `1`: inverted.
    Inverted,
    /// `2`: momentary taster.
    Taster,
    Other(u8),
}

impl Lxwp3SwitchMode {
    fn from_value(value: u8) -> Self {
        match value {
            0 => Self::Normal,
            1 => Self::Inverted,
            2 => Self::Taster,
            other => Self::Other(other),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use claims::{assert_none, assert_some_eq};

    #[test]
    fn parses_a_full_configuration() {
        let fields = FieldsIter::new(b"47.76,0,2.0,5.0,15,30,2.5,1.0,0,100,0.1,,0");
        let lxwp3 = Lxwp3::parse(fields);
        assert_some_eq!(lxwp3.altitude_offset, Length::from_feet(47.76));
        assert_some_eq!(lxwp3.speed_command_mode, Lxwp3SpeedCommandMode::External);
        assert_some_eq!(lxwp3.vario_filter, 2.0);
        assert_some_eq!(lxwp3.te_filter, 5.0);
        assert_some_eq!(lxwp3.te_level, 15.0);
        assert_some_eq!(lxwp3.vario_average, 30.0);
        assert_some_eq!(lxwp3.vario_range, 2.5);
        assert_some_eq!(
            lxwp3.speed_command_deadband,
            Speed::from_meters_per_second(1.0)
        );
        assert_some_eq!(lxwp3.switch_mode, Lxwp3SwitchMode::Normal);
        assert_some_eq!(
            lxwp3.speed_command_switch_speed,
            Speed::from_kilometers_per_hour(100.0)
        );
        assert_some_eq!(lxwp3.smart_diff, 0.1);
        assert_none!(lxwp3.glider_name);
        assert_some_eq!(lxwp3.time_offset, 0.0);
    }

    #[test]
    fn reads_a_glider_name_and_enumerated_modes() {
        let fields = FieldsIter::new(b"105,2,5.0,0,29,20,10.0,1.3,1,120,0,KA6e,2");
        let lxwp3 = Lxwp3::parse(fields);
        assert_some_eq!(lxwp3.speed_command_mode, Lxwp3SpeedCommandMode::Airspeed);
        assert_some_eq!(lxwp3.switch_mode, Lxwp3SwitchMode::Inverted);
        assert_some_eq!(lxwp3.glider_name, "KA6e".into());
        assert_some_eq!(lxwp3.time_offset, 2.0);
    }

    #[test]
    fn an_absent_mode_field_reads_as_absent() {
        let fields = FieldsIter::new(b"0,,,,,,,,,,,,");
        let lxwp3 = Lxwp3::parse(fields);
        assert_none!(lxwp3.speed_command_mode);
        assert_none!(lxwp3.switch_mode);
    }

    #[test]
    fn maps_enumerated_fields() {
        assert_eq!(
            Lxwp3SpeedCommandMode::from_value(0),
            Lxwp3SpeedCommandMode::External
        );
        assert_eq!(
            Lxwp3SpeedCommandMode::from_value(1),
            Lxwp3SpeedCommandMode::Circling
        );
        assert_eq!(
            Lxwp3SpeedCommandMode::from_value(2),
            Lxwp3SpeedCommandMode::Airspeed
        );
        assert_eq!(
            Lxwp3SpeedCommandMode::from_value(7),
            Lxwp3SpeedCommandMode::Other(7)
        );

        assert_eq!(Lxwp3SwitchMode::from_value(2), Lxwp3SwitchMode::Taster);
        assert_eq!(Lxwp3SwitchMode::from_value(9), Lxwp3SwitchMode::Other(9));
    }
}
