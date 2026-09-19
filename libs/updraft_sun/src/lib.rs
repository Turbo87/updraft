//! Calculates solar position and daily solar events.

use std::{error::Error, f64::consts::PI, fmt};
use time::{Date, Duration, OffsetDateTime, UtcOffset};
use updraft_geo::LatLon;
use updraft_units::Angle;

const JULIAN_DAY_UNIX_EPOCH: f64 = 2_440_587.5;
const SECONDS_PER_DAY: f64 = 86_400.0;

/// The position of the sun for an observer.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SolarPosition {
    azimuth: Angle,
    elevation: Angle,
}

impl SolarPosition {
    /// Returns the clockwise direction from true north.
    pub const fn azimuth(self) -> Angle {
        self.azimuth
    }

    /// Returns the signed angle above the astronomical horizon.
    pub const fn elevation(self) -> Angle {
        self.elevation
    }
}

/// An error for a non-finite coordinate or a latitude outside 90 degrees.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InvalidLocation;

impl fmt::Display for InvalidLocation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(
            "location must have a finite longitude and a latitude from -90 to 90 degrees",
        )
    }
}

impl Error for InvalidLocation {}

/// An error from calculating a complete set of daily solar events.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SolarEventsError {
    /// The coordinate is not valid.
    InvalidLocation,
    /// An event falls outside the supported date-time range.
    DateOutOfRange,
}

impl fmt::Display for SolarEventsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidLocation => InvalidLocation.fmt(formatter),
            Self::DateOutOfRange => {
                formatter.write_str("a solar event is outside the supported date-time range")
            }
        }
    }
}

impl Error for SolarEventsError {}

/// An event at a specified solar elevation, or the reason it does not occur.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SolarEvent {
    /// The sun crosses the specified elevation at this UTC instant.
    Occurs(OffsetDateTime),
    /// The sun remains above the specified elevation.
    AlwaysAbove,
    /// The sun remains below the specified elevation.
    AlwaysBelow,
}

/// Solar noon and the daylight and civil-twilight events for one date.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SolarEvents {
    solar_noon: OffsetDateTime,
    sunrise: SolarEvent,
    sunset: SolarEvent,
    civil_dawn: SolarEvent,
    civil_dusk: SolarEvent,
}

impl SolarEvents {
    /// Returns the local solar transit as a UTC instant.
    pub const fn solar_noon(self) -> OffsetDateTime {
        self.solar_noon
    }

    /// Returns the apparent sunrise event at -0.833 degrees elevation.
    pub const fn sunrise(self) -> SolarEvent {
        self.sunrise
    }

    /// Returns the apparent sunset event at -0.833 degrees elevation.
    pub const fn sunset(self) -> SolarEvent {
        self.sunset
    }

    /// Returns the civil dawn event at -6 degrees elevation.
    pub const fn civil_dawn(self) -> SolarEvent {
        self.civil_dawn
    }

    /// Returns the civil dusk event at -6 degrees elevation.
    pub const fn civil_dusk(self) -> SolarEvent {
        self.civil_dusk
    }
}

