use super::sample::{AltitudeDomain, SampleAcceptance};
use std::time::Duration;
use updraft_units::{Length, Speed};

/// Time constant of each vertical-speed smoothing stage.
/// The value is fitted against the recorded vario of an LXNAV LX9070.
const VERTICAL_SPEED_TIME_CONSTANT: Duration = Duration::from_secs(2);

/// The longest gap that two altitudes can still be differentiated across.
/// A larger gap restarts the vertical speed.
const MAX_ALTITUDE_INTERVAL: Duration = Duration::from_secs(30);

#[derive(Clone, Copy, Debug)]
struct Previous {
    time: Duration,
    altitude: Length,
    domain: AltitudeDomain,
}

/// Differentiates an altitude series and smooths the result through two stages.
#[derive(Clone, Debug, Default)]
pub struct Vario {
    previous: Option<Previous>,
    first_stage: Speed,
    value: Option<Speed>,
    smoothed_value: Option<Speed>,
}

impl Vario {
    /// Adds an altitude sample and updates the vertical speed.
    ///
    /// The method ignores samples whose timestamps do not advance. A gap that
    /// exceeds [`MAX_ALTITUDE_INTERVAL`] starts a new series. Another sample
    /// must arrive before a vertical speed is available.
    pub fn advance(
        &mut self,
        time: Duration,
        altitude: Length,
        domain: AltitudeDomain,
    ) -> SampleAcceptance {
        // A sample at a time that has not advanced carries nothing to
        // differentiate. It must not replace the reference for the next sample.
        if self
            .previous
            .is_some_and(|previous| previous.domain == domain && time <= previous.time)
        {
            return SampleAcceptance::Ignored;
        }
        let previous = self.previous.replace(Previous {
            time,
            altitude,
            domain,
        });
        let Some(previous) = previous.filter(|previous| previous.domain == domain) else {
            self.first_stage = Speed::default();
            self.value = None;
            self.smoothed_value = None;
            return SampleAcceptance::Accepted;
        };
        let interval = time - previous.time;
        if interval > MAX_ALTITUDE_INTERVAL {
            self.first_stage = Speed::default();
            self.value = None;
            self.smoothed_value = None;
            return SampleAcceptance::Accepted;
        }

        let meters_per_second = (altitude - previous.altitude).as_meters() / interval.as_secs_f64();
        let value = Speed::from_meters_per_second(meters_per_second);
        self.value = Some(value);

        // TODO: Each smoothing stage uses the sample interval, but the second
        // stage consumes the newly updated first stage. The filter response
        // therefore still depends on the update rate. The two-second time
        // constant is tuned for the 1 Hz recordings that we currently use.
        // Revisit this filter before using higher-rate pressure sources, such
        // as a phone barometer, because they will produce a different amount
        // of smoothing and delay.
        let weight = smoothing_weight(interval, VERTICAL_SPEED_TIME_CONSTANT);
        self.first_stage += weight * (value - self.first_stage);
        let smoothed_value = self.smoothed_value.unwrap_or_default();
        self.smoothed_value = Some(smoothed_value + weight * (self.first_stage - smoothed_value));
        SampleAcceptance::Accepted
    }

    pub fn value(&self) -> Option<Speed> {
        self.value
    }

    pub fn smoothed_value(&self) -> Option<Speed> {
        self.smoothed_value
    }
}

fn smoothing_weight(interval: Duration, time_constant: Duration) -> f64 {
    1. - (-interval.as_secs_f64() / time_constant.as_secs_f64()).exp()
}
