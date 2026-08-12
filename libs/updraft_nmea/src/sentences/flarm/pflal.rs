use crate::Time;
use crate::field::{FieldsIter, text};

/// A supported FLARM debug message (`$PFLAL`) with its source timestamp.
#[derive(Clone, Debug, PartialEq)]
pub struct Pflal {
    /// UTC time carried in the first six bytes of the debug message.
    pub timestamp: Time,
    /// Typed content following the timestamp.
    pub content: PflalContent,
}

/// Supported inner content from a FLARM debug message.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum PflalContent {
    /// Power and operating temperature diagnostics (`39 PWR`).
    Power(PflalPower),
    /// A firmware-specific configuration entry (`07`).
    Configuration(PflalConfiguration),
}

/// A key and raw text value from a `PFLAL` configuration diagnostic.
#[derive(Clone, Debug, PartialEq)]
pub struct PflalConfiguration {
    /// Configuration key reported by the device. Invalid UTF-8 uses the
    /// Unicode replacement character.
    pub key: Box<str>,
    /// Complete value after the key separator, including embedded spaces.
    /// Invalid UTF-8 uses the Unicode replacement character.
    pub value: Box<str>,
}

/// Values reported by `PFLAL` power diagnostics.
#[derive(Clone, Debug, PartialEq)]
pub struct PflalPower {
    /// Firmware-specific `STATE` value with an unknown meaning.
    pub state: u8,
    /// Firmware-specific `LVL` value with an unknown meaning.
    pub level: u8,
    /// Firmware-specific `BAT` value with an unknown meaning.
    pub battery: f64,
    /// External supply voltage (`EXT`) in volts.
    pub external_voltage: f64,
    /// Device operating temperature (`TEMP`) in degrees Celsius.
    pub operating_temperature: f64,
}

impl Pflal {
    pub fn parse(mut fields: FieldsIter<'_>) -> Option<Self> {
        let message = fields.bytes()?;
        let timestamp = Time::parse(message.get(..6)?)?;
        let content = PflalContent::parse(message.get(6..)?)?;
        Some(Self { timestamp, content })
    }
}

impl PflalContent {
    fn parse(payload: &[u8]) -> Option<Self> {
        if let Some(payload) = payload.strip_prefix(b"07") {
            return PflalConfiguration::parse(payload).map(Self::Configuration);
        }

        let mut tokens = payload
            .split(u8::is_ascii_whitespace)
            .filter(|token| !token.is_empty());
        expect_token(&mut tokens, b"39")?;
        expect_token(&mut tokens, b"PWR")?;
        expect_token(&mut tokens, b"STATE")?;
        let state = btoi::btoi(tokens.next()?).ok()?;
        expect_token(&mut tokens, b"LVL")?;
        let level = btoi::btoi(tokens.next()?).ok()?;
        expect_token(&mut tokens, b"BAT")?;
        let battery: f64 = fast_float2::parse(tokens.next()?).ok()?;
        expect_token(&mut tokens, b"EXT")?;
        let external_voltage: f64 = fast_float2::parse(tokens.next()?).ok()?;
        expect_token(&mut tokens, b"TEMP")?;
        let operating_temperature: f64 = fast_float2::parse(tokens.next()?).ok()?;
        Some(Self::Power(PflalPower {
            state,
            level,
            battery,
            external_voltage,
            operating_temperature,
        }))
    }
}

impl PflalConfiguration {
    fn parse(payload: &[u8]) -> Option<Self> {
        let separator = payload.iter().position(u8::is_ascii_whitespace)?;
        let (key, value) = payload.split_at(separator);
        (!key.is_empty()).then_some(Self {
            key: text(key),
            value: text(&value[1..]),
        })
    }
}

