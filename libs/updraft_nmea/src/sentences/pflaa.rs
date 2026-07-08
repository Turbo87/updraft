//! `PFLAA` — a single FLARM traffic target: relative position, movement,
//! and identity.

use updraft_units::{Angle, Length, Speed};

use crate::error::ParseError;
use crate::fields::Fields;
use crate::flarm::AlarmLevel;
use crate::scalars;

/// A parsed `PFLAA` sentence describing one tracked target.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Pflaa {
    /// Alarm level this target raises.
    pub alarm_level: AlarmLevel,
    /// Distance north of own position (negative = south). `None` for a
    /// non-directional target that reports no horizontal bearing.
    pub relative_north: Option<Length>,
    /// Distance east of own position (negative = west). `None` for a
    /// non-directional target that reports no horizontal bearing.
    pub relative_east: Option<Length>,
    /// Height above own position (negative = below), if reported.
    pub relative_vertical: Option<Length>,
    /// How the target's [`id`](Self::id) should be interpreted.
    pub id_type: IdType,
    /// The target's 24-bit id, if present.
    pub id: Option<u32>,
    /// A callsign/registration label appended to the id field as
    /// `id!LABEL`. This is a nonstandard augmentation seen in OGN-derived
    /// streams, not part of the FLARM specification.
    pub id_label: Option<String>,
    /// True track over ground, if the target is tracked.
    pub track: Option<Angle>,
    /// Turn rate in degrees per second, if reported.
    pub turn_rate: Option<f64>,
    /// Ground speed, if the target is tracked.
    pub ground_speed: Option<Speed>,
    /// Climb rate (positive up), if reported.
    pub climb_rate: Option<Speed>,
    /// The target's aircraft type.
    pub aircraft_type: AircraftType,
}

/// How a target's id (`PFLAA` field 5) should be interpreted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum IdType {
    /// `0` — random / anonymous (stealth) id.
    Random,
    /// `1` — official ICAO 24-bit address.
    Icao,
    /// `2` — stable FLARM id.
    Flarm,
    /// Any other, forward-compatible id-type code.
    Other(u8),
}

impl IdType {
    fn parse(field: &str) -> Result<Self, ParseError> {
        Ok(match scalars::u8(field)? {
            0 => Self::Random,
            1 => Self::Icao,
            2 => Self::Flarm,
            other => Self::Other(other),
        })
    }
}

/// The FLARM aircraft-type code (`PFLAA` field 11), given in hexadecimal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum AircraftType {
    /// `0` — unknown.
    Unknown,
    /// `1` — glider or motor glider.
    Glider,
    /// `2` — tow / tug plane.
    TowPlane,
    /// `3` — helicopter / rotorcraft.
    Helicopter,
    /// `4` — parachute / skydiver.
    Parachute,
    /// `5` — drop plane for skydivers.
    DropPlane,
    /// `6` — hang glider.
    HangGlider,
    /// `7` — paraglider.
    Paraglider,
    /// `8` — powered aircraft.
    PoweredAircraft,
    /// `9` — jet aircraft.
    JetAircraft,
    /// `A` — flying saucer (UFO).
    Ufo,
    /// `B` — balloon.
    Balloon,
    /// `C` — airship.
    Airship,
    /// `D` — unmanned aerial vehicle (drone).
    Drone,
    /// `F` — static obstacle.
    StaticObject,
    /// Any other, forward-compatible aircraft-type code.
    Other(u8),
}

impl AircraftType {
    fn parse(field: &str) -> Result<Self, ParseError> {
        let code = u8::from_str_radix(field, 16).map_err(|_| ParseError::InvalidNumber)?;
        Ok(match code {
            0x0 => Self::Unknown,
            0x1 => Self::Glider,
            0x2 => Self::TowPlane,
            0x3 => Self::Helicopter,
            0x4 => Self::Parachute,
            0x5 => Self::DropPlane,
            0x6 => Self::HangGlider,
            0x7 => Self::Paraglider,
            0x8 => Self::PoweredAircraft,
            0x9 => Self::JetAircraft,
            0xA => Self::Ufo,
            0xB => Self::Balloon,
            0xC => Self::Airship,
            0xD => Self::Drone,
            0xF => Self::StaticObject,
            other => Self::Other(other),
        })
    }
}

/// Split a `PFLAA` id field into its hex id and any appended `!LABEL`.
fn parse_id(field: &str) -> Result<(Option<u32>, Option<String>), ParseError> {
    let (hex, label) = field.split_once('!').unwrap_or((field, ""));
    let id = if hex.is_empty() {
        None
    } else {
        Some(scalars::hex_u32(hex)?)
    };
    let label = (!label.is_empty()).then(|| label.to_owned());
    Ok((id, label))
}

