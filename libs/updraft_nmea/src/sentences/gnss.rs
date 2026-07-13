//! Standard GNSS sentences: `GGA`, `RMC`, `GSA`, for any talker.

use crate::datetime::{Date, Time};
use crate::field::FieldsIter;
use crate::message::Talker;
use updraft_geo::LatLon;
use updraft_units::{Angle, Length, Speed};

/// GNSS fix data from a `GGA` sentence.
#[derive(Clone, Debug, PartialEq)]
pub struct Gga {
    pub talker: Talker,
    pub utc_time: Option<Time>,
    pub position: Option<LatLon>,
    pub fix_quality: GgaFixQuality,
    pub satellites_used: Option<u8>,
    pub hdop: Option<f64>,
    /// Altitude above mean sea level.
    pub altitude: Option<Length>,
    /// Height of the geoid above the WGS84 ellipsoid.
    pub geoid_separation: Option<Length>,
    pub dgps_age: Option<f64>,
    pub dgps_station: Option<u16>,
}

impl Gga {
    pub fn parse(talker: Talker, mut fields: FieldsIter<'_>) -> Self {
        Self {
            talker,
            utc_time: fields.bytes().and_then(Time::parse),
            position: fields.lat_lon(),
            fix_quality: fields
                .bytes()
                .map(GgaFixQuality::from_field)
                .unwrap_or_default(),
            satellites_used: fields.u8(),
            hdop: fields.f64(),
            altitude: meters(&mut fields),
            geoid_separation: meters(&mut fields),
            dgps_age: fields.f64(),
            dgps_station: fields.u16(),
        }
    }
}

/// A length field paired with its unit field, which `GGA` always reports as
/// `M` for meters. A missing or non-meter unit reads the length as absent.
/// Both fields are consumed even when the value is absent.
fn meters(fields: &mut FieldsIter<'_>) -> Option<Length> {
    let value = fields.f64();
    match fields.bytes()? {
        b"M" | b"m" => Some(Length::from_meters(value?)),
        _ => None,
    }
}

/// The fix quality reported in a `GGA` sentence.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum GgaFixQuality {
    #[default]
    Invalid,
    Gps,
    Dgps,
    Pps,
    RealTimeKinematic,
    FloatRtk,
    DeadReckoning,
    Manual,
    Simulation,
    Other(u8),
}

impl GgaFixQuality {
    fn from_field(field: &[u8]) -> Self {
        match field {
            b"0" => Self::Invalid,
            b"1" => Self::Gps,
            b"2" => Self::Dgps,
            b"3" => Self::Pps,
            b"4" => Self::RealTimeKinematic,
            b"5" => Self::FloatRtk,
            b"6" => Self::DeadReckoning,
            b"7" => Self::Manual,
            b"8" => Self::Simulation,
            field => btoi::btou(field).ok().map(Self::Other).unwrap_or_default(),
        }
    }
}

/// Recommended minimum GNSS data from an `RMC` sentence.
#[derive(Clone, Debug, PartialEq)]
pub struct Rmc {
    pub talker: Talker,
    pub utc_time: Option<Time>,
    pub status: RmcStatus,
    pub position: Option<LatLon>,
    pub speed_over_ground: Option<Speed>,
    pub course_over_ground: Option<Angle>,
    pub date: Option<Date>,
    /// Magnetic variation, positive east of true north.
    pub magnetic_variation: Option<Angle>,
    pub mode: Option<PositioningMode>,
}

impl Rmc {
    pub fn parse(talker: Talker, mut fields: FieldsIter<'_>) -> Self {
        Self {
            talker,
            utc_time: fields.bytes().and_then(Time::parse),
            status: fields
                .bytes()
                .map(RmcStatus::from_field)
                .unwrap_or_default(),
            position: fields.lat_lon(),
            speed_over_ground: fields.f64().map(Speed::from_knots),
            course_over_ground: fields.f64().map(Angle::from_degrees),
            date: fields.bytes().and_then(Date::parse_ddmmyy),
            magnetic_variation: magnetic_variation(&mut fields),
            mode: fields
                .bytes()
                .and_then(|field| field.first().copied())
                .map(|byte| PositioningMode::from_char(char::from(byte))),
        }
    }
}

/// The signed magnetic variation from a magnitude field and its `E`/`W`
/// hemisphere. Both fields are consumed even when the magnitude is absent.
fn magnetic_variation(fields: &mut FieldsIter<'_>) -> Option<Angle> {
    let degrees = fields.f64();
    match fields.bytes()? {
        b"E" => Some(Angle::from_degrees(degrees?)),
        b"W" => Some(Angle::from_degrees(-degrees?)),
        _ => None,
    }
}

/// Whether a `RMC` fix is valid. Any value other than `A`, including an
/// absent field, reads as `Void`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum RmcStatus {
    /// `A`: data valid.
    Active,
    /// `V`: navigation receiver warning.
    #[default]
    Void,
}

impl RmcStatus {
    fn from_field(field: &[u8]) -> Self {
        if field == b"A" {
            Self::Active
        } else {
            Self::default()
        }
    }
}

