//! Date and time of day carried by NMEA sentences.

use crate::encode::EncodeError;
use std::fmt;

const MILLISECONDS_PER_SECOND: u32 = 1_000;
const MILLISECONDS_PER_MINUTE: u32 = 60 * MILLISECONDS_PER_SECOND;
const MILLISECONDS_PER_HOUR: u32 = 60 * MILLISECONDS_PER_MINUTE;
const MILLISECONDS_PER_DAY: u32 = 24 * MILLISECONDS_PER_HOUR;

/// A UTC time of day with millisecond precision and no date.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Time {
    milliseconds_since_midnight: u32,
}

impl fmt::Debug for Time {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "Time({:02}:{:02}:{:02}.{:03})",
            self.hour(),
            self.minute(),
            self.second(),
            self.millisecond()
        )
    }
}

impl Time {
    /// Constructs a valid UTC time from clock components.
    ///
    /// `23:59:60` is accepted for a positive leap second.
    pub const fn from_hms_millis(
        hour: u8,
        minute: u8,
        second: u8,
        millisecond: u16,
    ) -> Option<Self> {
        let regular_time = hour < 24 && minute < 60 && second < 60;
        let leap_second = hour == 23 && minute == 59 && second == 60;
        if (!regular_time && !leap_second) || millisecond >= 1_000 {
            return None;
        }

        Some(Self {
            milliseconds_since_midnight: hour as u32 * MILLISECONDS_PER_HOUR
                + minute as u32 * MILLISECONDS_PER_MINUTE
                + second as u32 * MILLISECONDS_PER_SECOND
                + millisecond as u32,
        })
    }

    /// Parses `HHMMSS[.fraction]`, truncating fractional seconds to milliseconds.
    pub fn parse(field: &[u8]) -> Option<Self> {
        let hour = btoi::btou(field.get(0..2)?).ok()?;
        let minute = btoi::btou(field.get(2..4)?).ok()?;
        let second = btoi::btou(field.get(4..6)?).ok()?;
        let millisecond = match field.get(6..)? {
            [] => 0,
            [b'.', fraction @ ..] if !fraction.is_empty() => parse_milliseconds(fraction)?,
            _ => return None,
        };
        Self::from_hms_millis(hour, minute, second, millisecond)
    }

    /// Returns the hour component in `0..=23`.
    pub const fn hour(self) -> u8 {
        if self.milliseconds_since_midnight >= MILLISECONDS_PER_DAY {
            23
        } else {
            (self.milliseconds_since_midnight / MILLISECONDS_PER_HOUR) as u8
        }
    }

    /// Returns the minute component in `0..=59`.
    pub const fn minute(self) -> u8 {
        if self.milliseconds_since_midnight >= MILLISECONDS_PER_DAY {
            59
        } else {
            ((self.milliseconds_since_midnight / MILLISECONDS_PER_MINUTE) % 60) as u8
        }
    }

    /// Returns the second component in `0..=60`.
    pub const fn second(self) -> u8 {
        if self.milliseconds_since_midnight >= MILLISECONDS_PER_DAY {
            60
        } else {
            ((self.milliseconds_since_midnight / MILLISECONDS_PER_SECOND) % 60) as u8
        }
    }

    /// Returns the millisecond component in `0..=999`.
    pub const fn millisecond(self) -> u16 {
        (self.milliseconds_since_midnight % MILLISECONDS_PER_SECOND) as u16
    }

    /// Returns milliseconds since the start of the UTC day.
    ///
    /// A positive leap second occupies values `86_400_000..=86_400_999`.
    pub const fn milliseconds_since_midnight(self) -> u32 {
        self.milliseconds_since_midnight
    }

    /// Formats this time as an `HHMMSS.sss` NMEA field.
    pub fn to_nmea_field(self) -> String {
        format!(
            "{:02}{:02}{:02}.{:03}",
            self.hour(),
            self.minute(),
            self.second(),
            self.millisecond()
        )
    }
}

fn parse_milliseconds(fraction: &[u8]) -> Option<u16> {
    if !fraction.iter().all(u8::is_ascii_digit) {
        return None;
    }

    let hundreds = fraction.first().map_or(0, |digit| digit - b'0');
    let tens = fraction.get(1).map_or(0, |digit| digit - b'0');
    let ones = fraction.get(2).map_or(0, |digit| digit - b'0');
    Some(u16::from(hundreds) * 100 + u16::from(tens) * 10 + u16::from(ones))
}

/// A calendar date, with the two-digit year taken as 20xx.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Date {
    pub year: u16,
    pub month: u8,
    pub day: u8,
}

impl Date {
    pub fn new(year: u16, month: u8, day: u8) -> Self {
        Self { year, month, day }
    }

    /// Parse `DDMMYY` into a `Date` struct. The field must be exactly six
    /// digits, and values that cannot represent a real calendar date read as
    /// absent.
    pub fn parse_ddmmyy(field: &[u8]) -> Option<Self> {
        if field.len() != 6 {
            return None;
        }
        let day = btoi::btou(field.get(0..2)?).ok()?;
        let month = btoi::btou(field.get(2..4)?).ok()?;
        let year = 2000 + btoi::btou::<u16>(field.get(4..6)?).ok()?;
        ((1..=12).contains(&month) && (1..=days_in_month(year, month)).contains(&day))
            .then_some(Self { year, month, day })
    }

