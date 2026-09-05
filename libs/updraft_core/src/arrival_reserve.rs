use serde::{Deserialize, Serialize};

/// A finite, nonnegative arrival reserve in metres.
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ArrivalReserve(f64);

// Construction excludes NaN, so equality is reflexive.
impl Eq for ArrivalReserve {}

impl<'de> Deserialize<'de> for ArrivalReserve {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        f64::deserialize(deserializer)?
            .try_into()
            .map_err(serde::de::Error::custom)
    }
}

impl ArrivalReserve {
    pub fn meters(self) -> f64 {
        self.0
    }
}

impl Default for ArrivalReserve {
    fn default() -> Self {
        Self(200.0)
    }
}

impl TryFrom<f64> for ArrivalReserve {
    type Error = InvalidArrivalReserve;

    fn try_from(meters: f64) -> Result<Self, Self::Error> {
        if meters.is_finite() && meters >= 0.0 {
            Ok(Self(meters))
        } else {
            Err(InvalidArrivalReserve)
        }
    }
}

#[derive(Debug, thiserror::Error)]
#[error("Arrival reserve must be finite and nonnegative")]
pub struct InvalidArrivalReserve;

#[cfg(test)]
mod tests {
    use super::*;
    use claims::{assert_err, assert_ok};

    #[test]
    fn reserve_preserves_nonnegative_meters() {
        assert_eq!(ArrivalReserve::default().meters(), 200.0);
        for meters in [0.0, 304.8, f64::MAX] {
            let reserve = assert_ok!(ArrivalReserve::try_from(meters));
            assert_eq!(reserve.meters(), meters);
        }
    }

    #[test]
    fn reserve_rejects_negative_and_nonfinite_meters() {
        for meters in [-1.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert_err!(ArrivalReserve::try_from(meters));
        }
    }
}
