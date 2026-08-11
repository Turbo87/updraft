use crate::smoothing_weight;
use std::time::Duration;
use updraft_units::{PressureAltitude, Speed};

/// Standard gravity, in m/s².
const GRAVITY: f64 = 9.80665;

/// Time constant of each of the two vertical-speed smoothing stages, in
/// seconds. Fitted against the recorded vario of an LXNAV LX9070.
const VERTICAL_SPEED_TIME_CONSTANT: f64 = 2.;

/// The longest gap that two altitudes can still be differentiated across.
/// A larger gap restarts the vertical speed.
const MAX_ALTITUDE_INTERVAL: Duration = Duration::from_secs(30);

/// How long an airspeed stays usable after it arrives. It covers a
/// source that reports slowly, and expires one that went away.
const MAX_AGE: Duration = Duration::from_secs(5);

/// What the glider and the air around it are doing.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AirState {
    /// How fast the glider gains energy. Positive means climbing.
    ///
    /// This is the total-energy vertical speed, with the height that the
    /// glider trades for airspeed taken out. Without an airspeed it is
    /// the rate of climb itself.
    pub vertical_speed: Speed,
    /// How fast the glider gains height, with no total-energy
    /// compensation. Positive means climbing.
    ///
    /// This is what a vertical speed indicator shows. It counts the
    /// height gained by pulling up as a climb, which is what a pilot
    /// wants under power, and it stays available when the airspeed goes
    /// away.
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
    /// Vertical speed of the total energy height.
    total_energy: Vario,
    /// Vertical speed of the height alone, without the energy term.
    uncompensated: Vario,
    /// Latest airspeed from an instrument.
    measured_air_speed: Option<Timed<f64>>,
    /// The airspeed term of the previous total energy height, absent
    /// where no airspeed contributed one.
    previous_energy: Option<f64>,
}

/// A value with the time it was measured at.
#[derive(Clone, Copy, Debug)]
struct Timed<T> {
    time: Duration,
    value: T,
}

