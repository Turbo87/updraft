use std::fmt;

/// An exact frequency value with three decimal digits.
///
/// The value is stored in thousandths of the unit that accompanies it, so
/// two values compare exactly. CUP and OpenAIP both write `ddd.ddd`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FrequencyValue(u32);

impl FrequencyValue {
    /// Parses a value with at most three digits on each side of the decimal point.
    pub fn from_decimal(value: &str) -> Option<Self> {
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

impl fmt::Display for FrequencyValue {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{:03}.{:03}", self.0 / 1_000, self.0 % 1_000)
    }
}

/// The unit of a frequency value.
///
/// Airport frequencies are always MHz. Non-directional beacons use kHz.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrequencyUnit {
    Kilohertz,
    Megahertz,
}

/// One radio frequency.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Frequency {
    pub value: FrequencyValue,
    pub unit: FrequencyUnit,
}

/// The purpose of an airfield radio frequency.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrequencyType {
    Approach,
    Apron,
    Arrival,
    Center,
    Ctaf,
    Delivery,
    Departure,
    Fis,
    Gliding,
    Ground,
    Information,
    Multicom,
    Unicom,
    Radar,
    Tower,
    Atis,
    Radio,
    Other,
    Airmet,
    Awos,
    Lights,
    Volmet,
    Afis,
    Asos,
    Awis,
    Emergency,
    ClearanceDelivery,
    RemoteComOutlet,
    GroundComOutlet,
    FlightServiceStation,
    ClassC,
    ClassB,
    VfrAdvisory,
    Trsa,
}
