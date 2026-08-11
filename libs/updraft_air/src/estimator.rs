use crate::smoothing_weight;
use std::time::Duration;
use updraft_units::{PressureAltitude, Speed};

/// Time constant of each of the two vertical-speed smoothing stages, in
/// seconds. Fitted against the recorded vario of an LXNAV LX9070.
const VERTICAL_SPEED_TIME_CONSTANT: f64 = 2.;

/// The longest gap that two altitudes can still be differentiated across.
/// A larger gap restarts the vertical speed.
const MAX_ALTITUDE_INTERVAL: Duration = Duration::from_secs(30);

/// What the glider and the air around it are doing.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AirState {
    /// How fast the glider gains height. Positive means climbing.
    ///
    /// This is what a vertical speed indicator shows.
    pub rate_of_climb: Speed,
}

/// Derives the [`AirState`] from flight data.
///
/// Each input arrives on its own call, at whatever rate its source
/// produces it. Read the result from [`state`](Self::state) whenever it
/// is needed. The estimator keeps its state between calls, so the same
/// code serves a live sensor stream and a replayed recording.
///
/// A sample that is not a finite number is ignored where it arrives. The
/// filters hold their state for the whole flight and none of them can
/// heal, because a comparison against a NaN is false and every guard
/// that would reject it therefore passes.
#[derive(Clone, Debug)]
pub struct AirStateEstimator {
    /// Vertical speed of the height alone.
    uncompensated: Vario,
}

/// The height a vertical speed was last differentiated from.
#[derive(Clone, Copy, Debug)]
struct Previous {
    time: Duration,
    height: f64,
}

/// Differentiates a height series and smooths the result through two
/// exponential stages.
#[derive(Clone, Debug, Default)]
struct Vario {
    previous: Option<Previous>,
    first_stage: f64,
    value: Option<f64>,
}

impl Vario {
    /// Takes the next height.
    ///
    /// A gap longer than [`MAX_ALTITUDE_INTERVAL`] restarts the filter,
    /// because nothing can be differentiated across it.
    fn advance(&mut self, time: Duration, height: f64) {
        // A second sample at a time that has not advanced carries
        // nothing to differentiate, and it must not become the reference
        // either: the next real sample would be measured from it and the
        // reading would restart. Two devices that both report a pressure
        // altitude arrive in one batch under one timestamp, so this is
        // the ordinary case and not a fault.
        if self.previous.is_some_and(|previous| time <= previous.time) {
            return;
        }

        let previous = self.previous.replace(Previous { time, height });

        let usable = previous.and_then(|previous| {
            let interval = time.checked_sub(previous.time)?;
            (!interval.is_zero() && interval <= MAX_ALTITUDE_INTERVAL)
                .then_some((interval.as_secs_f64(), previous.height))
        });
        let Some((interval, previous_height)) = usable else {
            self.first_stage = 0.;
            self.value = None;
            return;
        };

        let raw = (height - previous_height) / interval;
        let weight = smoothing_weight(interval, VERTICAL_SPEED_TIME_CONSTANT);
        self.first_stage += weight * (raw - self.first_stage);
        let value = self.value.unwrap_or(0.);
        self.value = Some(value + weight * (self.first_stage - value));
    }
}

impl Default for AirStateEstimator {
    fn default() -> Self {
        Self::new()
    }
}

impl AirStateEstimator {
    pub fn new() -> Self {
        Self {
            uncompensated: Vario::default(),
        }
    }

    /// Takes a barometric altitude against the 1013.25 hPa datum.
    pub fn pressure_altitude(&mut self, time: Duration, altitude: PressureAltitude) {
        let altitude = altitude.into_inner().as_meters();
        if !altitude.is_finite() {
            return;
        }
        self.uncompensated.advance(time, altitude);
    }

