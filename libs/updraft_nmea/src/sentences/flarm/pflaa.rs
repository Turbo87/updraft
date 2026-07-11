use super::common::{FlarmAlarmLevel, FlarmId, bool_field, hex_field};
use crate::field::FieldsIter;
use updraft_units::{Angle, Length, Speed};

/// One nearby traffic target (`$PFLAA`).
///
/// Sent on a best-effort basis: individual targets may be skipped under
/// load, so collision warnings must come from `PFLAU`, not from this
/// sentence. Fields of targets in stealth/privacy mode read as absent.
#[derive(Clone, Debug, PartialEq)]
pub struct Pflaa {
    pub alarm_level: FlarmAlarmLevel,
    /// Position of the target relative to own position, along true north.
    /// For non-directional targets (`relative_east` absent) this carries
    /// the estimated distance instead.
    pub relative_north: Option<Length>,
    /// Position of the target relative to own position, along true east.
    /// Absent for non-directional targets.
    pub relative_east: Option<Length>,
    /// Vertical separation of the target above own position, negative when
    /// it is lower.
    pub relative_vertical: Option<Length>,
    /// How to interpret [`id`](Self::id). Absent when no identification is
    /// known (e.g. transponder Mode-C).
    pub id_type: Option<FlarmIdType>,
    /// The target's ID. Absent when no identification is known.
    pub id: Option<FlarmId>,
    /// The target's true ground track. Absent for stealth and
    /// non-directional targets.
    pub track: Option<Angle>,
    /// Turn rate in degrees per second, clockwise positive.
    pub turn_rate: Option<f64>,
    /// The target's ground speed, forced to `0` while it is on the ground.
    /// Absent for stealth and non-directional targets.
    pub ground_speed: Option<Speed>,
    /// The target's climb rate, positive when climbing. Absent for stealth
    /// and non-directional targets.
    pub climb_rate: Option<Speed>,
    pub aircraft_type: FlarmAircraftType,
    /// Whether the target asked not to be tracked: its data may not be
    /// persisted or forwarded. Introduced in protocol version 8.
    pub no_track: Option<bool>,
    /// Which receiver picked the target up. Introduced in protocol
    /// version 9.
    pub source: Option<FlarmSource>,
    /// Signal level of the received target in `dBm`. Introduced in
    /// protocol version 9.
    pub rssi: Option<f64>,
}

impl Pflaa {
    pub fn parse(mut fields: FieldsIter<'_>) -> Self {
        Self {
            alarm_level: FlarmAlarmLevel::from_field(fields.u8()),
            relative_north: fields.f64().map(Length::from_meters),
            relative_east: fields.f64().map(Length::from_meters),
            relative_vertical: fields.f64().map(Length::from_meters),
            id_type: fields.u8().map(FlarmIdType::from_value),
            id: fields.bytes().and_then(FlarmId::parse),
            track: fields.f64().map(Angle::from_degrees),
            turn_rate: fields.f64(),
            ground_speed: fields.f64().map(Speed::from_meters_per_second),
            climb_rate: fields.f64().map(Speed::from_meters_per_second),
            aircraft_type: FlarmAircraftType::from_field(hex_field(&mut fields)),
            no_track: bool_field(&mut fields),
            source: fields.u8().map(FlarmSource::from_value),
            rssi: fields.f64(),
        }
    }
}

/// How the ID of a `PFLAA` target is to be interpreted.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FlarmIdType {
    /// `0`: randomly generated ID, configured or from stealth mode.
    Random,
    /// `1`: official ICAO 24-bit aircraft address.
    Icao,
    /// `2`: fixed FLARM ID.
    Flarm,
    Other(u8),
}

impl FlarmIdType {
    fn from_value(value: u8) -> Self {
        match value {
            0 => Self::Random,
            1 => Self::Icao,
            2 => Self::Flarm,
            other => Self::Other(other),
        }
    }
}

