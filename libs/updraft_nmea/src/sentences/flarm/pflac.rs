use crate::field::{field, text};

/// FLARM configuration exchange (`$PFLAC`): read requests, set requests,
/// and the device's answers, as a generic item/values pair.
#[derive(Clone, Debug, PartialEq)]
pub struct Pflac {
    pub query_type: Option<PflacQueryType>,
    /// The configuration item being read or set (e.g. `NMEAOUT`), or
    /// `ERROR` in the answer to a failed request. Non-UTF-8 bytes are
    /// replaced with the Unicode replacement character.
    pub item: Option<Box<str>>,
    /// The value fields following the item, one entry per comma-separated
    /// field (empty fields kept as empty strings). Empty for a read
    /// request. Non-UTF-8 bytes are replaced with the Unicode replacement
    /// character.
    pub values: Vec<Box<str>>,
}

impl Pflac {
    pub fn parse(fields: &[&[u8]]) -> Self {
        Self {
            query_type: field(fields, 0).map(PflacQueryType::from_bytes),
            item: field(fields, 1).map(text),
            values: fields
                .get(2..)
                .unwrap_or_default()
                .iter()
                .copied()
                .map(text)
                .collect(),
        }
    }
}

/// The direction of a `PFLAC` configuration sentence.
#[derive(Clone, Debug, PartialEq)]
pub enum PflacQueryType {
    /// `R`: request to send the item's current value.
    Read,
    /// `S`: request to set the item to the given value.
    Set,
    /// `A`: the device's answer to a read or set request.
    Answer,
    Other(Box<str>),
}

impl PflacQueryType {
    fn from_bytes(bytes: &[u8]) -> Self {
        match bytes {
            b"R" => Self::Read,
            b"S" => Self::Set,
            b"A" => Self::Answer,
            other => Self::Other(text(other)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use claims::assert_some_eq;

    #[test]
    fn parses_a_read_request() {
        let fields: [&[u8]; 2] = [b"R", b"NMEAOUT"];
        let pflac = Pflac::parse(&fields);
        assert_some_eq!(pflac.query_type, PflacQueryType::Read);
        assert_some_eq!(pflac.item, "NMEAOUT".into());
        assert_eq!(pflac.values, Vec::<Box<str>>::new());
    }

    #[test]
    fn parses_a_set_request() {
        let fields: [&[u8]; 3] = [b"S", b"BAUD", b"5"];
        let pflac = Pflac::parse(&fields);
        assert_some_eq!(pflac.query_type, PflacQueryType::Set);
        assert_some_eq!(pflac.item, "BAUD".into());
        assert_eq!(pflac.values, vec!["5".into()]);
    }

    #[test]
    fn keeps_every_value_of_a_multi_value_answer() {
        let fields: [&[u8]; 4] = [b"A", b"RADIOID", b"1", b"A832ED"];
        let pflac = Pflac::parse(&fields);
        assert_some_eq!(pflac.query_type, PflacQueryType::Answer);
        assert_some_eq!(pflac.item, "RADIOID".into());
        assert_eq!(pflac.values, vec!["1".into(), "A832ED".into()]);
    }

    #[test]
    fn keeps_empty_value_fields() {
        // Unlike the item field, an empty value field is kept as an empty
        // string so the positions of the following values are preserved.
        let fields: [&[u8]; 4] = [b"S", b"ID", b"", b"foo"];
        let pflac = Pflac::parse(&fields);
        assert_eq!(pflac.values, vec!["".into(), "foo".into()]);
    }

    #[test]
    fn parses_an_error_answer() {
        let fields: [&[u8]; 2] = [b"A", b"ERROR"];
        let pflac = Pflac::parse(&fields);
        assert_some_eq!(pflac.query_type, PflacQueryType::Answer);
        assert_some_eq!(pflac.item, "ERROR".into());
        assert_eq!(pflac.values, Vec::<Box<str>>::new());
    }

    #[test]
    fn keeps_an_unknown_query_type_as_text() {
        // The ICD's error example: `$PFLAC,HELLO,GLIDER_PILOTS`.
        let fields: [&[u8]; 2] = [b"HELLO", b"GLIDER_PILOTS"];
        let pflac = Pflac::parse(&fields);
        assert_some_eq!(pflac.query_type, PflacQueryType::Other("HELLO".into()));
        assert_some_eq!(pflac.item, "GLIDER_PILOTS".into());
    }
}
