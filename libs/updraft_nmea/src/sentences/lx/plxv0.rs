use crate::field::{FieldsIter, text};

/// `$PLXV0`: the LXNAV vario setting exchange: read requests, writes, and
/// the device's answers, as a generic name/values pair.
///
/// A read request (`R`) carries just the name. The device answers with the
/// write (`W`) form carrying the current value. Values are kept as text so
/// every setting (`MC`, `BAL`, `QNH`, `POLAR`, `NMEARATE`, ...) can be
/// decoded without modeling each one. Non-UTF-8 bytes are replaced with
/// the Unicode replacement character.
#[derive(Clone, Debug, PartialEq)]
pub struct Plxv0 {
    /// The setting being read or written, e.g. `MC`, `POLAR`, `NMEARATE`.
    pub setting: Option<Box<str>>,
    /// Whether this is a read request or a write / answer.
    pub direction: Option<Plxv0Direction>,
    /// The value fields following the direction, one entry per
    /// comma-separated field (empty fields kept as empty strings). Empty
    /// for a read request.
    pub values: Vec<Box<str>>,
}

impl Plxv0 {
    pub fn parse(mut fields: FieldsIter<'_>) -> Self {
        Self {
            setting: fields.text(),
            direction: fields.bytes().map(Plxv0Direction::from_bytes),
            values: fields.map(text).collect(),
        }
    }
}

/// The direction of a `PLXV0` setting sentence.
#[derive(Clone, Debug, PartialEq)]
pub enum Plxv0Direction {
    /// `R`: request to send the setting's current value.
    Read,
    /// `W`: a write request, or the device's answer to a read.
    Write,
    Other(Box<str>),
}

impl Plxv0Direction {
    fn from_bytes(bytes: &[u8]) -> Self {
        match bytes {
            b"R" => Self::Read,
            b"W" => Self::Write,
            other => Self::Other(text(other)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use claims::assert_some_eq;

    #[test]
    fn parses_a_polar_write() {
        let fields = FieldsIter::new(b"POLAR,W,1.780,-3.030,1.930,30.0,292,600,265,90,LS 7,0,");
        let plxv0 = Plxv0::parse(fields);
        assert_some_eq!(plxv0.setting, "POLAR".into());
        assert_some_eq!(plxv0.direction, Plxv0Direction::Write);
        assert_eq!(plxv0.values.first(), Some(&"1.780".into()));
        assert_eq!(plxv0.values.len(), 11);
    }

    #[test]
    fn keeps_interior_empty_value_fields() {
        // An empty value between two present ones is kept as an empty
        // string so later values stay at their sent position.
        let plxv0 = Plxv0::parse(FieldsIter::new(b"POLAR,W,,1.5"));
        assert_eq!(plxv0.values, vec!["".into(), "1.5".into()]);
    }

    #[test]
    fn parses_a_scalar_write() {
        let plxv0 = Plxv0::parse(FieldsIter::new(b"MC,W,1.5"));
        assert_some_eq!(plxv0.setting, "MC".into());
        assert_some_eq!(plxv0.direction, Plxv0Direction::Write);
        assert_eq!(plxv0.values, vec!["1.5".into()]);
    }

    #[test]
    fn parses_a_read_request_with_no_values() {
        let plxv0 = Plxv0::parse(FieldsIter::new(b"ELEVATION,R"));
        assert_some_eq!(plxv0.setting, "ELEVATION".into());
        assert_some_eq!(plxv0.direction, Plxv0Direction::Read);
        assert_eq!(plxv0.values, Vec::<Box<str>>::new());
    }

    #[test]
    fn keeps_an_unknown_direction_as_text() {
        let plxv0 = Plxv0::parse(FieldsIter::new(b"CONNECTION,S"));
        assert_some_eq!(plxv0.direction, Plxv0Direction::Other("S".into()));
    }
}
