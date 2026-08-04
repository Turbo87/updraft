use std::fmt;

/// A validated four-digit octal transponder code.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AirspaceTransponderCode(u16);

impl AirspaceTransponderCode {
    /// Creates a four-digit code by adding leading zeroes to valid octal digits.
    pub fn from_octal_digits(code: u16) -> Option<Self> {
        let mut remaining = code;
        for _ in 0..4 {
            if remaining % 10 > 7 {
                return None;
            }
            remaining /= 10;
        }

        (remaining == 0).then_some(Self(code))
    }
}

impl fmt::Display for AirspaceTransponderCode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{:04}", self.0)
    }
}

/// One canonical airspace transponder setting.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AirspaceTransponderSetting {
    /// The validated four-digit octal code.
    pub code: AirspaceTransponderCode,
    /// Whether this is the primary transponder setting.
    pub primary: bool,
    /// Additional source remarks when present.
    pub remarks: Option<Box<str>>,
}
