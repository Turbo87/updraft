//! Standard GNSS sentences: `GGA`, `RMC`, `GSA`, for any talker.

use crate::datetime::{Date, Time};
use crate::encode::{EncodeError, SentenceEncoder, position_fields, talker_code};
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

impl TryFrom<&Gga> for Vec<u8> {
    type Error = EncodeError;

    fn try_from(gga: &Gga) -> Result<Self, Self::Error> {
        let mut sentence = SentenceEncoder::new(&format!("{}GGA", talker_code(&gga.talker)?));
        sentence.field(&gga.utc_time.map(Time::to_nmea_field).unwrap_or_default());
        for field in position_fields(gga.position) {
            sentence.field(&field);
        }
        sentence.field(&gga.fix_quality.to_nmea_field());
        sentence.field(&optional_field(gga.satellites_used));
        sentence.field(&optional_field(gga.hdop));
        encode_length_in_meters(&mut sentence, gga.altitude);
        encode_length_in_meters(&mut sentence, gga.geoid_separation);
        sentence.field(&optional_field(gga.dgps_age));
        sentence.field(&optional_field(gga.dgps_station));
        Ok(sentence.finish())
    }
}

fn encode_length_in_meters(sentence: &mut SentenceEncoder, length: Option<Length>) {
    match length {
        Some(length) => {
            sentence.field(&length.as_meters().to_string());
            sentence.field("M");
        }
        None => {
            sentence.field("");
            sentence.field("");
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

    fn to_nmea_field(self) -> String {
        match self {
            Self::Invalid => "0".to_owned(),
            Self::Gps => "1".to_owned(),
            Self::Dgps => "2".to_owned(),
            Self::Pps => "3".to_owned(),
            Self::RealTimeKinematic => "4".to_owned(),
            Self::FloatRtk => "5".to_owned(),
            Self::DeadReckoning => "6".to_owned(),
            Self::Manual => "7".to_owned(),
            Self::Simulation => "8".to_owned(),
            Self::Other(value) => value.to_string(),
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

impl TryFrom<&Rmc> for Vec<u8> {
    type Error = EncodeError;

    fn try_from(rmc: &Rmc) -> Result<Self, Self::Error> {
        let mut sentence = SentenceEncoder::new(&format!("{}RMC", talker_code(&rmc.talker)?));
        sentence.field(&rmc.utc_time.map(Time::to_nmea_field).unwrap_or_default());
        sentence.field(match rmc.status {
            RmcStatus::Active => "A",
            RmcStatus::Void => "V",
        });
        for field in position_fields(rmc.position) {
            sentence.field(&field);
        }
        sentence.field(&optional_field(
            rmc.speed_over_ground.map(|speed| speed.as_knots()),
        ));
        sentence.field(&optional_field(
            rmc.course_over_ground.map(|course| course.as_degrees()),
        ));
        sentence.field(
            &rmc.date
                .map(Date::to_nmea_field)
                .transpose()?
                .unwrap_or_default(),
        );
        match rmc.magnetic_variation {
            Some(variation) => {
                let degrees = variation.as_degrees();
                sentence.field(&degrees.abs().to_string());
                sentence.field(if degrees.is_sign_negative() { "W" } else { "E" });
            }
            None => {
                sentence.field("");
                sentence.field("");
            }
        }
        let mode = match rmc.mode {
            Some(PositioningMode::Autonomous) => "A".to_owned(),
            Some(PositioningMode::Differential) => "D".to_owned(),
            Some(PositioningMode::Estimated) => "E".to_owned(),
            Some(PositioningMode::Manual) => "M".to_owned(),
            Some(PositioningMode::NotValid) => "N".to_owned(),
            Some(PositioningMode::Other(mode)) if mode.is_ascii_uppercase() => mode.to_string(),
            Some(PositioningMode::Other(_)) => return Err(EncodeError::InvalidField("mode")),
            None => String::new(),
        };
        sentence.field(&mode);
        Ok(sentence.finish())
    }
}

fn optional_field<T: ToString>(value: Option<T>) -> String {
    value.map_or_else(String::new, |value| value.to_string())
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
    use crate::{Message, Step, parse};
    use approx::assert_abs_diff_eq;
    use claims::{assert_err_eq, assert_none, assert_ok, assert_some, assert_some_eq};

    #[test]
    fn encodes_complete_gga_sentence() {
        insta::assert_snapshot!(encode_gga_sentence(&complete_gga()));
    }

    #[test]
    fn encodes_gga_sentence_with_empty_optional_fields() {
        let gga = Gga {
            talker: Talker::Combined,
            utc_time: None,
            position: None,
            fix_quality: GgaFixQuality::Invalid,
            satellites_used: None,
            hdop: None,
            altitude: None,
            geoid_separation: None,
            dgps_age: None,
            dgps_station: None,
        };

        insta::assert_snapshot!(encode_gga_sentence(&gga));
    }

    #[test]
    fn parses_encoded_gga_sentence() {
        let expected = complete_gga();
        let sentence = assert_ok!(Vec::<u8>::try_from(&expected));
        let mut input = sentence.as_slice();

        let actual = match parse(&mut input) {
            Step::Frame(Message::Gga(gga)) => gga,
            step => panic!("expected encoded GGA frame, got {step:?}"),
        };

        assert_eq!(actual.talker, expected.talker);
        assert_eq!(actual.utc_time, expected.utc_time);
        assert_abs_diff_eq!(
            assert_some!(actual.position),
            assert_some!(expected.position),
            epsilon = 1e-9
        );
        assert_eq!(actual.fix_quality, expected.fix_quality);
        assert_eq!(actual.satellites_used, expected.satellites_used);
        assert_eq!(actual.hdop, expected.hdop);
        assert_eq!(actual.altitude, expected.altitude);
        assert_eq!(actual.geoid_separation, expected.geoid_separation);
        assert_eq!(actual.dgps_age, expected.dgps_age);
        assert_eq!(actual.dgps_station, expected.dgps_station);
    }

    #[test]
    fn encodes_complete_rmc_sentence() {
        insta::assert_snapshot!(encode_rmc_sentence(&complete_rmc()));
    }

    #[test]
    fn encodes_rmc_sentence_without_date() {
        let mut rmc = complete_rmc();
        rmc.date = None;

        insta::assert_snapshot!(encode_rmc_sentence(&rmc));
    }

    #[test]
    fn parses_encoded_rmc_sentence() {
        let expected = complete_rmc();
        let sentence = assert_ok!(Vec::<u8>::try_from(&expected));
        let mut input = sentence.as_slice();

        let actual = match parse(&mut input) {
            Step::Frame(Message::Rmc(rmc)) => rmc,
            step => panic!("expected encoded RMC frame, got {step:?}"),
        };

        assert_eq!(actual.talker, expected.talker);
        assert_eq!(actual.utc_time, expected.utc_time);
        assert_eq!(actual.status, expected.status);
        assert_abs_diff_eq!(
            assert_some!(actual.position),
            assert_some!(expected.position),
            epsilon = 1e-9
        );
        assert_abs_diff_eq!(
            assert_some!(actual.speed_over_ground),
            assert_some!(expected.speed_over_ground),
            epsilon = 1e-9
        );
        assert_abs_diff_eq!(
            assert_some!(actual.course_over_ground),
            assert_some!(expected.course_over_ground),
            epsilon = 1e-9
        );
        assert_eq!(actual.date, expected.date);
        assert_eq!(actual.magnetic_variation, expected.magnetic_variation);
        assert_eq!(actual.mode, expected.mode);
    }

    #[test]
    fn rejects_invalid_rmc_talker_codes() {
        for code in ["G", "GPS", "gP", "G*"] {
            let mut rmc = complete_rmc();
            rmc.talker = Talker::Other(code.into());

            assert_err_eq!(
                Vec::<u8>::try_from(&rmc),
                EncodeError::InvalidField("talker")
            );
        }
    }

    #[test]
    fn rejects_rmc_dates_outside_nmea_century() {
        for date in [Date::new(1999, 1, 1), Date::new(2100, 1, 1)] {
            let mut rmc = complete_rmc();
            rmc.date = Some(date);

            assert_err_eq!(Vec::<u8>::try_from(&rmc), EncodeError::InvalidField("date"));
        }
    }

    #[test]
    fn rejects_invalid_rmc_modes() {
        for mode in [',', 'a', 'Ä'] {
            let mut rmc = complete_rmc();
            rmc.mode = Some(PositioningMode::Other(mode));

            assert_err_eq!(Vec::<u8>::try_from(&rmc), EncodeError::InvalidField("mode"));
        }
    }

    #[test]
    fn formats_rmc_numbers_with_default_precision() {
        assert_eq!(optional_field(Some(1.234_567)), "1.234567");
    }

    fn complete_rmc() -> Rmc {
        Rmc {
            talker: Talker::Gps,
            utc_time: Some(assert_some!(Time::from_hms_millis(13, 47, 49, 600))),
            status: RmcStatus::Active,
            position: Some(LatLon::from_degrees(48.964_695, 7.097_321_5)),
            speed_over_ground: Some(Speed::from_knots(35.9)),
            course_over_ground: Some(Angle::from_degrees(270.6)),
            date: Some(Date::new(2024, 12, 28)),
            magnetic_variation: None,
            mode: Some(PositioningMode::Differential),
        }
    }

    fn complete_gga() -> Gga {
        Gga {
            talker: Talker::Gps,
            utc_time: Some(assert_some!(Time::from_hms_millis(13, 47, 49, 600))),
            position: Some(LatLon::from_degrees(48.964_695, 7.097_321_5)),
            fix_quality: GgaFixQuality::Dgps,
            satellites_used: Some(25),
            hdop: Some(1.0),
            altitude: Some(Length::from_meters(1452.0)),
            geoid_separation: Some(Length::from_meters(47.2)),
            dgps_age: Some(1.5),
            dgps_station: Some(23),
        }
    }

    fn encode_gga_sentence(gga: &Gga) -> String {
        let sentence = assert_ok!(Vec::<u8>::try_from(gga));
        let sentence = assert_ok!(String::from_utf8(sentence));
        assert!(sentence.ends_with("\r\n"));
        sentence
    }

    fn encode_rmc_sentence(rmc: &Rmc) -> String {
        let sentence = assert_ok!(Vec::<u8>::try_from(rmc));
        let sentence = assert_ok!(String::from_utf8(sentence));
        assert!(sentence.ends_with("\r\n"));
        sentence
    }

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
