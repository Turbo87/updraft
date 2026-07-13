use super::common::{FlarmAlarmLevel, FlarmId, bool_field, parse_hex};
use crate::field::FieldsIter;
use updraft_units::{Angle, Length};

/// `$PFLAU`: FLARM heartbeat, status, and the most relevant current threat,
/// sent about once per second.
///
/// Consumers are meant to derive collision warnings from this sentence
/// alone. `PFLAA` only adds detail.
#[derive(Clone, Debug, PartialEq)]
pub struct Pflau {
    /// Number of devices with unique IDs currently received.
    pub rx_count: Option<u8>,
    /// Radio transmission status: `true` for OK.
    pub tx_ok: Option<bool>,
    pub gps_status: PflauGpsStatus,
    /// Power status: `true` for OK, `false` for under- or over-voltage.
    pub power_ok: Option<bool>,
    pub alarm_level: FlarmAlarmLevel,
    /// Bearing to the threat, relative to own true ground track, clockwise
    /// positive. Absent for non-directional targets or when no aircraft is
    /// within range, and `0` for obstacle and Alert Zone alarms.
    pub relative_bearing: Option<Angle>,
    pub alarm_type: PflauAlarmType,
    /// Vertical separation of the threat above own position, negative when
    /// it is lower. Absent when no aircraft is within range.
    pub relative_vertical: Option<Length>,
    /// Horizontal distance to the threat, estimated from signal strength
    /// for non-directional targets. Absent when no aircraft is within
    /// range and no alarm is active.
    pub relative_distance: Option<Length>,
    /// ID of the threat, omitted by old protocol versions and absent when
    /// no aircraft is within range.
    pub id: Option<FlarmId>,
}

impl Pflau {
    pub fn parse(mut fields: FieldsIter<'_>) -> Self {
        Self {
            rx_count: fields.u8(),
            tx_ok: bool_field(&mut fields),
            gps_status: fields
                .bytes()
                .map(PflauGpsStatus::from_field)
                .unwrap_or_default(),
            power_ok: bool_field(&mut fields),
            alarm_level: fields
                .bytes()
                .map(FlarmAlarmLevel::from_field)
                .unwrap_or_default(),
            relative_bearing: fields.f64().map(Angle::from_degrees),
            alarm_type: fields
                .bytes()
                .map(PflauAlarmType::from_field)
                .unwrap_or_default(),
            relative_vertical: fields.f64().map(Length::from_meters),
            relative_distance: fields.f64().map(Length::from_meters),
            id: fields.bytes().and_then(FlarmId::parse),
        }
    }
}

/// The GPS status reported in a `PFLAU` sentence. Without a fix (`NoFix`)
/// the device cannot generate warnings.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum PflauGpsStatus {
    /// `0`: no GPS reception.
    #[default]
    NoFix,
    /// `1`: 3D fix, not airborne.
    OnGround,
    /// `2`: 3D fix, airborne.
    Airborne,
    Other(u8),
}

impl PflauGpsStatus {
    fn from_field(field: &[u8]) -> Self {
        match field {
            b"0" => Self::NoFix,
            b"1" => Self::OnGround,
            b"2" => Self::Airborne,
            field => btoi::btou(field).ok().map(Self::Other).unwrap_or_default(),
        }
    }
}

/// The kind of threat behind a `PFLAU` alarm. Transmitted as a
/// hexadecimal value.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum PflauAlarmType {
    /// `0`: no aircraft within range, or no-alarm traffic information.
    #[default]
    None,
    /// `2`: aircraft collision alarm.
    Aircraft,
    /// `3`: obstacle alarm (on old protocol versions also Alert Zones).
    Obstacle,
    /// `4`: traffic advisory, sent once when an aircraft first comes close.
    TrafficAdvisory,
    /// `10`-`FF`: Alert Zone alarm, carrying the zone type.
    AlertZone(u8),
    Other(u8),
}