    /// The current estimate, or `None` until two altitudes have arrived
    /// close enough together to be differentiated.
    pub fn state(&self) -> Option<AirState> {
        Some(AirState {
            rate_of_climb: Speed::from_meters_per_second(self.uncompensated.value?),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_abs_diff_eq;
    use claims::{assert_none, assert_some};
    use updraft_units::Length;

    fn meters(value: f64) -> PressureAltitude {
        PressureAltitude::new(Length::from_meters(value))
    }

    /// Climbs at `rate` m/s for `seconds` seconds, one sample each.
    fn climb(rate: f64, seconds: u64) -> AirStateEstimator {
        let mut estimator = AirStateEstimator::new();
        for second in 0..seconds {
            estimator.pressure_altitude(
                Duration::from_secs(second),
                meters(1000. + rate * second as f64),
            );
        }
        estimator
    }

    #[test]
    fn the_first_altitude_has_nothing_to_differentiate() {
        let mut estimator = AirStateEstimator::new();
        estimator.pressure_altitude(Duration::ZERO, meters(1000.));

        assert_none!(estimator.state());
    }

    #[test]
    fn a_steady_climb_reads_its_own_rate() {
        let state = assert_some!(climb(2., 60).state());

        assert_abs_diff_eq!(
            state.rate_of_climb,
            Speed::from_meters_per_second(2.),
            epsilon = 0.01
        );
    }

    #[test]
    fn a_long_gap_restarts_the_vertical_speed() {
        let mut estimator = climb(2., 60);
        // Nothing can be differentiated across a gap this long, and the
        // height either side of it is unrelated.
        estimator.pressure_altitude(Duration::from_secs(120), meters(1120.));

        assert_none!(estimator.state());
    }

    #[test]
    fn a_repeated_timestamp_does_not_blank_the_reading() {
        let mut estimator = AirStateEstimator::new();
        for second in 0..60 {
            // Two sources of a pressure altitude reach the estimator in
            // one batch, so both carry the time the bytes arrived.
            let time = Duration::from_secs(second);
            let altitude = meters(1000. + 2. * second as f64);
            estimator.pressure_altitude(time, altitude);
            estimator.pressure_altitude(time, altitude);
        }

        let state = assert_some!(estimator.state());
        assert_abs_diff_eq!(
            state.rate_of_climb,
            Speed::from_meters_per_second(2.),
            epsilon = 0.01
        );
    }

    #[test]
    fn a_source_timestamped_at_half_the_rate_still_reads_the_climb() {
        let mut estimator = AirStateEstimator::new();
        for half in 0..120 {
            // A 2 Hz source whose timestamps resolve to whole seconds:
            // every second arrives twice, and the height moves between
            // the two.
            estimator.pressure_altitude(Duration::from_secs(half / 2), meters(1000. + half as f64));
        }

        let state = assert_some!(estimator.state());
        assert_abs_diff_eq!(
            state.rate_of_climb,
            Speed::from_meters_per_second(2.),
            epsilon = 0.05
        );
    }

    #[test]
    fn an_altitude_that_is_not_a_number_is_ignored() {
        for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            let mut estimator = climb(2., 30);
            estimator.pressure_altitude(Duration::from_secs(30), meters(bad));
            for second in 31..60 {
                estimator.pressure_altitude(
                    Duration::from_secs(second),
                    meters(1000. + 2. * second as f64),
                );
            }

            // A NaN reaches every stage of the filter and none of them
            // can heal, so it has to be rejected where it arrives.
            let state = assert_some!(estimator.state(), "{bad}");
            assert_abs_diff_eq!(
                state.rate_of_climb,
                Speed::from_meters_per_second(2.),
                epsilon = 0.01
            );
        }
    }

    #[test]
    fn a_faster_barometer_gives_the_same_climb_rate() {
        // Ten samples per second instead of one. The smoothing weight
        // follows the interval, so the reading is the same climb.
        let mut estimator = AirStateEstimator::new();
        for tenth in 0..600u64 {
            let time = Duration::from_millis(tenth * 100);
            estimator.pressure_altitude(time, meters(1000. + 0.2 * tenth as f64));
        }

        let state = assert_some!(estimator.state());
        assert_abs_diff_eq!(
            state.rate_of_climb,
            Speed::from_meters_per_second(2.),
            epsilon = 0.01
        );
    }
}