/// The aircraft type of a `PFLAA` target. Transmitted as a hexadecimal
/// value.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FlarmAircraftType {
    /// `0` (reserved) or `A`: no aircraft type known.
    Unknown,
    /// `1`: glider, motor glider, or TMG.
    Glider,
    /// `2`: tow/tug plane.
    TowPlane,
    /// `3`: helicopter, gyrocopter, or other rotorcraft.
    Helicopter,
    /// `4`: skydiver or parachute.
    Skydiver,
    /// `5`: drop plane for skydivers.
    DropPlane,
    /// `6`: hang glider.
    HangGlider,
    /// `7`: paraglider.
    Paraglider,
    /// `8`: aircraft with reciprocating engine(s).
    PistonAircraft,
    /// `9`: aircraft with jet or turboprop engine(s).
    JetAircraft,
    /// `B`: balloon.
    Balloon,
    /// `C`: airship, blimp, or zeppelin.
    Airship,
    /// `D`: unmanned aerial vehicle (UAV, drone).
    Uav,
    /// `F`: static obstacle.
    StaticObstacle,
    Other(u8),
}

impl FlarmAircraftType {
    fn from_field(value: Option<u8>) -> Self {
        match value {
            None | Some(0x0) | Some(0xA) => Self::Unknown,
            Some(0x1) => Self::Glider,
            Some(0x2) => Self::TowPlane,
            Some(0x3) => Self::Helicopter,
            Some(0x4) => Self::Skydiver,
            Some(0x5) => Self::DropPlane,
            Some(0x6) => Self::HangGlider,
            Some(0x7) => Self::Paraglider,
            Some(0x8) => Self::PistonAircraft,
            Some(0x9) => Self::JetAircraft,
            Some(0xB) => Self::Balloon,
            Some(0xC) => Self::Airship,
            Some(0xD) => Self::Uav,
            Some(0xF) => Self::StaticObstacle,
            Some(other) => Self::Other(other),
        }
    }
}

/// The receiver a `PFLAA` target was picked up by. When a target is
/// received over several sources at once, FLARM reports the most direct
/// one (FLARM radio before ADS-B before rebroadcasts).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FlarmSource {
    /// `0`: FLARM radio.
    Flarm,
    /// `1`: ADS-B.
    AdsB,
    /// `3`: ADS-R (UAT ADS-B rebroadcast to 1090 MHz).
    AdsR,
    /// `4`: TIS-B (ground-station broadcast of non-ADS-B aircraft).
    TisB,
    /// `6`: transponder Mode-S (non-directional).
    ModeS,
    Other(u8),
}