impl PflauAlarmType {
    fn from_field(field: &[u8]) -> Self {
        match field {
            b"0" => Self::None,
            b"2" => Self::Aircraft,
            b"3" => Self::Obstacle,
            b"4" => Self::TrafficAdvisory,
            field => match parse_hex(field) {
                Some(zone @ 0x10..) => Self::AlertZone(zone),
                Some(other) => Self::Other(other),
                None => Self::default(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use claims::{assert_none, assert_some_eq};

    #[test]
    fn parses_a_priority_intruder() {
        // The ICD's alarm example: level 2, intruder at 11 o'clock,
        // 32 m below, 755 m away.
        let fields = FieldsIter::new(b"3,1,2,1,2,-30,2,-32,755,");
        let pflau = Pflau::parse(fields);
        assert_some_eq!(pflau.rx_count, 3);
        assert_some_eq!(pflau.tx_ok, true);
        assert_eq!(pflau.gps_status, PflauGpsStatus::Airborne);
        assert_some_eq!(pflau.power_ok, true);
        assert_eq!(pflau.alarm_level, FlarmAlarmLevel::Important);
        assert_some_eq!(pflau.relative_bearing, Angle::from_degrees(-30.0));
        assert_eq!(pflau.alarm_type, PflauAlarmType::Aircraft);
        assert_some_eq!(pflau.relative_vertical, Length::from_meters(-32.0));
        assert_some_eq!(pflau.relative_distance, Length::from_meters(755.0));
        assert_none!(pflau.id);
    }

    #[test]
    fn quiet_heartbeat_reads_no_threat() {
        // No alarm and nothing within range: the threat fields are empty.
        let fields = FieldsIter::new(b"2,1,1,1,0,,0,,,");
        let pflau = Pflau::parse(fields);
        assert_eq!(pflau.gps_status, PflauGpsStatus::OnGround);
        assert_eq!(pflau.alarm_level, FlarmAlarmLevel::None);
        assert_none!(pflau.relative_bearing);
        assert_eq!(pflau.alarm_type, PflauAlarmType::None);
        assert_none!(pflau.relative_vertical);
        assert_none!(pflau.relative_distance);
        assert_none!(pflau.id);
    }

    #[test]
    fn maps_gps_status() {
        assert_eq!(PflauGpsStatus::default(), PflauGpsStatus::NoFix);
        assert_eq!(PflauGpsStatus::from_field(b"0"), PflauGpsStatus::NoFix);
        assert_eq!(PflauGpsStatus::from_field(b"1"), PflauGpsStatus::OnGround);
        assert_eq!(PflauGpsStatus::from_field(b"2"), PflauGpsStatus::Airborne);
        assert_eq!(PflauGpsStatus::from_field(b"3"), PflauGpsStatus::Other(3));
    }

    #[test]
    fn maps_alarm_types() {
        assert_eq!(PflauAlarmType::default(), PflauAlarmType::None);
        assert_eq!(PflauAlarmType::from_field(b"0"), PflauAlarmType::None);
        assert_eq!(PflauAlarmType::from_field(b"2"), PflauAlarmType::Aircraft);
        assert_eq!(PflauAlarmType::from_field(b"3"), PflauAlarmType::Obstacle);
        assert_eq!(
            PflauAlarmType::from_field(b"4"),
            PflauAlarmType::TrafficAdvisory
        );
        // `10`-`FF` carry the type of an Alert Zone.
        assert_eq!(
            PflauAlarmType::from_field(b"41"),
            PflauAlarmType::AlertZone(0x41)
        );
        assert_eq!(PflauAlarmType::from_field(b"5"), PflauAlarmType::Other(5));
    }

    #[test]
    fn alarm_type_is_hexadecimal() {
        // The Alert Zone type `41` must read as 0x41, not decimal 41.
        let fields = FieldsIter::new(b"1,1,2,1,1,0,41,0,0,");
        let pflau = Pflau::parse(fields);
        assert_eq!(pflau.alarm_type, PflauAlarmType::AlertZone(0x41));
    }
}