/// The positioning-mode indicator added to `RMC` in NMEA 2.3.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PositioningMode {
    Autonomous,
    Differential,
    Estimated,
    Manual,
    NotValid,
    Other(char),
}

impl PositioningMode {
    fn from_char(mode: char) -> Self {
        match mode {
            'A' => Self::Autonomous,
            'D' => Self::Differential,
            'E' => Self::Estimated,
            'M' => Self::Manual,
            'N' => Self::NotValid,
            other => Self::Other(other),
        }
    }
}

/// GNSS DOP and the satellites used in the fix (`GSA`).
#[derive(Clone, Debug, PartialEq)]
pub struct Gsa {
    pub talker: Talker,
    pub selection_mode: GsaSelectionMode,
    pub fix_type: GsaFixType,
    /// PRNs of the satellites used in the fix (up to twelve).
    pub satellites: Vec<u16>,
    pub pdop: Option<f64>,
    pub hdop: Option<f64>,
    pub vdop: Option<f64>,
}

impl Gsa {
    pub fn parse(talker: Talker, mut fields: FieldsIter<'_>) -> Self {
        Self {
            talker,
            selection_mode: fields
                .bytes()
                .map(GsaSelectionMode::from_field)
                .unwrap_or_default(),
            fix_type: fields
                .bytes()
                .map(GsaFixType::from_field)
                .unwrap_or_default(),
            // Twelve satellite fields; absent ones are consumed but dropped.
            satellites: (0..12).filter_map(|_| fields.u16()).collect(),
            pdop: fields.f64(),
            hdop: fields.f64(),
            vdop: fields.f64(),
        }
    }
}

/// Whether the `GSA` fix mode was chosen automatically or manually. Any
/// value other than `M`, including an absent field, reads as `Automatic`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum GsaSelectionMode {
    /// `A`: automatic 2D/3D selection.
    #[default]
    Automatic,
    /// `M`: manually forced.
    Manual,
}

impl GsaSelectionMode {
    fn from_field(field: &[u8]) -> Self {
        if field == b"M" {
            Self::Manual
        } else {
            Self::default()
        }
    }
}

/// The dimensionality of a `GSA` fix.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum GsaFixType {
    /// `1`: no fix.
    #[default]
    NoFix,
    /// `2`: 2D fix.
    TwoDimensional,
    /// `3`: 3D fix.
    ThreeDimensional,
    Other(u8),
}

impl GsaFixType {
    fn from_field(field: &[u8]) -> Self {
        match field {
            b"1" => Self::NoFix,
            b"2" => Self::TwoDimensional,
            b"3" => Self::ThreeDimensional,
            field => btoi::btou(field).ok().map(Self::Other).unwrap_or_default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use claims::{assert_none, assert_some_eq};

    #[test]
    fn reads_altitude_and_geoid_separation_in_meters() {
        let fields = FieldsIter::new(b"123519,4807.038,N,01131.000,E,1,08,0.9,545.4,M,46.9,M,,");
        let gga = Gga::parse(Talker::Gps, fields);
        assert_some_eq!(gga.altitude, Length::from_meters(545.4));
        assert_some_eq!(gga.geoid_separation, Length::from_meters(46.9));
    }

    #[test]
    fn ignores_altitude_with_a_non_meter_unit() {
        let fields = FieldsIter::new(b"123519,4807.038,N,01131.000,E,1,08,0.9,545.4,F,46.9,F,,");
        let gga = Gga::parse(Talker::Gps, fields);
        assert_none!(gga.altitude);
        assert_none!(gga.geoid_separation);
    }

    #[test]
    fn maps_gga_fix_quality_codes() {
        assert_eq!(GgaFixQuality::default(), GgaFixQuality::Invalid);
        assert_eq!(GgaFixQuality::from_field(b"0"), GgaFixQuality::Invalid);
        assert_eq!(GgaFixQuality::from_field(b"1"), GgaFixQuality::Gps);
        assert_eq!(GgaFixQuality::from_field(b"8"), GgaFixQuality::Simulation);
        assert_eq!(GgaFixQuality::from_field(b"9"), GgaFixQuality::Other(9));
    }

    #[test]
    fn signs_magnetic_variation_by_hemisphere() {
        assert_some_eq!(
            magnetic_variation(&mut FieldsIter::new(b"3.5,E")),
            Angle::from_degrees(3.5)
        );
        assert_some_eq!(
            magnetic_variation(&mut FieldsIter::new(b"3.5,W")),
            Angle::from_degrees(-3.5)
        );
        assert_none!(magnetic_variation(&mut FieldsIter::new(b"3.5,X")));
    }

    #[test]
    fn keeps_three_digit_satellite_prns() {
        // Galileo ids run 301-336, past a byte, so they must survive decoding.
        let fields = FieldsIter::new(b"A,3,301,302,336,,,,,,,,,,1.2,1.0,1.0");
        let gsa = Gsa::parse(Talker::Galileo, fields);
        assert_eq!(gsa.satellites.as_slice(), [301, 302, 336]);
        assert_some_eq!(gsa.pdop, 1.2);
        assert_some_eq!(gsa.vdop, 1.0);
    }
}
