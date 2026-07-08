//! Types shared by the FLARM sentences (`PFLAU`, `PFLAA`).

use crate::error::ParseError;

/// A FLARM collision-alarm level, shared by `PFLAU` (the highest current
/// level) and `PFLAA` (per-target level).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum AlarmLevel {
    /// `0` — no alarm.
    None,
    /// `1` — low-level alarm (earliest warning).
    Low,
    /// `2` — important alarm.
    Important,
    /// `3` — urgent alarm (latest, closest warning).
    Urgent,
}

impl AlarmLevel {
    pub(crate) fn parse(field: &str) -> Result<Self, ParseError> {
        match field {
            "0" => Ok(Self::None),
            "1" => Ok(Self::Low),
            "2" => Ok(Self::Important),
            "3" => Ok(Self::Urgent),
            _ => Err(ParseError::InvalidField),
        }
    }
}
