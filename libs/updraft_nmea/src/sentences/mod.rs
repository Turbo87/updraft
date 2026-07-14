//! Decoders for each NMEA sentence family.

mod cambridge;
mod flarm;
mod garmin;
mod gnss;
mod lx;
mod openvario;

pub use cambridge::{CaiG, CaiSetting, CaiW};
pub use flarm::{
    FlarmAircraftType, FlarmAlarmLevel, FlarmId, FlarmIdType, FlarmSource, Pflaa, Pflac,
    PflacQueryType, Pflau, PflauAlarmType, PflauGpsStatus,
};
pub use garmin::{Pgrmz, PgrmzFixDimension};
pub use gnss::{
    Gga, GgaFixQuality, Gsa, GsaFixType, GsaSelectionMode, PositioningMode, Rmc, RmcStatus,
};
pub use lx::{
    Lxwp0, Lxwp1, Lxwp2, Lxwp3, Lxwp3SpeedCommandMode, Lxwp3SwitchMode, Plxv0, Plxv0Direction,
    Plxvc, PlxvcMessageType, Plxvf, Plxvs, PlxvsMode, Plxvtarg,
};
pub use openvario::{Pov, PovDatum};

use crate::field::FieldsIter;
use crate::message::{Message, Talker, Unknown};

/// Routes a complete checksum-stripped sentence to the matching family,
/// falling back to [`Message::Unknown`].
pub fn parse_sentence(sentence: &[u8]) -> Message {
    let mut fields = FieldsIter::new(sentence);
    let address = fields.next().unwrap_or_default();

    match address {
        b"!g" => return Message::CaiG(CaiG::parse(fields)),
        b"!w" => return Message::CaiW(CaiW::parse(fields)),
        b"$PGRMZ" => return Message::Pgrmz(Pgrmz::parse(fields)),
        b"$PFLAU" => return Message::Pflau(Pflau::parse(fields)),
        b"$PFLAA" => return Message::Pflaa(Pflaa::parse(fields)),
        b"$PFLAC" => return Message::Pflac(Pflac::parse(fields)),
        b"$LXWP0" => return Message::Lxwp0(Lxwp0::parse(fields)),
        b"$LXWP1" => return Message::Lxwp1(Lxwp1::parse(fields)),
        b"$LXWP2" => return Message::Lxwp2(Lxwp2::parse(fields)),
        b"$LXWP3" => return Message::Lxwp3(Lxwp3::parse(fields)),
        b"$PLXVF" => return Message::Plxvf(Plxvf::parse(fields)),
        b"$PLXVS" => return Message::Plxvs(Plxvs::parse(fields)),
        b"$PLXV0" => return Message::Plxv0(Plxv0::parse(fields)),
        b"$PLXVC" => return Message::Plxvc(Plxvc::parse(fields)),
        b"$PLXVTARG" => return Message::Plxvtarg(Plxvtarg::parse(fields)),
        b"$POV" => return Message::Pov(Pov::parse(fields)),
        _ => {}
    }

    let Some((code, sentence_type)) = address.strip_prefix(b"$").and_then(split_standard_address)
    else {
        return Message::Unknown(Unknown::from_bytes(sentence));
    };

    let talker = Talker::from_code(code);
    match sentence_type {
        b"GGA" => Message::Gga(Gga::parse(talker, fields)),
        b"RMC" => Message::Rmc(Rmc::parse(talker, fields)),
        b"GSA" => Message::Gsa(Gsa::parse(talker, fields)),
        _ => Message::Unknown(Unknown::from_bytes(sentence)),
    }
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