impl<T> Timed<T> {
    fn fresh_at(&self, now: Duration) -> bool {
        now.checked_sub(self.time).is_some_and(|age| age <= MAX_AGE)
    }
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
    /// `rebase` is how far the height moved because of a change of
    /// reference rather than a climb: the airspeed term appearing or
    /// going away. It is added to the stored previous height, so the
    /// difference stays a climb and the filter keeps running.
    ///
    /// A gap longer than [`MAX_ALTITUDE_INTERVAL`] restarts the filter,
    /// because nothing can be differentiated across it.
    fn advance(&mut self, time: Duration, height: f64, rebase: f64) {
        if let Some(previous) = self.previous.as_mut() {
            previous.height += rebase;
        }

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
            total_energy: Vario::default(),
            uncompensated: Vario::default(),
            measured_air_speed: None,
            previous_energy: None,
        }
    }

    /// Takes a barometric altitude against the 1013.25 hPa datum.
    pub fn pressure_altitude(&mut self, time: Duration, altitude: PressureAltitude) {
        let altitude = altitude.into_inner().as_meters();
        if !altitude.is_finite() {
            return;
        }
        self.advance_vertical_speed(time, altitude);
    }

    /// Takes a true airspeed from an instrument. It expires after five
    /// seconds, so the compensation stops when the instrument goes away.
    pub fn air_speed(&mut self, time: Duration, speed: Speed) {
        let speed = speed.as_meters_per_second();
        if !speed.is_finite() {
            return;
        }
        self.measured_air_speed = Some(Timed { time, value: speed });
    }

    /// The airspeed to work from, while an instrument still reports one.
    fn air_speed_at(&self, now: Duration) -> Option<f64> {
        self.measured_air_speed
            .filter(|speed| speed.fresh_at(now))
            .map(|speed| speed.value)
    }

    /// Differentiates both the total energy height and the height alone.
    ///
    /// The airspeed term enters and leaves whole. Rebasing the previous
    /// height by it keeps the reading on the climb across the change,
    /// where restarting the filter would lose it: 57 m of energy at
    /// 120 km/h would otherwise read as 57 m of sink.
    fn advance_vertical_speed(&mut self, time: Duration, height: f64) {
        let air_speed = self.air_speed_at(time);
        let energy = air_speed.map_or(0., |speed| speed * speed / (2. * GRAVITY));

        // Its size at the previous sample is known where the term was
        // already there, and the current one stands in for it where it
        // was not.
        let compensation = match (self.previous_energy, air_speed.is_some()) {
            (None, true) => energy,
            (Some(previous), false) => -previous,
            _ => 0.,
        };
        self.previous_energy = air_speed.map(|_| energy);

        self.total_energy
            .advance(time, height + energy, compensation);
        self.uncompensated.advance(time, height, 0.);
    }

    /// The current estimate, or `None` until two altitudes have arrived
    /// close enough together to be differentiated.
    pub fn state(&self) -> Option<AirState> {
        // The uncompensated chain needs heights alone, so it is the one
        // that is always available.
        let rate_of_climb = self.uncompensated.value?;
        let vertical_speed = self.total_energy.value.unwrap_or(rate_of_climb);
        Some(AirState {
            vertical_speed: Speed::from_meters_per_second(vertical_speed),
            rate_of_climb: Speed::from_meters_per_second(rate_of_climb),
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

    /// Flies `seconds` seconds at a steady height, decelerating from
    /// 120 km/h by 1 km/h each second and trading the airspeed for
    /// height. `compensated` reports the airspeed to the estimator.
    fn pull_up(compensated: bool) -> AirStateEstimator {
        let mut estimator = AirStateEstimator::new();
        for second in 0..60 {
            let time = Duration::from_secs(second);
            let speed = (120. - second as f64) / 3.6;
            // Every metre per second of airspeed given up buys `v/g` of
            // height, so the total energy height does not move.
            let altitude = 1000. + (33.333 * 33.333 - speed * speed) / (2. * GRAVITY);
            if compensated {
                estimator.air_speed(time, Speed::from_meters_per_second(speed));
            }
            estimator.pressure_altitude(time, meters(altitude));
        }
        estimator
    }

    #[test]
    fn a_pull_up_trades_airspeed_for_height_without_a_climb() {
        let state = assert_some!(pull_up(true).state());

        assert_abs_diff_eq!(state.vertical_speed, Speed::ZERO, epsilon = 0.01);
        // The glider does gain height, and the uncompensated reading
        // reports it. Only the total-energy reading takes it out.
        assert!(
            state.rate_of_climb.as_meters_per_second() > 0.5,
            "rate of climb read {:?}",
            state.rate_of_climb
        );
    }

    #[test]
    fn without_an_airspeed_a_pull_up_reads_as_a_climb() {
        let state = assert_some!(pull_up(false).state());

        // Nothing separates height bought with airspeed from height
        // bought with lift, so both readings agree.
        assert_eq!(state.vertical_speed, state.rate_of_climb);
        assert!(state.vertical_speed.as_meters_per_second() > 0.5);
    }

    #[test]
    fn losing_the_airspeed_sensor_does_not_show_as_a_climb() {
        let mut estimator = AirStateEstimator::new();
        for second in 0..60 {
            let time = Duration::from_secs(second);
            estimator.air_speed(time, Speed::from_kilometers_per_hour(120.));
            estimator.pressure_altitude(time, meters(1000.));
        }

        // The energy term is 57 m at 120 km/h, and dropping it must not
        // read as 57 m of sink. The airspeed expires five seconds after
        // the last one arrived.
        for second in 60..70 {
            estimator.pressure_altitude(Duration::from_secs(second), meters(1000.));
        }

        let state = assert_some!(estimator.state());
        assert_abs_diff_eq!(state.vertical_speed, Speed::ZERO, epsilon = 0.01);
    }

    #[test]
    fn an_airspeed_that_is_not_a_number_is_ignored() {
        for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            let mut estimator = AirStateEstimator::new();
            for second in 0..60 {
                let time = Duration::from_secs(second);
                if second == 30 {
                    estimator.air_speed(time, Speed::from_meters_per_second(bad));
                }
                estimator.air_speed(time, Speed::from_kilometers_per_hour(120.));
                estimator.pressure_altitude(time, meters(1000. + 2. * second as f64));
            }

            let state = assert_some!(estimator.state(), "{bad}");
            assert_abs_diff_eq!(
                state.vertical_speed,
                Speed::from_meters_per_second(2.),
                epsilon = 0.01
            );
        }
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

    #[test]
    fn gaining_the_airspeed_sensor_does_not_show_as_a_climb() {
        let mut estimator = AirStateEstimator::new();
        for second in 0..60u64 {
            estimator.pressure_altitude(Duration::from_secs(second), meters(1000.));
        }

        // The energy term is 57 m at 120 km/h. Adding it must not read
        // as 57 m of climb.
        for second in 60..70u64 {
            let time = Duration::from_secs(second);
            estimator.air_speed(time, Speed::from_kilometers_per_hour(120.));
            estimator.pressure_altitude(time, meters(1000.));
        }

        let state = assert_some!(estimator.state());
        assert_abs_diff_eq!(state.vertical_speed, Speed::ZERO, epsilon = 0.01);
    }

    #[test]
    fn the_rate_of_climb_survives_losing_the_airspeed_sensor() {
        let mut estimator = AirStateEstimator::new();
        for second in 0..60u64 {
            let time = Duration::from_secs(second);
            estimator.air_speed(time, Speed::from_kilometers_per_hour(120.));
            estimator.pressure_altitude(time, meters(1000. + 2. * second as f64));
        }

        // Dropping the energy term takes 57 m out of the total energy
        // height. Rebasing the previous height by the same amount keeps
        // both readings on the 2 m/s climb across the change.
        let mut lowest_rate = f64::INFINITY;
        let mut lowest_vertical = f64::INFINITY;
        for second in 60..70u64 {
            let time = Duration::from_secs(second);
            estimator.pressure_altitude(time, meters(1000. + 2. * second as f64));
            let state = assert_some!(estimator.state());
            lowest_rate = lowest_rate.min(state.rate_of_climb.as_meters_per_second());
            lowest_vertical = lowest_vertical.min(state.vertical_speed.as_meters_per_second());
        }

        assert!(lowest_rate >= 1.99, "rate of climb dipped to {lowest_rate}");
        assert!(
            lowest_vertical >= 1.99,
            "vertical speed dipped to {lowest_vertical}"
        );
    }
}