impl FlarmSource {
    fn from_value(value: u8) -> Self {
        match value {
            0 => Self::Flarm,
            1 => Self::AdsB,
            3 => Self::AdsR,
            4 => Self::TisB,
            6 => Self::ModeS,
            other => Self::Other(other),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use claims::{assert_none, assert_some, assert_some_eq};

    #[test]
    fn parses_a_traffic_target() {
        // The ICD's traffic example: a glider 1.2 km south, 1.2 km east,
        // 220 m higher, on a south track at 30 m/s, sinking 1.4 m/s.
        let fields = FieldsIter::new(b"0,-1234,1234,220,2,DD8F12,180,,30,-1.4,1");
        let pflaa = Pflaa::parse(fields);
        assert_eq!(pflaa.alarm_level, FlarmAlarmLevel::None);
        assert_some_eq!(pflaa.relative_north, Length::from_meters(-1234.0));
        assert_some_eq!(pflaa.relative_east, Length::from_meters(1234.0));
        assert_some_eq!(pflaa.relative_vertical, Length::from_meters(220.0));
        assert_some_eq!(pflaa.id_type, FlarmIdType::Flarm);
        let id = assert_some!(pflaa.id);
        assert_eq!(id.address, 0xDD8F12);
        assert_none!(id.callsign);
        assert_some_eq!(pflaa.track, Angle::from_degrees(180.0));
        assert_none!(pflaa.turn_rate);
        assert_some_eq!(pflaa.ground_speed, Speed::from_meters_per_second(30.0));
        assert_some_eq!(pflaa.climb_rate, Speed::from_meters_per_second(-1.4));
        assert_eq!(pflaa.aircraft_type, FlarmAircraftType::Glider);
        assert_none!(pflaa.no_track);
        assert_none!(pflaa.source);
        assert_none!(pflaa.rssi);
    }

    #[test]
    fn parses_the_version_9_trailing_fields() {
        let fields = FieldsIter::new(b"0,1206,504,182,1,DDA85C,240,,49,2.5,9,0,1,-58.5");
        let pflaa = Pflaa::parse(fields);
        assert_eq!(pflaa.aircraft_type, FlarmAircraftType::JetAircraft);
        assert_some_eq!(pflaa.no_track, false);
        assert_some_eq!(pflaa.source, FlarmSource::AdsB);
        assert_some_eq!(pflaa.rssi, -58.5);
    }

    #[test]
    fn non_directional_target_keeps_only_a_distance_estimate() {
        // A transponder Mode-S target: no bearing, so `relative_north`
        // carries the distance estimate, and the identity/motion fields
        // are empty.
        let fields = FieldsIter::new(b"0,1852,,-163,,,,,,,0");
        let pflaa = Pflaa::parse(fields);
        assert_some_eq!(pflaa.relative_north, Length::from_meters(1852.0));
        assert_none!(pflaa.relative_east);
        assert_none!(pflaa.id_type);
        assert_none!(pflaa.id);
        assert_none!(pflaa.track);
        assert_none!(pflaa.ground_speed);
        assert_none!(pflaa.climb_rate);
        assert_eq!(pflaa.aircraft_type, FlarmAircraftType::Unknown);
    }

    #[test]
    fn maps_aircraft_types() {
        // `0` is reserved and `A` is "unknown": both read as `Unknown`,
        // like an absent field.
        assert_eq!(
            FlarmAircraftType::from_field(None),
            FlarmAircraftType::Unknown
        );
        assert_eq!(
            FlarmAircraftType::from_field(Some(0x0)),
            FlarmAircraftType::Unknown
        );
        assert_eq!(
            FlarmAircraftType::from_field(Some(0xA)),
            FlarmAircraftType::Unknown
        );
        assert_eq!(
            FlarmAircraftType::from_field(Some(0x1)),
            FlarmAircraftType::Glider
        );
        assert_eq!(
            FlarmAircraftType::from_field(Some(0x7)),
            FlarmAircraftType::Paraglider
        );
        assert_eq!(
            FlarmAircraftType::from_field(Some(0xF)),
            FlarmAircraftType::StaticObstacle
        );
        assert_eq!(
            FlarmAircraftType::from_field(Some(0xE)),
            FlarmAircraftType::Other(0xE)
        );
    }

    #[test]
    fn maps_id_types_and_sources() {
        assert_eq!(FlarmIdType::from_value(0), FlarmIdType::Random);
        assert_eq!(FlarmIdType::from_value(1), FlarmIdType::Icao);
        assert_eq!(FlarmIdType::from_value(2), FlarmIdType::Flarm);
        assert_eq!(FlarmIdType::from_value(3), FlarmIdType::Other(3));

        assert_eq!(FlarmSource::from_value(0), FlarmSource::Flarm);
        assert_eq!(FlarmSource::from_value(1), FlarmSource::AdsB);
        assert_eq!(FlarmSource::from_value(3), FlarmSource::AdsR);
        assert_eq!(FlarmSource::from_value(4), FlarmSource::TisB);
        assert_eq!(FlarmSource::from_value(6), FlarmSource::ModeS);
        assert_eq!(FlarmSource::from_value(2), FlarmSource::Other(2));
    }
}
