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

#[cfg(test)]
mod tests {
    use super::*;
    use claims::{assert_none, assert_some_eq};

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
    fn a_missing_license_reads_as_absent() {
        // Many devices omit the license field entirely.
        let lxwp1 = Lxwp1::parse(FieldsIter::new(b"V7,12345,1.0,1.0"));
        assert_some_eq!(lxwp1.product, "V7".into());
        assert_none!(lxwp1.license);
    }
}
