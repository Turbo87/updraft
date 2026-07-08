//! `GGA` — Global Positioning System Fix Data: the primary position,
//! fix-quality, and altitude sentence.

use updraft_geo::LatLon;
use updraft_units::Length;

use crate::datetime::Time;
use crate::error::ParseError;
use crate::fields::Fields;
use crate::scalars;

/// A parsed `GGA` sentence.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Gga {
    /// UTC time of the fix, if present.
    pub time: Option<Time>,
    /// Fix position, or `None` when the receiver has no fix.
    pub position: Option<LatLon>,
    /// The fix quality / positioning mode.
    pub fix_quality: FixQuality,
    /// Number of satellites used in the fix.
    pub satellites: Option<u8>,
    /// Horizontal dilution of precision.
    pub hdop: Option<f64>,
    /// Antenna altitude above mean sea level (the geoid).
    pub altitude: Option<Length>,
    /// Height of the geoid above the WGS84 ellipsoid.
    pub geoid_separation: Option<Length>,
}

/// The GGA fix-quality indicator (field 6).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum FixQuality {
    /// No fix (`0`).
    Invalid,
    /// Autonomous GPS fix (`1`).
    Gps,
    /// Differential GPS fix (`2`).
    DifferentialGps,
    /// Precise Positioning Service fix (`3`).
    Pps,
    /// Real-time kinematic, fixed integers (`4`).
    RealTimeKinematic,
    /// Real-time kinematic, float solution (`5`).
    FloatRtk,
    /// Dead-reckoning / estimated (`6`).
    DeadReckoning,
    /// Manual input mode (`7`).
    Manual,
    /// Simulation mode (`8`).
    Simulation,
}

impl FixQuality {
    fn from_code(code: u8) -> Result<Self, ParseError> {
        Ok(match code {
            0 => Self::Invalid,
            1 => Self::Gps,
            2 => Self::DifferentialGps,
            3 => Self::Pps,
            4 => Self::RealTimeKinematic,
            5 => Self::FloatRtk,
            6 => Self::DeadReckoning,
            7 => Self::Manual,
            8 => Self::Simulation,
            _ => return Err(ParseError::InvalidField),
        })
    }
}

pub(crate) fn parse(mut fields: Fields<'_>) -> Result<Gga, ParseError> {
    let time = scalars::opt(fields.next_required()?, Time::parse)?;
    let position = scalars::position(&mut fields)?;
    let fix_quality = FixQuality::from_code(scalars::u8(fields.next_required()?)?)?;
    let satellites = scalars::opt_u8(fields.next_required()?)?;
    let hdop = scalars::opt_f64(fields.next_required()?)?;
    let altitude = scalars::opt_f64(fields.next_required()?)?.map(Length::from_meters);
    let _altitude_unit = fields.next_required()?; // always "M"
    let geoid_separation = scalars::opt_f64(fields.next_required()?)?.map(Length::from_meters);
    // The remaining fields (geoid unit, DGPS age, DGPS station id) are not
    // needed by the glide computer and are left unparsed.
    Ok(Gga {
        time,
        position,
        fix_quality,
        satellites,
        hdop,
        altitude,
        geoid_separation,
    })
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;

    use super::*;
    use crate::{ParseResult, parse};

    #[test]
    fn full_fix_from_flight_corpus() {
        let result =
            parse("$GPGGA,134749.60,4857.88170,N,00705.83929,E,2,25,1.00,1452.0,M,47.2,M,,*63")
                .unwrap();
        let ParseResult::Gga(gga) = result else {
            panic!("expected GGA, got {result:?}");
        };

        assert_eq!(
            gga.time,
            Some(Time {
                hour: 13,
                minute: 47,
                seconds: 49.60,
            })
        );
        let position = gga.position.unwrap();
        assert_relative_eq!(position.latitude().as_degrees(), 48.0 + 57.88170 / 60.0);
        assert_relative_eq!(position.longitude().as_degrees(), 7.0 + 5.83929 / 60.0);
        assert_eq!(gga.fix_quality, FixQuality::DifferentialGps);
        assert_eq!(gga.satellites, Some(25));
        assert_relative_eq!(gga.hdop.unwrap(), 1.00);
        assert_relative_eq!(gga.altitude.unwrap().as_meters(), 1452.0);
        assert_relative_eq!(gga.geoid_separation.unwrap().as_meters(), 47.2);
    }

    /// A no-fix sentence with empty optional fields, cross-checked against
    /// the `nmea` crate's own test corpus (checksum `4F`).
    #[test]
    fn no_fix_with_empty_fields() {
        let result = parse("$GPGGA,133605.0,5521.75946,N,03731.93769,E,0,00,,,M,,M,,*4F").unwrap();
        let ParseResult::Gga(gga) = result else {
            panic!("expected GGA, got {result:?}");
        };

        assert_eq!(gga.fix_quality, FixQuality::Invalid);
        assert_eq!(gga.satellites, Some(0));
        assert_eq!(gga.hdop, None);
        assert_eq!(gga.altitude, None);
        assert_eq!(gga.geoid_separation, None);
        // Position is still reported even with a `0` fix quality.
        assert!(gga.position.is_some());
    }

    #[test]
    fn any_gnss_talker_is_accepted() {
        // GLONASS/GNSS/BeiDou talkers route to the same parser.
        for line in [
            "$GNGGA,134749.60,4857.88170,N,00705.83929,E,2,25,1.00,1452.0,M,47.2,M,,*7D",
            "$GLGGA,134749.60,4857.88170,N,00705.83929,E,2,25,1.00,1452.0,M,47.2,M,,*7F",
        ] {
            assert!(matches!(parse(line), Ok(ParseResult::Gga(_))), "{line}");
        }
    }

    #[test]
    fn rejects_bad_fix_quality() {
        // Fix quality 9 is out of range.
        let line = "$GPGGA,134749.60,4857.88170,N,00705.83929,E,9,25,1.00,1452.0,M,47.2,M,,*68";
        assert_eq!(parse(line), Err(ParseError::InvalidField));
    }
}
