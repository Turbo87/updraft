use updraft_geo::LatLon;
use updraft_nmea::{Date as NmeaDate, Time as NmeaTime};
use updraft_units::{Angle, EllipsoidAltitude, Speed};

/// A UTC instant stored as Unix epoch milliseconds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UtcInstant(i64);

impl UtcInstant {
    /// Creates a UTC instant from Unix epoch milliseconds.
    pub const fn from_unix_milliseconds(unix_milliseconds: i64) -> Self {
        Self(unix_milliseconds)
    }

    /// Creates an instant from an offset date-time.
    pub fn from_offset_date_time(value: time::OffsetDateTime) -> Self {
        let milliseconds = value.unix_timestamp_nanos().div_euclid(1_000_000);
        Self(i64::try_from(milliseconds).expect("an OffsetDateTime fits in Unix milliseconds"))
    }

    /// Converts a valid NMEA date and time to a UTC instant.
    pub fn from_nmea_date_time(date: NmeaDate, time: NmeaTime) -> Option<Self> {
        let month = time::Month::try_from(date.month).ok()?;
        let date = time::Date::from_calendar_date(i32::from(date.year), month, date.day).ok()?;
        let time = time::Time::from_hms_milli(
            time.hour(),
            time.minute(),
            time.second().min(59),
            time.millisecond(),
        )
        .ok()?;
        let unix_seconds = date.with_time(time).assume_utc().unix_timestamp();
        let unix_milliseconds = unix_seconds * 1_000 + i64::from(time.millisecond());
        Some(Self::from_unix_milliseconds(unix_milliseconds))
    }

    /// Returns Unix epoch milliseconds.
    pub const fn unix_milliseconds(self) -> i64 {
        self.0
    }

    /// Returns the instant within 12 hours of this one that has the given
    /// time of day.
    pub fn nearest_with_time_of_day(self, time: UtcTime) -> Self {
        const DAY: i64 = 86_400_000;
        let time = i64::from(time.milliseconds_since_midnight());
        let offset = (time - self.0.rem_euclid(DAY) + DAY / 2).rem_euclid(DAY) - DAY / 2;
        Self(self.0.saturating_add(offset))
    }

    /// Advances the instant by monotonic elapsed time.
    pub fn saturating_add(self, duration: std::time::Duration) -> Self {
        let milliseconds = i64::try_from(duration.as_millis()).unwrap_or(i64::MAX);
        Self(self.0.saturating_add(milliseconds))
    }
}

/// A UTC time of day stored as milliseconds since midnight.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UtcTime(u32);

impl UtcTime {
    const MILLISECONDS_PER_DAY: u32 = 86_400_000;

    /// Creates a UTC time when the value is within one regular UTC day.
    pub const fn from_milliseconds_since_midnight(
        milliseconds_since_midnight: u32,
    ) -> Option<Self> {
        if milliseconds_since_midnight < Self::MILLISECONDS_PER_DAY {
            Some(Self(milliseconds_since_midnight))
        } else {
            None
        }
    }

    /// Converts an NMEA time and maps a leap second to `23:59:59`.
    pub fn from_nmea_time(time: NmeaTime) -> Self {
        Self(
            time.hour() as u32 * 3_600_000
                + time.minute() as u32 * 60_000
                + time.second().min(59) as u32 * 1_000
                + time.millisecond() as u32,
        )
    }

    /// Returns milliseconds since midnight UTC.
    pub const fn milliseconds_since_midnight(self) -> u32 {
        self.0
    }
}

/// A canonical GPS fix time.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FixTime {
    UtcInstant(UtcInstant),
    UtcTimeOfDay(UtcTime),
}

/// A position report from the device's own GNSS receiver.
///
/// Distinct from a fix decoded out of NMEA: it arrives already structured,
/// from a source the operating system vouches for, so it never passes
/// through [`crate::Decoder`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Fix {
    pub position: LatLon,
    pub altitude_ellipsoid: Option<EllipsoidAltitude>,
    pub track: Option<Angle>,
    pub ground_speed: Option<Speed>,
    pub fix_time: Option<UtcInstant>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use claims::{assert_none, assert_some};

    #[test]
    fn utc_time_rejects_the_next_midnight() {
        let last_millisecond = assert_some!(UtcTime::from_milliseconds_since_midnight(86_399_999));
        assert_eq!(last_millisecond.milliseconds_since_midnight(), 86_399_999);
        assert_none!(UtcTime::from_milliseconds_since_midnight(86_400_000));
    }

    #[test]
    fn nearest_time_of_day_crosses_midnight() {
        // 2026-01-01T23:59:59.000Z
        let before_midnight = UtcInstant::from_unix_milliseconds(1_767_311_999_000);
        let after_midnight = assert_some!(UtcTime::from_milliseconds_since_midnight(500));
        assert_eq!(
            before_midnight.nearest_with_time_of_day(after_midnight),
            UtcInstant::from_unix_milliseconds(1_767_312_000_500)
        );
        let midnight = UtcInstant::from_unix_milliseconds(1_767_312_000_000);
        let last_second = assert_some!(UtcTime::from_milliseconds_since_midnight(86_399_000));
        assert_eq!(
            midnight.nearest_with_time_of_day(last_second),
            before_midnight
        );
    }
}