/// Calculates the sun's apparent azimuth and geometric elevation.
///
/// The calculation uses UTC and the NOAA solar equations. It does not apply
/// atmospheric refraction because pressure and temperature are not inputs.
pub fn solar_position(
    location: LatLon,
    instant: OffsetDateTime,
) -> Result<SolarPosition, InvalidLocation> {
    let latitude = location.latitude().as_degrees();
    let longitude = location.longitude().as_degrees();
    validate_location(latitude, longitude)?;

    let instant = instant.to_offset(UtcOffset::UTC);
    let coordinates = solar_coordinates(julian_day(instant));
    let utc_minutes = f64::from(instant.hour()) * 60.0
        + f64::from(instant.minute())
        + f64::from(instant.second()) / 60.0
        + f64::from(instant.nanosecond()) / 60_000_000_000.0;
    let true_solar_minutes =
        (utc_minutes + coordinates.equation_of_time_minutes + 4.0 * longitude).rem_euclid(1_440.0);
    let hour_angle = true_solar_minutes / 4.0 - 180.0;

    let latitude = latitude.to_radians();
    let declination = coordinates.declination_radians;
    let hour_angle = hour_angle.to_radians();
    let cosine_zenith =
        latitude.sin() * declination.sin() + latitude.cos() * declination.cos() * hour_angle.cos();
    let elevation = PI / 2.0 - cosine_zenith.clamp(-1.0, 1.0).acos();
    let azimuth = (hour_angle
        .sin()
        .atan2(hour_angle.cos() * latitude.sin() - declination.tan() * latitude.cos())
        .to_degrees()
        + 180.0)
        .rem_euclid(360.0);

    Ok(SolarPosition {
        azimuth: Angle::from_degrees(azimuth),
        elevation: Angle::from_radians(elevation),
    })
}

/// Calculates solar noon, sunrise, sunset, civil dawn, and civil dusk.
///
/// The date selects the solar transit for that date at the specified
/// longitude. Event times are UTC instants and can fall on an adjacent UTC
/// date. Sunrise and sunset use -0.833 degrees elevation. Civil twilight uses
/// -6 degrees elevation.
pub fn solar_events(location: LatLon, date: Date) -> Result<SolarEvents, SolarEventsError> {
    let latitude = location.latitude().as_degrees();
    let longitude = location.longitude().as_degrees();
    validate_location(latitude, longitude).map_err(|_| SolarEventsError::InvalidLocation)?;

    let midnight = date.midnight().assume_utc();
    let julian_day = julian_day(midnight);
    let solar_noon_minutes = solar_noon_minutes(julian_day, longitude);
    let solar_noon = instant_at_minutes(midnight, solar_noon_minutes)?;
    let (sunrise, sunset) = solar_crossings(julian_day, midnight, latitude, longitude, -0.833)?;
    let (civil_dawn, civil_dusk) =
        solar_crossings(julian_day, midnight, latitude, longitude, -6.0)?;

    Ok(SolarEvents {
        solar_noon,
        sunrise,
        sunset,
        civil_dawn,
        civil_dusk,
    })
}

#[derive(Clone, Copy)]
struct SolarCoordinates {
    declination_radians: f64,
    equation_of_time_minutes: f64,
}

fn solar_coordinates(julian_day: f64) -> SolarCoordinates {
    let century = (julian_day - 2_451_545.0) / 36_525.0;
    let mean_longitude =
        (280.466_46 + century * (36_000.769_83 + century * 0.000_303_2)).rem_euclid(360.0);
    let mean_anomaly = 357.529_11 + century * (35_999.050_29 - 0.000_153_7 * century);
    let mean_anomaly_radians = mean_anomaly.to_radians();
    let eccentricity = 0.016708634 - century * (0.000042037 + 0.0000001267 * century);
    let equation_of_center = mean_anomaly_radians.sin()
        * (1.914602 - century * (0.004817 + 0.000014 * century))
        + (2.0 * mean_anomaly_radians).sin() * (0.019993 - 0.000101 * century)
        + (3.0 * mean_anomaly_radians).sin() * 0.000289;
    let true_longitude = mean_longitude + equation_of_center;
    let omega = (125.04 - 1934.136 * century).to_radians();
    let apparent_longitude = true_longitude - 0.00569 - 0.00478 * omega.sin();
    let mean_obliquity = 23.0
        + (26.0 + (21.448 - century * (46.815 + century * (0.00059 - century * 0.001813))) / 60.0)
            / 60.0;
    let obliquity = (mean_obliquity + 0.00256 * omega.cos()).to_radians();
    let declination = (obliquity.sin() * apparent_longitude.to_radians().sin()).asin();

    let longitude_radians = mean_longitude.to_radians();
    let y = (obliquity / 2.0).tan().powi(2);
    let equation_of_time = 4.0
        * (y * (2.0 * longitude_radians).sin() - 2.0 * eccentricity * mean_anomaly_radians.sin()
            + 4.0
                * eccentricity
                * y
                * mean_anomaly_radians.sin()
                * (2.0 * longitude_radians).cos()
            - 0.5 * y.powi(2) * (4.0 * longitude_radians).sin()
            - 1.25 * eccentricity.powi(2) * (2.0 * mean_anomaly_radians).sin())
        .to_degrees();

    SolarCoordinates {
        declination_radians: declination,
        equation_of_time_minutes: equation_of_time,
    }
}

