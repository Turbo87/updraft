//! `RMC` — Recommended Minimum specific GNSS data: time, date, position,
//! ground speed, and track in one sentence.

use updraft_geo::LatLon;
use updraft_units::{Angle, Speed};

use crate::datetime::{Date, Time};
use crate::error::ParseError;
use crate::fields::Fields;
use crate::scalars;

/// A parsed `RMC` sentence.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Rmc {
    /// UTC time of the fix, if present.
    pub time: Option<Time>,
    /// Whether the data is valid (`A`) or a void navigation warning (`V`).
    pub status: RmcStatus,
    /// Fix position, or `None` when the receiver has no fix.
    pub position: Option<LatLon>,
    /// Speed over ground.
    pub speed_over_ground: Option<Speed>,
    /// Course over ground, degrees true.
    pub track: Option<Angle>,
    /// UTC date of the fix, if present.
    pub date: Option<Date>,
    /// Magnetic variation, signed east-positive / west-negative.
    pub magnetic_variation: Option<Angle>,
}

/// The RMC data-validity status (field 2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum RmcStatus {
    /// `A` — data valid.
    Active,
    /// `V` — navigation receiver warning, data void.
    Void,
}

impl RmcStatus {
    fn parse(field: &str) -> Result<Self, ParseError> {
        match field {
            "A" => Ok(Self::Active),
            "V" => Ok(Self::Void),
            _ => Err(ParseError::InvalidField),
        }
    }
}

/// Consume the `value, E/W` magnetic-variation pair into a signed angle.
fn magnetic_variation(fields: &mut Fields<'_>) -> Result<Option<Angle>, ParseError> {
    let value = fields.next_required()?;
    let direction = fields.next_required()?;
    if value.is_empty() {
        return Ok(None);
    }
    let magnitude = scalars::f64(value)?;
    let sign = match direction {
        "E" => 1.0,
        "W" => -1.0,
        _ => return Err(ParseError::InvalidField),
    };
    Ok(Some(Angle::from_degrees(sign * magnitude)))
}

pub(crate) fn parse(mut fields: Fields<'_>) -> Result<Rmc, ParseError> {
    let time = scalars::opt(fields.next_required()?, Time::parse)?;
    let status = RmcStatus::parse(fields.next_required()?)?;
    let position = scalars::position(&mut fields)?;
    let speed_over_ground = scalars::opt_f64(fields.next_required()?)?.map(Speed::from_knots);
    let track = scalars::opt_f64(fields.next_required()?)?.map(Angle::from_degrees);
    let date = scalars::opt(fields.next_required()?, Date::parse)?;
    let magnetic_variation = magnetic_variation(&mut fields)?;
    // The remaining fields (FAA mode indicator, navigation status) are not
    // needed by the glide computer and are left unparsed.
    Ok(Rmc {
        time,
        status,
        position,
        speed_over_ground,
        track,
        date,
        magnetic_variation,
    })
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;

    use super::*;
    use crate::{ParseResult, parse};

    #[test]
    fn active_fix_from_flight_corpus() {
        let result =
            parse("$GPRMC,134749.60,A,4857.88170,N,00705.83929,E,35.9,270.6,281224,,,D*61")
                .unwrap();
        let ParseResult::Rmc(rmc) = result else {
            panic!("expected RMC, got {result:?}");
        };

        assert_eq!(rmc.status, RmcStatus::Active);
        let position = rmc.position.unwrap();
        assert_relative_eq!(position.latitude().as_degrees(), 48.0 + 57.88170 / 60.0);
        assert_relative_eq!(position.longitude().as_degrees(), 7.0 + 5.83929 / 60.0);
        assert_relative_eq!(rmc.speed_over_ground.unwrap().as_knots(), 35.9);
        assert_relative_eq!(rmc.track.unwrap().as_degrees(), 270.6);
        assert_eq!(
            rmc.date,
            Some(Date {
                year: 2024,
                month: 12,
                day: 28,
            })
        );
        assert_eq!(rmc.magnetic_variation, None);
    }

    /// Pre-2.3 sentence with a west longitude and east magnetic variation,
    /// cross-checked against the `nmea` crate's test corpus (checksum `2B`).
    #[test]
    fn signed_longitude_and_variation() {
        let result =
            parse("$GPRMC,225446.33,A,4916.45,N,12311.12,W,000.5,054.7,191194,020.3,E,A*2B")
                .unwrap();
        let ParseResult::Rmc(rmc) = result else {
            panic!("expected RMC, got {result:?}");
        };

        let position = rmc.position.unwrap();
        assert_relative_eq!(position.latitude().as_degrees(), 49.0 + 16.45 / 60.0);
        assert_relative_eq!(position.longitude().as_degrees(), -(123.0 + 11.12 / 60.0));
        assert_relative_eq!(rmc.speed_over_ground.unwrap().as_knots(), 0.5);
        assert_relative_eq!(rmc.magnetic_variation.unwrap().as_degrees(), 20.3);
        assert_eq!(rmc.date.unwrap().year, 1994);
    }

    #[test]
    fn void_status_without_fix() {
        let result = parse("$GPRMC,,V,,,,,,,,,,N*53").unwrap();
        let ParseResult::Rmc(rmc) = result else {
            panic!("expected RMC, got {result:?}");
        };

        assert_eq!(rmc.status, RmcStatus::Void);
        assert_eq!(rmc.time, None);
        assert_eq!(rmc.position, None);
        assert_eq!(rmc.speed_over_ground, None);
        assert_eq!(rmc.date, None);
    }
}
