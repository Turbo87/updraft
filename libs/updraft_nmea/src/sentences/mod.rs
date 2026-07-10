//! Decoders for each NMEA sentence family.

mod flarm;
mod garmin;
mod gnss;
mod lx;

pub use flarm::{
    FlarmAircraftType, FlarmAlarmLevel, FlarmId, FlarmIdType, FlarmSource, Pflaa, Pflac,
    PflacQueryType, Pflau, PflauAlarmType, PflauGpsStatus,
};
pub use garmin::{Pgrmz, PgrmzFixDimension};
pub use gnss::{
    Gga, GgaFixQuality, Gsa, GsaFixType, GsaSelectionMode, PositioningMode, Rmc, RmcStatus,
};
pub use lx::{Lxwp0, Lxwp1, Lxwp2, Lxwp3, Lxwp3SpeedCommandMode, Lxwp3SwitchMode};

use crate::message::{Message, Talker, Unknown};

/// Routes a sentence body (everything after the start marker, with the
/// checksum stripped) to the matching family, falling back to
/// [`Message::Unknown`].
pub fn parse_body(body: &[u8]) -> Message {
    let (address, rest) = split_once(body, b',').unwrap_or((body, b""));

    match address {
        b"PGRMZ" => return Message::Pgrmz(Pgrmz::parse(&fields(rest))),
        b"PFLAU" => return Message::Pflau(Pflau::parse(&fields(rest))),
        b"PFLAA" => return Message::Pflaa(Pflaa::parse(&fields(rest))),
        b"PFLAC" => return Message::Pflac(Pflac::parse(&fields(rest))),
        b"LXWP0" => return Message::Lxwp0(Lxwp0::parse(&fields(rest))),
        b"LXWP1" => return Message::Lxwp1(Lxwp1::parse(&fields(rest))),
        b"LXWP2" => return Message::Lxwp2(Lxwp2::parse(&fields(rest))),
        b"LXWP3" => return Message::Lxwp3(Lxwp3::parse(&fields(rest))),
        _ => {}
    }

    let Some((code, sentence_type)) = split_standard_address(address) else {
        return Message::Unknown(Unknown::from_bytes(body));
    };

    let talker = Talker::from_code(code);
    match sentence_type {
        b"GGA" => Message::Gga(Gga::parse(talker, &fields(rest))),
        b"RMC" => Message::Rmc(Rmc::parse(talker, &fields(rest))),
        b"GSA" => Message::Gsa(Gsa::parse(talker, &fields(rest))),
        _ => Message::Unknown(Unknown::from_bytes(body)),
    }
}

/// Splits `body` at the first `separator`, dropping it, or `None` if it is
/// absent.
fn split_once(body: &[u8], separator: u8) -> Option<(&[u8], &[u8])> {
    let index = body.iter().position(|&byte| byte == separator)?;
    Some((&body[..index], &body[index + 1..]))
}

/// Splits the comma-separated argument list of a sentence into fields.
fn fields(args: &[u8]) -> Vec<&[u8]> {
    args.split(|&byte| byte == b',').collect()
}

/// Splits a standard (non-proprietary) address into its talker code and
/// three-letter sentence type, or `None` for proprietary (`P…`) or
/// too-short addresses.
fn split_standard_address(address: &[u8]) -> Option<(&[u8], &[u8])> {
    if address.first() == Some(&b'P') {
        return None;
    }
    let split = address.len().checked_sub(3)?;
    Some(address.split_at(split))
}
