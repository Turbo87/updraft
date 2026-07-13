//! Types and field helpers shared across the FLARM sentence family.

use crate::field::FieldsIter;

/// The collision-alarm level assessed by FLARM, shared by `PFLAU` and
/// `PFLAA`. The time-to-impact brackets follow the current ICD. Older
/// firmware used slightly different ones.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FlarmAlarmLevel {
    /// `0`: no alarm, also sent for no-alarm traffic information.
    None,
    /// `1`: 15-20 seconds to impact, an Alert Zone alarm, or a traffic
    /// advisory.
    Low,
    /// `2`: 10-15 seconds to impact.
    Important,
    /// `3`: 0-10 seconds to impact.
    Urgent,
    Other(u8),
}

impl FlarmAlarmLevel {
    pub(super) fn from_field(value: Option<u8>) -> Self {
        match value {
            None | Some(0) => Self::None,
            Some(1) => Self::Low,
            Some(2) => Self::Important,
            Some(3) => Self::Urgent,
            Some(other) => Self::Other(other),
        }
    }
}

/// A traffic target ID: six hex digits, optionally followed by `!` and a
/// callsign appended by some devices (e.g. `39103C!FJLKN`).
#[derive(Clone, PartialEq)]
pub struct FlarmId {
    /// The numeric device address. How to interpret it is delivered
    /// separately as a [`FlarmIdType`](super::FlarmIdType).
    pub address: u32,
    /// The callsign appended after `!`, when the device sends one.
    pub callsign: Option<Box<str>>,
}

impl FlarmId {
    /// Parses a `HHHHHH` or `HHHHHH!CALLSIGN` ID field. An ID whose hex
    /// part does not parse, or whose bytes are not valid UTF-8, reads as
    /// absent.
    pub(super) fn parse(field: &[u8]) -> Option<Self> {
        let field = std::str::from_utf8(field).ok()?;
        let (address, callsign) = field
            .split_once('!')
            .map(|(address, callsign)| (address, Some(callsign)))
            .unwrap_or((field, None));

        let address = u32::from_str_radix(address, 16).ok()?;
        let callsign = callsign
            .filter(|callsign| !callsign.is_empty())
            .map(Into::into);

        Some(Self { address, callsign })
    }
}

/// Renders in the wire form (`FlarmId(39103C!FJLKN)`): a decimal address
/// would be unrecognizable, IDs are universally written as hex.
impl std::fmt::Debug for FlarmId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "FlarmId({:06X}", self.address)?;
        if let Some(callsign) = &self.callsign {
            write!(f, "!{callsign}")?;
        }
        f.write_str(")")
    }
}

/// A FLARM `0`/`1` status field. Any other value reads as absent.
pub(super) fn bool_field(fields: &mut FieldsIter<'_>) -> Option<bool> {
    match fields.bytes()? {
        b"0" => Some(false),
        b"1" => Some(true),
        _ => None,
    }
}

/// A hexadecimal field: FLARM sends alarm and aircraft types in hex.
pub(super) fn hex_field(fields: &mut FieldsIter<'_>) -> Option<u8> {
    btoi::btou_radix(fields.bytes()?, 16).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use claims::{assert_none, assert_some, assert_some_eq};

    #[test]
    fn parses_flarm_ids() {
        let id = assert_some!(FlarmId::parse(b"5A77B1"));
        assert_eq!(id.address, 0x5A77B1);
        assert_none!(id.callsign);

        // Some devices append the callsign after a `!`.
        let id = assert_some!(FlarmId::parse(b"39103C!FJLKN"));
        assert_eq!(id.address, 0x39103C);
        assert_some_eq!(id.callsign, "FJLKN".into());

        // A trailing `!` without a callsign reads as no callsign.
        let id = assert_some!(FlarmId::parse(b"39103C!"));
        assert_none!(id.callsign);

        // A non-hex address reads as absent, callsign or not.
        assert_none!(FlarmId::parse(b"XYZ"));
        assert_none!(FlarmId::parse(b"!FJLKN"));

        // A non-UTF-8 callsign rejects the whole ID.
        assert_none!(FlarmId::parse(b"39103C!\xFF"));
    }

    #[test]
    fn debug_renders_ids_in_wire_form() {
        let id = assert_some!(FlarmId::parse(b"39103C!FJLKN"));
        assert_eq!(format!("{id:?}"), "FlarmId(39103C!FJLKN)");

        // Leading zeros are restored for the standard six-digit form.
        let id = assert_some!(FlarmId::parse(b"000FA3"));
        assert_eq!(format!("{id:?}"), "FlarmId(000FA3)");
    }

    #[test]
    fn maps_alarm_levels() {
        assert_eq!(FlarmAlarmLevel::from_field(None), FlarmAlarmLevel::None);
        assert_eq!(FlarmAlarmLevel::from_field(Some(0)), FlarmAlarmLevel::None);
        assert_eq!(FlarmAlarmLevel::from_field(Some(1)), FlarmAlarmLevel::Low);
        assert_eq!(
            FlarmAlarmLevel::from_field(Some(2)),
            FlarmAlarmLevel::Important
        );
        assert_eq!(
            FlarmAlarmLevel::from_field(Some(3)),
            FlarmAlarmLevel::Urgent
        );
        assert_eq!(
            FlarmAlarmLevel::from_field(Some(4)),
            FlarmAlarmLevel::Other(4)
        );
    }

    #[test]
    fn non_boolean_status_fields_are_absent() {
        let mut fields = FieldsIter::new(b"2,true,");
        assert_none!(bool_field(&mut fields));
        assert_none!(bool_field(&mut fields));
        assert_none!(bool_field(&mut fields));
        assert_none!(bool_field(&mut fields));
    }
}
