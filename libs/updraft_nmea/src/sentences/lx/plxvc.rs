use crate::field::{FieldsIter, text};

/// `$PLXVC`: the LXNAV command and file-transfer protocol used by Nano
/// loggers and s-varios, decoded as a generic key / type / values triple.
///
/// It carries many exchanges: device `INFO`, `LOGBOOK` and `FLIGHT`
/// download, task `DECL`, `RADIO` and `XPDR` state, each keyed by the
/// first field. Values are kept as text so any exchange can be read
/// without modelling every command. Non-UTF-8 bytes are replaced with the
/// Unicode replacement character.
#[derive(Clone, Debug, PartialEq)]
pub struct Plxvc {
    /// The command key, e.g. `INFO`, `LOGBOOK`, `FLIGHT`, `DECL`, `RADIO`,
    /// `XPDR`.
    pub key: Option<Box<str>>,
    /// Whether this is a request, write, answer, confirmation, or set.
    pub message_type: Option<PlxvcMessageType>,
    /// The value fields following the type, one entry per comma-separated
    /// field (empty fields kept as empty strings).
    pub values: Vec<Box<str>>,
}

impl Plxvc {
    pub fn parse(mut fields: FieldsIter<'_>) -> Self {
        Self {
            key: fields.text(),
            message_type: fields.bytes().map(PlxvcMessageType::from_bytes),
            values: fields.map(text).collect(),
        }
    }
}

/// The kind of a `PLXVC` exchange, from its second field.
#[derive(Clone, Debug, PartialEq)]
pub enum PlxvcMessageType {
    /// `R`: a read request.
    Read,
    /// `W`: a write request.
    Write,
    /// `A`: the device's answer.
    Answer,
    /// `C`: a confirmation (e.g. of a written declaration row).
    Confirm,
    /// `S`: a set request.
    Set,
    Other(Box<str>),
}

impl PlxvcMessageType {
    fn from_bytes(bytes: &[u8]) -> Self {
        match bytes {
            b"R" => Self::Read,
            b"W" => Self::Write,
            b"A" => Self::Answer,
            b"C" => Self::Confirm,
            b"S" => Self::Set,
            other => Self::Other(text(other)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use claims::assert_some_eq;

    #[test]
    fn parses_a_radio_answer() {
        let plxvc = Plxvc::parse(FieldsIter::new(b"RADIO,A,COMM,128800,CELJE"));
        assert_some_eq!(plxvc.key, "RADIO".into());
        assert_some_eq!(plxvc.message_type, PlxvcMessageType::Answer);
        assert_eq!(
            plxvc.values,
            vec!["COMM".into(), "128800".into(), "CELJE".into()]
        );
    }

    #[test]
    fn keeps_interior_empty_value_fields() {
        // An empty value between two present ones is kept as an empty
        // string so later values stay at their sent position.
        let plxvc = Plxvc::parse(FieldsIter::new(b"RADIO,A,COMM,,CELJE"));
        assert_eq!(plxvc.values, vec!["COMM".into(), "".into(), "CELJE".into()]);
    }

    #[test]
    fn parses_an_info_answer() {
        let plxvc = Plxvc::parse(FieldsIter::new(b"INFO,A,LX9000,9.5,May 12 2012,45123"));
        assert_some_eq!(plxvc.key, "INFO".into());
        assert_some_eq!(plxvc.message_type, PlxvcMessageType::Answer);
        assert_eq!(
            plxvc.values,
            vec![
                "LX9000".into(),
                "9.5".into(),
                "May 12 2012".into(),
                "45123".into(),
            ]
        );
    }

    #[test]
    fn maps_message_types() {
        assert_eq!(PlxvcMessageType::from_bytes(b"R"), PlxvcMessageType::Read);
        assert_eq!(PlxvcMessageType::from_bytes(b"W"), PlxvcMessageType::Write);
        assert_eq!(PlxvcMessageType::from_bytes(b"A"), PlxvcMessageType::Answer);
        assert_eq!(
            PlxvcMessageType::from_bytes(b"C"),
            PlxvcMessageType::Confirm
        );
        assert_eq!(PlxvcMessageType::from_bytes(b"S"), PlxvcMessageType::Set);
        assert_eq!(
            PlxvcMessageType::from_bytes(b"Z"),
            PlxvcMessageType::Other("Z".into())
        );
    }
}
