//! The typed messages produced by [`parse`](crate::parse) and the values
//! shared across sentence families.

use crate::field::text;
use crate::sentences::{Gga, Gsa, Pflaa, Pflac, Pflau, Pgrmz, Rmc};

/// A single decoded NMEA sentence, faithful to the wire.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum Message {
    /// GNSS fix data (`**GGA`).
    Gga(Gga),
    /// Recommended minimum GNSS data (`**RMC`).
    Rmc(Rmc),
    /// GNSS DOP and active satellites (`**GSA`).
    Gsa(Gsa),
    /// Garmin barometric altitude (`PGRMZ`).
    Pgrmz(Pgrmz),
    /// FLARM heartbeat, status, and basic alarms (`PFLAU`).
    Pflau(Pflau),
    /// FLARM data on one proximate aircraft (`PFLAA`).
    Pflaa(Pflaa),
    /// FLARM configuration read/set/answer (`PFLAC`).
    Pflac(Pflac),
    /// A well-formed sentence of a type this crate does not decode.
    Unknown(Unknown),
}

/// A well-formed but unrecognised sentence, kept so it can be counted or
/// logged rather than silently dropped.
#[derive(Clone, Debug, PartialEq)]
pub struct Unknown {
    /// The sentence body: everything after the start marker, up to the
    /// checksum or, for a checksum-less sentence, the terminating newline.
    /// Non-UTF-8 bytes are replaced with the Unicode replacement character.
    pub sentence: Box<str>,
}

impl Unknown {
    pub fn from_bytes(body: &[u8]) -> Self {
        Self {
            sentence: text(body),
        }
    }
}

/// The talker that emitted a standard GNSS sentence, taken from the two
/// characters before the three-letter sentence type.
#[derive(Clone, Debug, PartialEq)]
pub enum Talker {
    /// GPS (`GP`).
    Gps,
    /// GLONASS (`GL`).
    Glonass,
    /// Galileo (`GA`).
    Galileo,
    /// BeiDou (`GB`, or the nonstandard `BD` alias some devices emit).
    BeiDou,
    /// QZSS (`GQ`).
    Qzss,
    /// A combined multi-constellation solution (`GN`).
    Combined,
    /// Any other talker code, kept as text.
    Other(Box<str>),
}

impl Talker {
    pub fn from_code(code: &[u8]) -> Self {
        match code {
            b"GP" => Self::Gps,
            b"GL" => Self::Glonass,
            b"GA" => Self::Galileo,
            b"GB" | b"BD" => Self::BeiDou,
            b"GQ" => Self::Qzss,
            b"GN" => Self::Combined,
            _ => Self::Other(text(code)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_standard_and_aliased_talker_codes() {
        assert_eq!(Talker::from_code(b"GP"), Talker::Gps);
        assert_eq!(Talker::from_code(b"GL"), Talker::Glonass);
        assert_eq!(Talker::from_code(b"GA"), Talker::Galileo);
        assert_eq!(Talker::from_code(b"GB"), Talker::BeiDou);
        // `BD` is a nonstandard BeiDou alias some receivers emit.
        assert_eq!(Talker::from_code(b"BD"), Talker::BeiDou);
        assert_eq!(Talker::from_code(b"GQ"), Talker::Qzss);
        assert_eq!(Talker::from_code(b"GN"), Talker::Combined);
        assert_eq!(Talker::from_code(b"AI"), Talker::Other("AI".into()));
    }
}