fn expect_token<'a>(tokens: &mut impl Iterator<Item = &'a [u8]>, expected: &[u8]) -> Option<()> {
    (tokens.next()? == expected).then_some(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use claims::{assert_none, assert_some, assert_some_eq};

    #[test]
    fn parses_power_diagnostics() {
        let timestamp = assert_some!(Time::from_hms_millis(9, 54, 17, 0));
        let pflal = assert_some!(Pflal::parse(FieldsIter::new(
            b"09541739 PWR STATE 4 LVL 28 BAT 0.00 EXT 13.06 TEMP 37.8",
        )));
        assert_eq!(pflal.timestamp, timestamp);
        let PflalContent::Power(power) = pflal.content else {
            panic!("expected power diagnostics");
        };
        assert_eq!(power.state, 4);
        assert_eq!(power.level, 28);
        assert_eq!(power.battery, 0.0);
        assert_eq!(power.external_voltage, 13.06);
        assert_eq!(power.operating_temperature, 37.8);
    }

    #[test]
    fn parses_a_configuration_diagnostic() {
        let pflal = assert_some!(Pflal::parse(FieldsIter::new(b"09593807FRW 7.40")));
        assert_eq!(
            pflal.content,
            PflalContent::Configuration(PflalConfiguration {
                key: "FRW".into(),
                value: "7.40".into(),
            })
        );
    }

    #[test]
    fn preserves_an_empty_configuration_value() {
        let pflal = assert_some!(Pflal::parse(FieldsIter::new(b"09593807OBSTEXP ")));
        assert_eq!(
            pflal.content,
            PflalContent::Configuration(PflalConfiguration {
                key: "OBSTEXP".into(),
                value: "".into(),
            })
        );
    }

    #[test]
    fn preserves_an_unknown_configuration_key_and_value_spaces() {
        let pflal = assert_some!(Pflal::parse(FieldsIter::new(
            b"09593807FUTURE raw value with spaces",
        )));
        assert_eq!(
            pflal.content,
            PflalContent::Configuration(PflalConfiguration {
                key: "FUTURE".into(),
                value: "raw value with spaces".into(),
            })
        );
    }

    #[test]
    fn rejects_a_configuration_without_a_key_or_separator() {
        assert_none!(Pflal::parse(FieldsIter::new(b"09593807FRW")));
        assert_none!(Pflal::parse(FieldsIter::new(b"09593807 7.40")));
    }

    #[test]
    fn rejects_an_unsupported_payload() {
        assert_none!(Pflal::parse(FieldsIter::new(b"095417GPS 7 39")));
    }

    #[test]
    fn rejects_an_invalid_timestamp() {
        let fields = FieldsIter::new(b"25541739 PWR STATE 4 LVL 28 BAT 0.00 EXT 13.06 TEMP 37.8");
        assert_none!(Pflal::parse(fields));
    }

    #[test]
    fn rejects_missing_or_misordered_labels() {
        let fields = FieldsIter::new(b"09541739 PWR LVL 28 BAT 0.00 EXT 13.06 TEMP 37.8");
        assert_none!(Pflal::parse(fields));

        let fields = FieldsIter::new(b"09541739 PWR STATE 4 LVL 28 BAT 0.00 TEMP 37.8 EXT 13.06");
        assert_none!(Pflal::parse(fields));
    }

    #[test]
    fn rejects_invalid_power_values() {
        let fields = FieldsIter::new(b"09541739 PWR STATE X LVL 28 BAT 0.00 EXT 13.06 TEMP 37.8");
        assert_none!(Pflal::parse(fields));

        let fields = FieldsIter::new(b"09541739 PWR STATE 4 LVL 256 BAT 0.00 EXT 13.06 TEMP 37.8");
        assert_none!(Pflal::parse(fields));

        let fields = FieldsIter::new(b"09541739 PWR STATE 4 LVL 28 BAT X EXT 13.06 TEMP 37.8");
        assert_none!(Pflal::parse(fields));
    }

    #[test]
    fn parses_non_finite_power_values() {
        let fields = FieldsIter::new(b"09541739 PWR STATE 4 LVL 28 BAT 0.00 EXT NaN TEMP 37.8");
        let pflal = assert_some!(Pflal::parse(fields));
        let PflalContent::Power(power) = pflal.content else {
            panic!("expected power diagnostics");
        };
        assert!(power.external_voltage.is_nan());

        let fields = FieldsIter::new(b"09541739 PWR STATE 4 LVL 28 BAT 0.00 EXT 13.06 TEMP inf");
        let pflal = assert_some!(Pflal::parse(fields));
        let PflalContent::Power(power) = pflal.content else {
            panic!("expected power diagnostics");
        };
        assert_eq!(power.operating_temperature, f64::INFINITY);
    }

    #[test]
    fn ignores_tokens_after_power_diagnostics() {
        let pflal = Pflal::parse(FieldsIter::new(
            b"09541739 PWR STATE 4 LVL 28 BAT 0.00 EXT 13.06 TEMP 37.8 EXTRA",
        ));
        assert_some_eq!(
            pflal.map(|pflal| pflal.content),
            PflalContent::Power(PflalPower {
                state: 4,
                level: 28,
                battery: 0.0,
                external_voltage: 13.06,
                operating_temperature: 37.8,
            })
        );
    }
}
