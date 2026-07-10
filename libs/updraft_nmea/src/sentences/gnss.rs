//! Standard GNSS sentences: `GGA`, `RMC`, `GSA`, for any talker.

use crate::datetime::{Date, Time};
use crate::field::{f64_field, field, lat_lon, parsed_field};
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
    pub fn parse(talker: Talker, fields: &[&[u8]]) -> Self {
        Self {
            talker,
            utc_time: field(fields, 0).and_then(Time::parse),
            position: lat_lon(fields, 1, 2, 3, 4),
            fix_quality: GgaFixQuality::from_field(parsed_field(fields, 5)),
            satellites_used: parsed_field(fields, 6),
            hdop: f64_field(fields, 7),
            altitude: meters(fields, 8, 9),
            geoid_separation: meters(fields, 10, 11),
            dgps_age: f64_field(fields, 12),
            dgps_station: parsed_field(fields, 13),
        }
    }
}

/// A length field paired with its unit field, which `GGA` always reports as
/// `M` for meters. A missing or non-meter unit reads the length as absent.
fn meters(fields: &[&[u8]], value: usize, unit: usize) -> Option<Length> {
    let value = f64_field(fields, value)?;
    match field(fields, unit)? {
        b"M" | b"m" => Some(Length::from_meters(value)),
        _ => None,
    }
}

/// The fix quality reported in a `GGA` sentence.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum GgaFixQuality {
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
    fn from_field(value: Option<u8>) -> Self {
        match value {
            None | Some(0) => Self::Invalid,
            Some(1) => Self::Gps,
            Some(2) => Self::Dgps,
            Some(3) => Self::Pps,
            Some(4) => Self::RealTimeKinematic,
            Some(5) => Self::FloatRtk,
            Some(6) => Self::DeadReckoning,
            Some(7) => Self::Manual,
            Some(8) => Self::Simulation,
            Some(other) => Self::Other(other),
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
    pub fn parse(talker: Talker, fields: &[&[u8]]) -> Self {
        Self {
            talker,
            utc_time: field(fields, 0).and_then(Time::parse),
            status: RmcStatus::from_field(field(fields, 1)),
            position: lat_lon(fields, 2, 3, 4, 5),
            speed_over_ground: f64_field(fields, 6).map(Speed::from_knots),
            course_over_ground: f64_field(fields, 7).map(Angle::from_degrees),
            date: field(fields, 8).and_then(Date::parse_ddmmyy),
            magnetic_variation: magnetic_variation(fields, 9, 10),
            mode: field(fields, 11)
                .and_then(|field| field.first().copied())
                .map(|byte| PositioningMode::from_char(char::from(byte))),
        }
    }
}

/// The signed magnetic variation from a magnitude field and its `E`/`W`
/// hemisphere.
fn magnetic_variation(fields: &[&[u8]], value: usize, hemisphere: usize) -> Option<Angle> {
    let degrees = f64_field(fields, value)?;
    match field(fields, hemisphere)? {
        b"E" => Some(Angle::from_degrees(degrees)),
        b"W" => Some(Angle::from_degrees(-degrees)),
        _ => None,
    }
}

/// Whether a `RMC` fix is valid. Any value other than `A`, including an
/// absent field, reads as `Void`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RmcStatus {
    /// `A`: data valid.
    Active,
    /// `V`: navigation receiver warning.
    Void,
}

