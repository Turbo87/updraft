//! `PFLAU` — FLARM status and the single most relevant threat.

use updraft_units::{Angle, Length};

use crate::error::ParseError;
use crate::fields::Fields;
use crate::flarm::AlarmLevel;
use crate::scalars;

/// A parsed `PFLAU` sentence.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Pflau {
    /// Number of devices with which a connection is currently received.
    pub rx: u8,
    /// Whether this device is currently transmitting.
    pub tx: bool,
    /// GPS reception status.
    pub gps: GpsStatus,
    /// Whether the device power supply is within range.
    pub power_ok: bool,
    /// The highest alarm level among all tracked aircraft.
    pub alarm_level: AlarmLevel,
    /// Bearing to the most relevant threat, relative to track
    /// (east-positive, `-180..180`). `None` when there is no threat.
    pub relative_bearing: Option<Angle>,
    /// The kind of the most relevant threat.
    pub alarm_type: AlarmType,
    /// Relative height of the threat above own position. `None` when there
    /// is no threat.
    pub relative_vertical: Option<Length>,
    /// Horizontal distance to the threat. `None` when there is no threat.
    pub relative_distance: Option<Length>,
    /// FLARM/ICAO id of the threat, if any.
    pub id: Option<u32>,
}

/// The `PFLAU` GPS status (field 3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum GpsStatus {
    /// `0` — no GPS reception.
    NoFix,
    /// `1` — 3D fix on the ground.
    OnGround,
    /// `2` — 3D fix while airborne.
    Airborne,
}

impl GpsStatus {
    fn parse(field: &str) -> Result<Self, ParseError> {
        match field {
            "0" => Ok(Self::NoFix),
            "1" => Ok(Self::OnGround),
            "2" => Ok(Self::Airborne),
            _ => Err(ParseError::InvalidField),
        }
    }
}

/// The `PFLAU` alarm type (field 7), given in hexadecimal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum AlarmType {
    /// `0` — no aircraft within range or no alarm.
    None,
    /// `2` — aircraft alarm.
    Aircraft,
    /// `3` — obstacle or terrain alarm.
    Obstacle,
    /// `4` — alert zone (e.g. skydiver drop zone).
    AlertZone,
    /// Any other, forward-compatible alarm-type code.
    Other(u8),
}

impl AlarmType {
    fn parse(field: &str) -> Result<Self, ParseError> {
        let code = u8::from_str_radix(field, 16).map_err(|_| ParseError::InvalidNumber)?;
        Ok(match code {
            0 => Self::None,
            2 => Self::Aircraft,
            3 => Self::Obstacle,
            4 => Self::AlertZone,
            other => Self::Other(other),
        })
    }
}

fn parse_bool(field: &str) -> Result<bool, ParseError> {
    match field {
        "1" => Ok(true),
        "0" => Ok(false),
        _ => Err(ParseError::InvalidField),
    }
}

pub(crate) fn parse(mut fields: Fields<'_>) -> Result<Pflau, ParseError> {
    let rx = scalars::u8(fields.next_required()?)?;
    let tx = parse_bool(fields.next_required()?)?;
    let gps = GpsStatus::parse(fields.next_required()?)?;
    let power_ok = parse_bool(fields.next_required()?)?;
    let alarm_level = AlarmLevel::parse(fields.next_required()?)?;
    let relative_bearing = scalars::opt_f64(fields.next_required()?)?.map(Angle::from_degrees);
    let alarm_type = AlarmType::parse(fields.next_required()?)?;
    let relative_vertical = scalars::opt_f64(fields.next_required()?)?.map(Length::from_meters);
    let relative_distance = scalars::opt_f64(fields.next_required()?)?.map(Length::from_meters);
    let id = scalars::opt(fields.next_field().unwrap_or(""), scalars::hex_u32)?;

    Ok(Pflau {
        rx,
        tx,
        gps,
        power_ok,
        alarm_level,
        relative_bearing,
        alarm_type,
        relative_vertical,
        relative_distance,
        id,
    })
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;

    use super::*;
    use crate::{ParseResult, parse};

    fn pflau(line: &str) -> Pflau {
        match parse(line).unwrap() {
            ParseResult::Pflau(pflau) => pflau,
            other => panic!("expected PFLAU, got {other:?}"),
        }
    }

    #[test]
    fn airborne_no_threat_from_flight_corpus() {
        let pflau = pflau("$PFLAU,11,1,2,1,0,,0,,,*7C");
        assert_eq!(pflau.rx, 11);
        assert!(pflau.tx);
        assert_eq!(pflau.gps, GpsStatus::Airborne);
        assert!(pflau.power_ok);
        assert_eq!(pflau.alarm_level, AlarmLevel::None);
        assert_eq!(pflau.alarm_type, AlarmType::None);
        assert_eq!(pflau.relative_bearing, None);
        assert_eq!(pflau.relative_vertical, None);
        assert_eq!(pflau.relative_distance, None);
        assert_eq!(pflau.id, None);
    }

    #[test]
    fn active_threat_populates_relative_geometry() {
        let pflau = pflau("$PFLAU,3,1,2,1,2,-30,2,-120,1500,DDA85C*74");
        assert_eq!(pflau.alarm_level, AlarmLevel::Important);
        assert_eq!(pflau.alarm_type, AlarmType::Aircraft);
        assert_relative_eq!(pflau.relative_bearing.unwrap().as_degrees(), -30.0);
        assert_relative_eq!(pflau.relative_vertical.unwrap().as_meters(), -120.0);
        assert_relative_eq!(pflau.relative_distance.unwrap().as_meters(), 1500.0);
        assert_eq!(pflau.id, Some(0xDDA85C));
    }

    #[test]
    fn no_gps_no_power() {
        let pflau = pflau("$PFLAU,0,0,0,1,0,,0,,,*4F");
        assert_eq!(pflau.gps, GpsStatus::NoFix);
        assert!(!pflau.tx);
    }
}
