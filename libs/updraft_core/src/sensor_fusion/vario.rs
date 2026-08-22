use std::time::Duration;
use updraft_units::{Length, Speed};

/// The longest gap that two altitudes can still be differentiated across.
/// A larger gap restarts the vertical speed.
const MAX_ALTITUDE_INTERVAL: Duration = Duration::from_secs(30);

#[derive(Clone, Copy, Debug)]
struct Previous {
    time: Duration,
    altitude: Length,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SampleAcceptance {
    Accepted,
    Ignored,
}

/// Differentiates an altitude series.
#[derive(Clone, Debug, Default)]
pub struct Vario {
    previous: Option<Previous>,
    value: Option<Speed>,
}

impl Vario {
    /// Adds an altitude sample and updates the vertical speed.
    ///
    /// The method ignores samples whose timestamps do not advance. A gap that
    /// exceeds [`MAX_ALTITUDE_INTERVAL`] starts a new series. Another sample
    /// must arrive before a vertical speed is available.
    pub fn advance(&mut self, time: Duration, altitude: Length) -> SampleAcceptance {
        // A sample at a time that has not advanced carries nothing to
        // differentiate. It must not replace the reference for the next sample.
        if self.previous.is_some_and(|previous| time <= previous.time) {
            return SampleAcceptance::Ignored;
        }

        let previous = self.previous.replace(Previous { time, altitude });
        let Some(previous) = previous else {
            self.value = None;
            return SampleAcceptance::Accepted;
        };
        let interval = time - previous.time;
        if interval > MAX_ALTITUDE_INTERVAL {
            self.value = None;
            return SampleAcceptance::Accepted;
        }

        let meters_per_second = (altitude - previous.altitude).as_meters() / interval.as_secs_f64();
        self.value = Some(Speed::from_meters_per_second(meters_per_second));
        SampleAcceptance::Accepted
    }

    pub fn value(&self) -> Option<Speed> {
        self.value
    }
}