fn solar_noon_minutes(julian_day: f64, longitude: f64) -> f64 {
    let mut minutes = 720.0 - 4.0 * longitude;
    for _ in 0..2 {
        let coordinates = solar_coordinates(julian_day + minutes / 1_440.0);
        minutes = 720.0 - 4.0 * longitude - coordinates.equation_of_time_minutes;
    }
    minutes
}

fn solar_crossings(
    julian_day: f64,
    midnight: OffsetDateTime,
    latitude: f64,
    longitude: f64,
    elevation: f64,
) -> Result<(SolarEvent, SolarEvent), SolarEventsError> {
    Ok((
        solar_event(
            julian_day,
            midnight,
            latitude,
            longitude,
            elevation,
            CrossingDirection::Morning,
        )?,
        solar_event(
            julian_day,
            midnight,
            latitude,
            longitude,
            elevation,
            CrossingDirection::Evening,
        )?,
    ))
}

fn solar_event(
    julian_day: f64,
    midnight: OffsetDateTime,
    latitude: f64,
    longitude: f64,
    elevation: f64,
    direction: CrossingDirection,
) -> Result<SolarEvent, SolarEventsError> {
    let mut minutes = solar_noon_minutes(julian_day, longitude);
    for _ in 0..2 {
        let coordinates = solar_coordinates(julian_day + minutes / 1_440.0);
        let hour_angle = match hour_angle(latitude, coordinates.declination_radians, elevation) {
            Ok(hour_angle) => hour_angle,
            Err(event) => return Ok(event),
        };
        let signed_hour_angle = match direction {
            CrossingDirection::Morning => hour_angle,
            CrossingDirection::Evening => -hour_angle,
        };
        minutes =
            720.0 - 4.0 * (longitude + signed_hour_angle) - coordinates.equation_of_time_minutes;
    }
    Ok(SolarEvent::Occurs(instant_at_minutes(midnight, minutes)?))
}

#[derive(Clone, Copy)]
enum CrossingDirection {
    Morning,
    Evening,
}

fn hour_angle(latitude: f64, declination: f64, elevation: f64) -> Result<f64, SolarEvent> {
    let latitude = latitude.to_radians();
    let cosine = (elevation.to_radians().sin() - latitude.sin() * declination.sin())
        / (latitude.cos() * declination.cos());
    if cosine > 1.0 {
        Err(SolarEvent::AlwaysBelow)
    } else if cosine < -1.0 {
        Err(SolarEvent::AlwaysAbove)
    } else {
        Ok(cosine.acos().to_degrees())
    }
}

fn instant_at_minutes(
    midnight: OffsetDateTime,
    minutes: f64,
) -> Result<OffsetDateTime, SolarEventsError> {
    midnight
        .checked_add(Duration::seconds_f64(minutes * 60.0))
        .ok_or(SolarEventsError::DateOutOfRange)
}

fn julian_day(instant: OffsetDateTime) -> f64 {
    instant.unix_timestamp_nanos() as f64 / 1_000_000_000.0 / SECONDS_PER_DAY
        + JULIAN_DAY_UNIX_EPOCH
}

fn validate_location(latitude: f64, longitude: f64) -> Result<(), InvalidLocation> {
    if latitude.is_finite() && (-90.0..=90.0).contains(&latitude) && longitude.is_finite() {
        Ok(())
    } else {
        Err(InvalidLocation)
    }
}
