use crate::encode::{EncodeError, SentenceEncoder, optional_field};
use crate::field::FieldsIter;
use updraft_units::Speed;

/// `$LXWP2`: the glide-computer settings: MacCready, ballast, bugs, the
/// active polar, and audio volume.
///
/// On LX systems this is also accepted as input, so a connected device and
/// the instrument can keep these settings in sync.
///
/// The LXNAV polar coefficient normalization (scaling, `v` in km/h/100) is
/// left to the consumer.
#[derive(Clone, Debug, PartialEq)]
pub struct Lxwp2 {
    /// MacCready setting.
    pub mac_cready: Option<Speed>,
    /// Ballast as an overload factor (total mass over reference mass),
    /// nominally 1.0-1.5.
    pub ballast: Option<f64>,
    /// Bugs as a percentage degradation, nominally 0-100.
    /// Some older firmware instead reports a 1.00-1.10 factor here.
    pub bugs: Option<f64>,
    /// Polar coefficient `a`.
    pub polar_a: Option<f64>,
    /// Polar coefficient `b`.
    pub polar_b: Option<f64>,
    /// Polar coefficient `c`.
    pub polar_c: Option<f64>,
    /// Audio volume as a percentage, nominally 0-100.
    pub volume: Option<f64>,
}

impl Lxwp2 {
    pub fn parse(mut fields: FieldsIter<'_>) -> Self {
        Self {
            mac_cready: fields.f64().map(Speed::from_meters_per_second),
            ballast: fields.f64(),
            bugs: fields.f64(),
            polar_a: fields.f64(),
            polar_b: fields.f64(),
            polar_c: fields.f64(),
            volume: fields.f64(),
        }
    }
}

impl TryFrom<&Lxwp2> for Vec<u8> {
    type Error = EncodeError;

    fn try_from(lxwp2: &Lxwp2) -> Result<Self, Self::Error> {
        let mut sentence = SentenceEncoder::new("LXWP2");
        sentence.field(&optional_field(
            lxwp2.mac_cready.map(Speed::as_meters_per_second),
        ));
        sentence.field(&optional_field(lxwp2.ballast));
        sentence.field(&optional_field(lxwp2.bugs));
        sentence.field(&optional_field(lxwp2.polar_a));
        sentence.field(&optional_field(lxwp2.polar_b));
        sentence.field(&optional_field(lxwp2.polar_c));
        sentence.field(&optional_field(lxwp2.volume));
        Ok(sentence.finish())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Message, Step, parse};
    use claims::{assert_none, assert_ok, assert_some_eq};

    fn complete_lxwp2() -> Lxwp2 {
        Lxwp2 {
            mac_cready: Some(Speed::from_meters_per_second(1.5)),
            ballast: Some(1.11),
            bugs: Some(13.0),
            polar_a: Some(2.96),
            polar_b: Some(-3.03),
            polar_c: Some(1.35),
            volume: Some(45.0),
        }
    }

    fn encode_lxwp2_sentence(lxwp2: &Lxwp2) -> String {
        let sentence = assert_ok!(Vec::<u8>::try_from(lxwp2));
        let sentence = assert_ok!(String::from_utf8(sentence));
        assert!(sentence.ends_with("\r\n"));
        sentence
    }

    #[test]
    fn encodes_complete_lxwp2_sentence() {
        insta::assert_snapshot!(encode_lxwp2_sentence(&complete_lxwp2()));
    }

    #[test]
    fn encodes_mac_cready_only_lxwp2_sentence() {
        let lxwp2 = Lxwp2 {
            mac_cready: Some(Speed::from_meters_per_second(0.5)),
            ballast: None,
            bugs: None,
            polar_a: None,
            polar_b: None,
            polar_c: None,
            volume: None,
        };

        insta::assert_snapshot!(encode_lxwp2_sentence(&lxwp2));
    }

    #[test]
    fn parses_encoded_lxwp2_sentence() {
        let expected = complete_lxwp2();
        let sentence = assert_ok!(Vec::<u8>::try_from(&expected));
        let mut input = sentence.as_slice();
        let actual = match parse(&mut input) {
            Step::Frame(Message::Lxwp2(lxwp2)) => lxwp2,
            step => panic!("expected encoded LXWP2 frame, got {step:?}"),
        };

        assert_eq!(actual, expected);
    }

    #[test]
    fn parses_settings_with_a_polar_and_volume() {
        let lxwp2 = Lxwp2::parse(FieldsIter::new(b"1.5,1.11,13,2.96,-3.03,1.35,45"));
        assert_some_eq!(lxwp2.mac_cready, Speed::from_meters_per_second(1.5));
        assert_some_eq!(lxwp2.ballast, 1.11);
        assert_some_eq!(lxwp2.bugs, 13.0);
        assert_some_eq!(lxwp2.polar_a, 2.96);
        assert_some_eq!(lxwp2.polar_b, -3.03);
        assert_some_eq!(lxwp2.polar_c, 1.35);
        assert_some_eq!(lxwp2.volume, 45.0);
    }

    #[test]
    fn keeps_present_fields_when_the_polar_and_volume_are_omitted() {
        // A short form: MacCready, ballast, and bugs only.
        let lxwp2 = Lxwp2::parse(FieldsIter::new(b"1.7,1.1,5"));
        assert_some_eq!(lxwp2.mac_cready, Speed::from_meters_per_second(1.7));
        assert_some_eq!(lxwp2.ballast, 1.1);
        assert_some_eq!(lxwp2.bugs, 5.0);
        assert_none!(lxwp2.polar_a);
        assert_none!(lxwp2.volume);
    }
}
