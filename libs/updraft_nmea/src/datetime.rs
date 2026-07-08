use crate::error::ParseError;

/// A UTC wall-clock time of day, as carried by NMEA `hhmmss.sss` fields.
///
/// The crate deliberately keeps date and time as plain calendar/clock
/// components rather than a single timestamp type: NMEA sentences report
/// them separately and with no time zone, and combining them into an
/// instant is the caller's concern (the `core-time` step). No range
/// validation is performed beyond what the field layout implies.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Time {
    /// Hours, `0..=23` for well-formed input.
    pub hour: u8,
    /// Minutes, `0..=59` for well-formed input.
    pub minute: u8,
    /// Seconds including any fractional part.
    pub seconds: f64,
}

impl Time {
    pub(crate) fn parse(field: &str) -> Result<Self, ParseError> {
        let hour = field
            .get(..2)
            .ok_or(ParseError::InvalidField)?
            .parse()
            .map_err(|_| ParseError::InvalidNumber)?;
        let minute = field
            .get(2..4)
            .ok_or(ParseError::InvalidField)?
            .parse()
            .map_err(|_| ParseError::InvalidNumber)?;
        let seconds = field
            .get(4..)
            .filter(|rest| !rest.is_empty())
            .ok_or(ParseError::InvalidField)?
            .parse()
            .map_err(|_| ParseError::InvalidNumber)?;
        Ok(Self {
            hour,
            minute,
            seconds,
        })
    }
}

/// A UTC calendar date, as carried by NMEA `ddmmyy` fields.
///
/// The two-digit year is expanded with the common pivot: `00..=69` maps to
/// `2000..=2069` and `70..=99` to `1970..=1999`. See [`Time`] for why date
/// and time stay separate components here.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Date {
    /// Four-digit year (the two-digit field expanded via the pivot).
    pub year: u16,
    /// Month, `1..=12` for well-formed input.
    pub month: u8,
    /// Day of month, `1..=31` for well-formed input.
    pub day: u8,
}

impl Date {
    pub(crate) fn parse(field: &str) -> Result<Self, ParseError> {
        let day = field
            .get(..2)
            .ok_or(ParseError::InvalidField)?
            .parse()
            .map_err(|_| ParseError::InvalidNumber)?;
        let month = field
            .get(2..4)
            .ok_or(ParseError::InvalidField)?
            .parse()
            .map_err(|_| ParseError::InvalidNumber)?;
        let year_of_century: u16 = field
            .get(4..6)
            .ok_or(ParseError::InvalidField)?
            .parse()
            .map_err(|_| ParseError::InvalidNumber)?;
        let year = if year_of_century < 70 {
            2000 + year_of_century
        } else {
            1900 + year_of_century
        };
        Ok(Self { year, month, day })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_fractional_seconds() {
        assert_eq!(
            Time::parse("134749.60").unwrap(),
            Time {
                hour: 13,
                minute: 47,
                seconds: 49.60,
            }
        );
    }

    #[test]
    fn rejects_truncated_time() {
        assert_eq!(Time::parse("1347"), Err(ParseError::InvalidField));
    }

    #[test]
    fn expands_two_digit_year() {
        assert_eq!(
            Date::parse("281224").unwrap(),
            Date {
                year: 2024,
                month: 12,
                day: 28,
            }
        );
        // 94 -> 1994 via the 70-year pivot.
        assert_eq!(Date::parse("191194").unwrap().year, 1994);
    }
}
