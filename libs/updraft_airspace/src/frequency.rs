use std::fmt;

/// An exact MHz frequency that formats as the OpenAIP `ddd.ddd` value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AirspaceFrequencyValue(u32);

impl AirspaceFrequencyValue {
    /// Parses a MHz value with at most three digits on each side of the decimal point.
    pub fn from_megahertz(value: &str) -> Option<Self> {
        let (whole, fractional) = match value.split_once('.') {
            Some((_, "")) => return None,
            Some(parts) => parts,
            None => (value, ""),
        };

        if whole.is_empty()
            || whole.len() > 3
            || fractional.len() > 3
            || !whole.bytes().all(|byte| byte.is_ascii_digit())
            || !fractional.bytes().all(|byte| byte.is_ascii_digit())
        {
            return None;
        }

        let fractional_digits = fractional.len() as u32;
        let whole = whole.parse::<u32>().ok()?;
        let fractional = match fractional {
            "" => 0,
            value => value.parse::<u32>().ok()?,
        };
        let scale = 10_u32.pow(3 - fractional_digits);
        Some(Self(whole * 1_000 + fractional * scale))
    }
}

impl fmt::Display for AirspaceFrequencyValue {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{:03}.{:03}", self.0 / 1_000, self.0 % 1_000)
    }
}

/// A frequency unit with its OpenAIP numeric value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum AirspaceFrequencyUnit {
    Megahertz = 2,
}

/// One canonical airspace radio frequency.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AirspaceFrequency {
    /// The validated frequency value.
    pub value: AirspaceFrequencyValue,
    /// The frequency unit.
    pub unit: AirspaceFrequencyUnit,
    /// The source frequency name when it is present.
    pub name: Option<Box<str>>,
    /// Whether this is the primary frequency.
    pub primary: Option<bool>,
    /// Additional source remarks when present.
    pub remarks: Option<Box<str>>,
}
