use crate::encode::{EncodeError, SentenceEncoder};
use crate::field::FieldsIter;

/// `$LXWP1`: device identification, sent about once a minute.
///
/// Used to recognize which LXNAV product is on the port. Every field is
/// free-form text kept as sent. Non-UTF-8 bytes are replaced with the
/// Unicode replacement character.
#[derive(Clone, Debug, PartialEq)]
pub struct Lxwp1 {
    /// Product / instrument name, e.g. `LX9000`, `V7`, `NANO3`.
    pub product: Option<Box<str>>,
    /// Serial number. Kept as text: some devices report it as a bare
    /// number, others pad or prefix it.
    pub serial: Option<Box<str>>,
    /// Software (firmware) version.
    pub software_version: Option<Box<str>>,
    /// Hardware version.
    pub hardware_version: Option<Box<str>>,
    /// Optional license string some devices append.
    pub license: Option<Box<str>>,
}

impl Lxwp1 {
    pub fn parse(mut fields: FieldsIter<'_>) -> Self {
        Self {
            product: fields.text(),
            serial: fields.text(),
            software_version: fields.text(),
            hardware_version: fields.text(),
            license: fields.text(),
        }
    }
}

impl TryFrom<&Lxwp1> for Vec<u8> {
    type Error = EncodeError;

    fn try_from(lxwp1: &Lxwp1) -> Result<Self, Self::Error> {
        let mut sentence = SentenceEncoder::new("LXWP1");
        sentence.text_field(lxwp1.product.as_deref(), "product")?;
        sentence.text_field(lxwp1.serial.as_deref(), "serial")?;
        sentence.text_field(lxwp1.software_version.as_deref(), "software_version")?;
        sentence.text_field(lxwp1.hardware_version.as_deref(), "hardware_version")?;
        sentence.text_field(lxwp1.license.as_deref(), "license")?;
        Ok(sentence.finish())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Message, Step, parse};
    use claims::{assert_err_eq, assert_none, assert_ok, assert_some_eq};

    #[test]
    fn encodes_complete_lxwp1_sentence() {
        insta::assert_snapshot!(encode_lxwp1_sentence(&complete_lxwp1()));
    }

    #[test]
    fn encodes_lxwp1_sentence_with_empty_identification_fields() {
        let lxwp1 = Lxwp1 {
            product: None,
            serial: None,
            software_version: None,
            hardware_version: None,
            license: None,
        };

        insta::assert_snapshot!(encode_lxwp1_sentence(&lxwp1));
    }

    #[test]
    fn rejects_invalid_lxwp1_text_fields() {
        let mut lxwp1 = complete_lxwp1();
        lxwp1.product = Some("LX,9000".into());
        assert_err_eq!(
            Vec::<u8>::try_from(&lxwp1),
            EncodeError::InvalidField("product")
        );

        let mut lxwp1 = complete_lxwp1();
        lxwp1.serial = Some("45*123".into());
        assert_err_eq!(
            Vec::<u8>::try_from(&lxwp1),
            EncodeError::InvalidField("serial")
        );

        let mut lxwp1 = complete_lxwp1();
        lxwp1.software_version = Some("9\r5".into());
        assert_err_eq!(
            Vec::<u8>::try_from(&lxwp1),
            EncodeError::InvalidField("software_version")
        );

        let mut lxwp1 = complete_lxwp1();
        lxwp1.hardware_version = Some("2\n0".into());
        assert_err_eq!(
            Vec::<u8>::try_from(&lxwp1),
            EncodeError::InvalidField("hardware_version")
        );

        let mut lxwp1 = complete_lxwp1();
        lxwp1.license = Some("LIZENZÄ".into());
        assert_err_eq!(
            Vec::<u8>::try_from(&lxwp1),
            EncodeError::InvalidField("license")
        );
    }

    #[test]
    fn parses_encoded_lxwp1_sentence() {
        let expected = complete_lxwp1();
        let sentence = assert_ok!(Vec::<u8>::try_from(&expected));
        let mut input = sentence.as_slice();

        let actual = match parse(&mut input) {
            Step::Frame(Message::Lxwp1(lxwp1)) => lxwp1,
            step => panic!("expected encoded LXWP1 frame, got {step:?}"),
        };

        assert_eq!(actual, expected);
    }

    fn complete_lxwp1() -> Lxwp1 {
        Lxwp1 {
            product: Some("LX9000".into()),
            serial: Some("45123".into()),
            software_version: Some("9.5".into()),
            hardware_version: Some("2.0".into()),
            license: Some("ABC123".into()),
        }
    }

    fn encode_lxwp1_sentence(lxwp1: &Lxwp1) -> String {
        let sentence = assert_ok!(Vec::<u8>::try_from(lxwp1));
        let sentence = assert_ok!(String::from_utf8(sentence));
        assert!(sentence.ends_with("\r\n"));
        sentence
    }

    #[test]
    fn parses_a_full_identification() {
        let lxwp1 = Lxwp1::parse(FieldsIter::new(b"LX9000,45123,9.5,2.0,ABC123,"));
        assert_some_eq!(lxwp1.product, "LX9000".into());
        assert_some_eq!(lxwp1.serial, "45123".into());
        assert_some_eq!(lxwp1.software_version, "9.5".into());
        assert_some_eq!(lxwp1.hardware_version, "2.0".into());
        assert_some_eq!(lxwp1.license, "ABC123".into());
    }

    #[test]
    fn missing_license_reads_as_absent() {
        // Many devices omit the license field entirely.
        let lxwp1 = Lxwp1::parse(FieldsIter::new(b"V7,12345,1.0,1.0"));
        assert_some_eq!(lxwp1.product, "V7".into());
        assert_none!(lxwp1.license);
    }
}
