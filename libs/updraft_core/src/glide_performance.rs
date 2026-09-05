use serde::{Deserialize, Serialize};

/// A finite, nonnegative MacCready value in metres per second.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct MacCready(f64);

impl<'de> Deserialize<'de> for MacCready {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        f64::deserialize(deserializer)?
            .try_into()
            .map_err(serde::de::Error::custom)
    }
}

impl TryFrom<f64> for MacCready {
    type Error = InvalidMacCready;

    fn try_from(meters_per_second: f64) -> Result<Self, Self::Error> {
        if meters_per_second.is_finite() && meters_per_second >= 0.0 {
            Ok(Self(meters_per_second))
        } else {
            Err(InvalidMacCready)
        }
    }
}

#[derive(Debug, thiserror::Error)]
#[error("MacCready must be finite and nonnegative")]
pub struct InvalidMacCready;

/// Session controls that reset when the core starts.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct GlidePerformance {
    pub mac_cready: MacCready,
}

#[cfg(test)]
mod tests {
    use super::*;
    use claims::{assert_err, assert_ok};

    #[test]
    fn maccready_accepts_only_finite_nonnegative_values() {
        assert_eq!(MacCready::default().0, 0.0);
        for value in [0.0, 1.5, f64::MAX] {
            assert_eq!(assert_ok!(MacCready::try_from(value)).0, value);
        }
        for value in [-1.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert_err!(MacCready::try_from(value));
        }
    }
}
