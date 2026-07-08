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
}
