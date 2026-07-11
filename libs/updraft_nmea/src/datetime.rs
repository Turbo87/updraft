//! Date and time of day carried by NMEA sentences, kept exactly as
//! transmitted.

use crate::field::parse_from_utf8;

/// A time of day, as transmitted, without timezone.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Time {
    pub hour: u8,
    pub minute: u8,
    pub seconds: f32,
}

impl Time {
    pub fn new(hour: u8, minute: u8, seconds: f32) -> Self {
        Self {
            hour,
            minute,
            seconds,
        }
    }

    /// Parse `HHMMSS[.SS]` into a `Time` struct. Values that cannot represent
    /// a real time of day read as absent. The seconds bound leaves room for a
    /// positive leap second (`23:59:60`).
    pub fn parse(field: &[u8]) -> Option<Self> {
        let hour = parse_from_utf8(field.get(0..2)?)?;
        let minute = parse_from_utf8(field.get(2..4)?)?;
        let seconds = fast_float2::parse(field.get(4..)?).ok()?;
        (hour <= 23 && minute <= 59 && (0.0..61.0).contains(&seconds)).then_some(Self {
            hour,
            minute,
            seconds,
        })
    }
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
        let day = parse_from_utf8(field.get(0..2)?)?;
        let month = parse_from_utf8(field.get(2..4)?)?;
        let year = 2000 + parse_from_utf8::<u16>(field.get(4..6)?)?;
        ((1..=12).contains(&month) && (1..=days_in_month(year, month)).contains(&day))
            .then_some(Self { year, month, day })
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
    use claims::{assert_none, assert_some_eq};

    #[test]
    fn parses_time() {
        assert_some_eq!(Time::parse(b"235959"), Time::new(23, 59, 59.0));
        assert_some_eq!(Time::parse(b"134749.60"), Time::new(13, 47, 49.60));
    }

    #[test]
    fn accepts_leap_second() {
        assert_some_eq!(Time::parse(b"235960"), Time::new(23, 59, 60.0));
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
    }

    #[test]
    fn rejects_non_numeric_seconds() {
        assert_none!(Time::parse(b"2359-9"));
        assert_none!(Time::parse(b"2359in"));
    }

    #[test]
    fn parses_date() {
        assert_some_eq!(Date::parse_ddmmyy(b"281224"), Date::new(2024, 12, 28));
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