    /// Formats this date as a `DDMMYY` NMEA field.
    ///
    /// Returns an error for a year outside `2000..=2099`.
    pub fn to_nmea_field(self) -> Result<String, EncodeError> {
        if !(2000..=2099).contains(&self.year) {
            return Err(EncodeError::InvalidField("date"));
        }

        Ok(format!(
            "{:02}{:02}{:02}",
            self.day,
            self.month,
            self.year % 100
        ))
    }
}

/// The number of days in `month` of `year`, or `0` for a month outside
/// `1..=12`.
fn days_in_month(year: u16, month: u8) -> u8 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => 0,
    }
}

fn is_leap_year(year: u16) -> bool {
    (year.is_multiple_of(4) && !year.is_multiple_of(100)) || year.is_multiple_of(400)
}

#[cfg(test)]
mod tests {
    use super::*;
    use claims::{assert_gt, assert_lt, assert_none, assert_ok, assert_some, assert_some_eq};

    #[test]
    fn parses_time() {
        let end_of_day = assert_some!(Time::from_hms_millis(23, 59, 59, 0));
        let afternoon = assert_some!(Time::from_hms_millis(13, 47, 49, 600));
        assert_some_eq!(Time::parse(b"235959"), end_of_day);
        assert_some_eq!(Time::parse(b"134749.60"), afternoon);
    }

    #[test]
    fn normalizes_fractional_seconds_to_milliseconds() {
        let expected = assert_some!(Time::from_hms_millis(13, 47, 49, 600));
        assert_some_eq!(Time::parse(b"134749.6"), expected);
        assert_some_eq!(Time::parse(b"134749.60"), expected);
        assert_some_eq!(Time::parse(b"134749.600"), expected);
        assert_some_eq!(Time::parse(b"134749.6009"), expected);
    }

    #[test]
    fn exposes_time_components() {
        let time = assert_some!(Time::from_hms_millis(13, 47, 49, 605));
        assert_eq!(time.hour(), 13);
        assert_eq!(time.minute(), 47);
        assert_eq!(time.second(), 49);
        assert_eq!(time.millisecond(), 605);
        assert_eq!(time.milliseconds_since_midnight(), 49_669_605);
    }

    #[test]
    fn formats_time_debug_as_clock_time() {
        let time = assert_some!(Time::from_hms_millis(12, 34, 56, 789));
        insta::assert_debug_snapshot!(time, @"Time(12:34:56.789)");
    }

    #[test]
    fn compares_normalized_times() {
        let earlier = assert_some!(Time::parse(b"134749.6009"));
        let same = assert_some!(Time::parse(b"134749.6001"));
        let later = assert_some!(Time::parse(b"134749.601"));
        assert_eq!(earlier, same);
        assert_lt!(earlier, later);
    }

    #[test]
    fn accepts_leap_second() {
        let leap_second = assert_some!(Time::parse(b"235960.123"));
        assert_some_eq!(Time::from_hms_millis(23, 59, 60, 123), leap_second);
        assert_eq!(leap_second.hour(), 23);
        assert_eq!(leap_second.minute(), 59);
        assert_eq!(leap_second.second(), 60);
        assert_eq!(leap_second.millisecond(), 123);
        let previous = assert_some!(Time::from_hms_millis(23, 59, 59, 999));
        assert_gt!(leap_second, previous);
    }

    #[test]
    fn rejects_too_short_time() {
        assert_none!(Time::parse(b"1347"));
    }

    #[test]
    fn rejects_out_of_range_time() {
        assert_none!(Time::parse(b"245959"));
        assert_none!(Time::parse(b"236059"));
        assert_none!(Time::parse(b"235999"));
        assert_none!(Time::parse(b"125960"));
        assert_none!(Time::from_hms_millis(23, 59, 59, 1000));
    }

    #[test]
    fn rejects_non_numeric_seconds() {
        assert_none!(Time::parse(b"2359-9"));
        assert_none!(Time::parse(b"2359in"));
        assert_none!(Time::parse(b"235959."));
        assert_none!(Time::parse(b"235959.12x"));
    }

    #[test]
    fn parses_date() {
        assert_some_eq!(Date::parse_ddmmyy(b"281224"), Date::new(2024, 12, 28));
    }

    #[test]
    fn formats_date_for_nmea() {
        let date = Date::new(2024, 12, 28);
        let field = assert_ok!(date.to_nmea_field());

        assert_eq!(field, "281224");
        assert_some_eq!(Date::parse_ddmmyy(field.as_bytes()), date);
    }

    #[test]
    fn rejects_out_of_range_date() {
        assert_none!(Date::parse_ddmmyy(b"001224"));
        assert_none!(Date::parse_ddmmyy(b"321224"));
        assert_none!(Date::parse_ddmmyy(b"280024"));
        assert_none!(Date::parse_ddmmyy(b"281324"));
    }

    #[test]
    fn rejects_wrong_length_date() {
        assert_none!(Date::parse_ddmmyy(b"28122"));
        assert_none!(Date::parse_ddmmyy(b"2812249"));
    }

    #[test]
    fn rejects_impossible_calendar_dates() {
        assert_none!(Date::parse_ddmmyy(b"300224")); // Feb 30
        assert_none!(Date::parse_ddmmyy(b"310424")); // Apr 31
        assert_none!(Date::parse_ddmmyy(b"290223")); // Feb 29 of a common year
    }

    #[test]
    fn accepts_leap_day() {
        assert_some_eq!(Date::parse_ddmmyy(b"290224"), Date::new(2024, 2, 29));
    }
}
