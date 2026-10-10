use serde::{Deserialize, Serialize};

/// The trail length in whole minutes, from 0 to 1440. 0 turns the trail off.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct TrailLength(u16);

impl<'de> Deserialize<'de> for TrailLength {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        u16::deserialize(deserializer)?
            .try_into()
            .map_err(serde::de::Error::custom)
    }
}

impl TrailLength {
    const MAX_MINUTES: u16 = 24 * 60;

    pub fn minutes(self) -> u16 {
        self.0
    }
}

impl Default for TrailLength {
    fn default() -> Self {
        Self(60)
    }
}

impl TryFrom<u16> for TrailLength {
    type Error = InvalidTrailLength;

    fn try_from(minutes: u16) -> Result<Self, Self::Error> {
        if minutes <= Self::MAX_MINUTES {
            Ok(Self(minutes))
        } else {
            Err(InvalidTrailLength)
        }
    }
}

#[derive(Debug, thiserror::Error)]
#[error("Trail length must be from 0 to 1440 minutes")]
pub struct InvalidTrailLength;

#[cfg(test)]
mod tests {
    use super::*;
    use claims::{assert_err, assert_ok};

    #[test]
    fn length_accepts_minutes_up_to_one_day() {
        for minutes in [0, 60, 1440] {
            let length = assert_ok!(TrailLength::try_from(minutes));
            assert_eq!(length.minutes(), minutes);
        }
    }

    #[test]
    fn length_rejects_more_than_one_day() {
        for minutes in [1441, u16::MAX] {
            assert_err!(TrailLength::try_from(minutes));
        }
    }
}
