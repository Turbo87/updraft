use crate::encode::{EncodeError, SentenceEncoder, optional_field};
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

impl TryFrom<&Plxvs> for Vec<u8> {
    type Error = EncodeError;

    fn try_from(plxvs: &Plxvs) -> Result<Self, Self::Error> {
        let mut sentence = SentenceEncoder::new("PLXVS");
        sentence.field(&optional_field(plxvs.outside_air_temperature));
        sentence.field(&optional_field(plxvs.mode.map(PlxvsMode::to_nmea_field)));
        sentence.field(&optional_field(plxvs.supply_voltage));
        sentence.field(&optional_field(
            plxvs
                .igc_pressure_altitude
                .map(|altitude| altitude.as_meters()),
        ));
        sentence.text_field(plxvs.flap_position.as_deref(), "flap_position")?;
        Ok(sentence.finish())
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

    fn to_nmea_field(self) -> String {
        match self {
            Self::Vario => "0".to_owned(),
            Self::SpeedCommand => "1".to_owned(),
            Self::Other(value) => value.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Message, Step, parse};
    use claims::{assert_err_eq, assert_none, assert_ok, assert_some_eq};

    #[test]
    fn encodes_complete_plxvs_sentence() {
        insta::assert_snapshot!(encode_plxvs_sentence(&complete_plxvs()));
    }

    #[test]
    fn encodes_temperature_only_plxvs_sentence() {
        let plxvs = Plxvs {
            outside_air_temperature: Some(23.1),
            mode: None,
            supply_voltage: None,
            igc_pressure_altitude: None,
            flap_position: None,
        };

        insta::assert_snapshot!(encode_plxvs_sentence(&plxvs));
    }

    #[test]
    fn rejects_invalid_plxvs_flap_text() {
        let mut plxvs = complete_plxvs();
        plxvs.flap_position = Some("L,R".into());

        assert_err_eq!(
            Vec::<u8>::try_from(&plxvs),
            EncodeError::InvalidField("flap_position")
        );
    }

    #[test]
    fn parses_encoded_plxvs_sentence() {
        let expected = complete_plxvs();
        let sentence = assert_ok!(Vec::<u8>::try_from(&expected));
        let mut input = sentence.as_slice();

        let actual = match parse(&mut input) {
            Step::Frame(Message::Plxvs(plxvs)) => plxvs,
            step => panic!("expected encoded PLXVS frame, got {step:?}"),
        };

        assert_eq!(actual, expected);
    }

    fn complete_plxvs() -> Plxvs {
        Plxvs {
            outside_air_temperature: Some(18.4),
            mode: Some(PlxvsMode::SpeedCommand),
            supply_voltage: Some(12.1),
            igc_pressure_altitude: Some(Length::from_meters(1543.2)),
            flap_position: Some("L".into()),
        }
    }

    fn encode_plxvs_sentence(plxvs: &Plxvs) -> String {
        let sentence = assert_ok!(Vec::<u8>::try_from(plxvs));
        let sentence = assert_ok!(String::from_utf8(sentence));
        assert!(sentence.ends_with("\r\n"));
        sentence
    }

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
        assert_some_eq!(PlxvsMode::from_field(b"0"), PlxvsMode::Vario);
        assert_some_eq!(PlxvsMode::from_field(b"1"), PlxvsMode::SpeedCommand);
        assert_some_eq!(PlxvsMode::from_field(b"5"), PlxvsMode::Other(5));
    }
}