pub(crate) fn parse(mut fields: Fields<'_>) -> Result<Pflaa, ParseError> {
    let alarm_level = AlarmLevel::parse(fields.next_required()?)?;
    let relative_north = scalars::opt_f64(fields.next_required()?)?.map(Length::from_meters);
    let relative_east = scalars::opt_f64(fields.next_required()?)?.map(Length::from_meters);
    let relative_vertical = scalars::opt_f64(fields.next_required()?)?.map(Length::from_meters);
    let id_type = IdType::parse(fields.next_required()?)?;
    let (id, id_label) = parse_id(fields.next_required()?)?;
    let track = scalars::opt_f64(fields.next_required()?)?.map(Angle::from_degrees);
    let turn_rate = scalars::opt_f64(fields.next_required()?)?;
    let ground_speed =
        scalars::opt_f64(fields.next_required()?)?.map(Speed::from_meters_per_second);
    let climb_rate = scalars::opt_f64(fields.next_required()?)?.map(Speed::from_meters_per_second);
    let aircraft_type = AircraftType::parse(fields.next_required()?)?;
    // Optional trailing fields (no-track flag, source, RSSI) are ignored.

    Ok(Pflaa {
        alarm_level,
        relative_north,
        relative_east,
        relative_vertical,
        id_type,
        id,
        id_label,
        track,
        turn_rate,
        ground_speed,
        climb_rate,
        aircraft_type,
    })
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;

    use super::*;
    use crate::{ParseResult, parse};

    fn pflaa(line: &str) -> Pflaa {
        match parse(line).unwrap() {
            ParseResult::Pflaa(pflaa) => pflaa,
            other => panic!("expected PFLAA, got {other:?}"),
        }
    }

    #[test]
    fn target_with_ogn_callsign_from_flight_corpus() {
        let pflaa = pflaa("$PFLAA,0,-1540,-1020,-1126,1,39103C!FJLKN,93,0,33,4.9,8*63");
        assert_eq!(pflaa.alarm_level, AlarmLevel::None);
        assert_relative_eq!(pflaa.relative_north.unwrap().as_meters(), -1540.0);
        assert_relative_eq!(pflaa.relative_east.unwrap().as_meters(), -1020.0);
        assert_relative_eq!(pflaa.relative_vertical.unwrap().as_meters(), -1126.0);
        assert_eq!(pflaa.id_type, IdType::Icao);
        assert_eq!(pflaa.id, Some(0x39103C));
        assert_eq!(pflaa.id_label.as_deref(), Some("FJLKN"));
        assert_relative_eq!(pflaa.track.unwrap().as_degrees(), 93.0);
        assert_relative_eq!(pflaa.ground_speed.unwrap().as_meters_per_second(), 33.0);
        assert_relative_eq!(pflaa.climb_rate.unwrap().as_meters_per_second(), 4.9);
        assert_eq!(pflaa.aircraft_type, AircraftType::PoweredAircraft);
    }

    #[test]
    fn plain_hex_id_without_label() {
        let pflaa = pflaa("$PFLAA,0,-40770,-41860,8108,1,392AEB,101,0,233,0.0,0*2B");
        assert_eq!(pflaa.id, Some(0x392AEB));
        assert_eq!(pflaa.id_label, None);
        assert_eq!(pflaa.aircraft_type, AircraftType::Unknown);
    }

    #[test]
    fn descending_jet_target() {
        // A Ryanair ADS-B target relayed via FLARM/OGN: jet, sinking.
        let pflaa = pflaa("$PFLAA,0,54477,-1026,3058,1,4D22BC!RYR71VG,26,0,170,-4.9,9*4F");
        assert_eq!(pflaa.aircraft_type, AircraftType::JetAircraft);
        assert_eq!(pflaa.id_label.as_deref(), Some("RYR71VG"));
        assert_relative_eq!(pflaa.climb_rate.unwrap().as_meters_per_second(), -4.9);
    }

    /// Non-directional target from the `flight_1` corpus: the relative-east
    /// field (and the movement fields) are empty.
    #[test]
    fn non_directional_target_has_no_east() {
        let pflaa = pflaa("$PFLAA,0,13895,,-413,1,39299C!FJKLR,,,,0.0,8*4F");
        assert_relative_eq!(pflaa.relative_north.unwrap().as_meters(), 13895.0);
        assert_eq!(pflaa.relative_east, None);
        assert_relative_eq!(pflaa.relative_vertical.unwrap().as_meters(), -413.0);
        assert_eq!(pflaa.track, None);
        assert_relative_eq!(pflaa.climb_rate.unwrap().as_meters_per_second(), 0.0);
    }

    #[test]
    fn stealth_target_without_movement_fields() {
        let pflaa = pflaa("$PFLAA,0,1000,2000,300,2,ABC123,,,,,1,1*18");
        assert_eq!(pflaa.id_type, IdType::Flarm);
        assert_eq!(pflaa.id, Some(0xABC123));
        assert_eq!(pflaa.track, None);
        assert_eq!(pflaa.turn_rate, None);
        assert_eq!(pflaa.ground_speed, None);
        assert_eq!(pflaa.climb_rate, None);
        assert_eq!(pflaa.aircraft_type, AircraftType::Glider);
    }
}