impl RmcStatus {
    fn from_field(field: Option<&[u8]>) -> Self {
        if field == Some(b"A".as_slice()) {
            Self::Active
        } else {
            Self::Void
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
    pub fn parse(talker: Talker, fields: &[&[u8]]) -> Self {
        Self {
            talker,
            selection_mode: GsaSelectionMode::from_field(field(fields, 0)),
            fix_type: GsaFixType::from_field(parsed_field(fields, 1)),
            satellites: (2..14)
                .filter_map(|index| parsed_field(fields, index))
                .collect(),
            pdop: f64_field(fields, 14),
            hdop: f64_field(fields, 15),
            vdop: f64_field(fields, 16),
        }
    }
}

/// Whether the `GSA` fix mode was chosen automatically or manually. Any
/// value other than `M`, including an absent field, reads as `Automatic`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum GsaSelectionMode {
    /// `A`: automatic 2D/3D selection.
    Automatic,
    /// `M`: manually forced.
    Manual,
}

impl GsaSelectionMode {
    fn from_field(field: Option<&[u8]>) -> Self {
        if field == Some(b"M".as_slice()) {
            Self::Manual
        } else {
            Self::Automatic
        }
    }
}

/// The dimensionality of a `GSA` fix.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum GsaFixType {
    /// `1`: no fix.
    NoFix,
    /// `2`: 2D fix.
    TwoDimensional,
    /// `3`: 3D fix.
    ThreeDimensional,
    Other(u8),
}

impl GsaFixType {
    fn from_field(value: Option<u8>) -> Self {
        match value {
            None | Some(1) => Self::NoFix,
            Some(2) => Self::TwoDimensional,
            Some(3) => Self::ThreeDimensional,
            Some(other) => Self::Other(other),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use claims::{assert_none, assert_some_eq};

    fn gga_fields() -> Vec<&'static [u8]> {
        vec![
            b"123519",
            b"4807.038",
            b"N",
            b"01131.000",
            b"E",
            b"1",
            b"08",
            b"0.9",
            b"545.4",
            b"M",
            b"46.9",
            b"M",
            b"",
            b"",
        ]
    }

    #[test]
    fn reads_altitude_and_geoid_separation_in_meters() {
        let gga = Gga::parse(Talker::Gps, &gga_fields());
        assert_some_eq!(gga.altitude, Length::from_meters(545.4));
        assert_some_eq!(gga.geoid_separation, Length::from_meters(46.9));
    }

    #[test]
    fn ignores_altitude_with_a_non_meter_unit() {
        let mut fields = gga_fields();
        fields[9] = b"F";
        fields[11] = b"F";
        let gga = Gga::parse(Talker::Gps, &fields);
        assert_none!(gga.altitude);
        assert_none!(gga.geoid_separation);
    }

    #[test]
    fn maps_gga_fix_quality_codes() {
        assert_eq!(GgaFixQuality::from_field(None), GgaFixQuality::Invalid);
        assert_eq!(GgaFixQuality::from_field(Some(0)), GgaFixQuality::Invalid);
        assert_eq!(GgaFixQuality::from_field(Some(1)), GgaFixQuality::Gps);
        assert_eq!(
            GgaFixQuality::from_field(Some(8)),
            GgaFixQuality::Simulation
        );
        assert_eq!(GgaFixQuality::from_field(Some(9)), GgaFixQuality::Other(9));
    }

    #[test]
    fn signs_magnetic_variation_by_hemisphere() {
        let east: [&[u8]; 2] = [b"3.5", b"E"];
        assert_some_eq!(magnetic_variation(&east, 0, 1), Angle::from_degrees(3.5));
        let west: [&[u8]; 2] = [b"3.5", b"W"];
        assert_some_eq!(magnetic_variation(&west, 0, 1), Angle::from_degrees(-3.5));
        let unknown: [&[u8]; 2] = [b"3.5", b"X"];
        assert_none!(magnetic_variation(&unknown, 0, 1));
    }

    #[test]
    fn keeps_three_digit_satellite_prns() {
        // Galileo ids run 301-336, past a byte, so they must survive decoding.
        let fields: [&[u8]; 17] = [
            b"A", b"3", b"301", b"302", b"336", b"", b"", b"", b"", b"", b"", b"", b"", b"",
            b"1.2", b"1.0", b"1.0",
        ];
        let gsa = Gsa::parse(Talker::Galileo, &fields);
        assert_eq!(gsa.satellites, vec![301, 302, 336]);
    }
}
