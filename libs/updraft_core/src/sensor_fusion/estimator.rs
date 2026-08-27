use super::vario::{SampleAcceptance, Vario};
use std::time::Duration;
use updraft_units::{Length, PressureAltitude, Speed};

const GRAVITY: f64 = 9.80665;

#[derive(Clone, Copy, Debug)]
struct AirSpeedSample {
    time: Duration,
    speed: Speed,
}

/// Flight values derived from the available sensor inputs.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Estimate {
    /// Rate of change of pressure altitude. Positive means climbing.
    pub raw_vertical_speed: Option<Speed>,
    /// Smoothed rate of change of pressure altitude. Positive means climbing.
    pub vertical_speed: Option<Speed>,
    pub vario: Option<Speed>,
}

/// Derives flight values from timestamped physical measurements.
///
/// This layer owns numerical estimator state. [`SensorFusion`](super::SensorFusion)
/// owns selected-source continuity, freshness, and protocol projection.
#[derive(Clone, Debug)]
pub struct Estimator {
    vario: Vario,
    uncompensated: Vario,
    measured_air_speed: Option<AirSpeedSample>,
}

impl Default for Estimator {
    fn default() -> Self {
        Self::new()
    }
}

impl Estimator {
    pub fn new() -> Self {
        Self {
            vario: Vario::default(),
            uncompensated: Vario::default(),
            measured_air_speed: None,
        }
    }

    /// Takes a barometric altitude against the 1013.25 hPa datum.
    pub fn pressure_altitude(
        &mut self,
        time: Duration,
        altitude: PressureAltitude,
    ) -> SampleAcceptance {
        self.advance_vertical_speed(time, altitude.into_inner())
    }

    /// Adds an airspeed sample when its timestamp advances.
    pub fn air_speed(&mut self, time: Duration, speed: Speed) -> SampleAcceptance {
        if self
            .measured_air_speed
            .is_some_and(|previous| time <= previous.time)
        {
            return SampleAcceptance::Ignored;
        }
        self.measured_air_speed = Some(AirSpeedSample { time, speed });
        SampleAcceptance::Accepted
    }

    pub fn clear_air_speed(&mut self) {
        self.reset_air_speed();
    }

    /// Clears state that assumes continuity with one airspeed source.
    pub fn reset_air_speed(&mut self) {
        self.measured_air_speed = None;
        self.vario = Vario::default();
    }

    fn advance_vertical_speed(&mut self, time: Duration, altitude: Length) -> SampleAcceptance {
        let acceptance = self.uncompensated.advance(time, altitude);
        if acceptance == SampleAcceptance::Ignored {
            return acceptance;
        }

        let Some(AirSpeedSample { speed, .. }) = self.measured_air_speed else {
            return acceptance;
        };
        let speed = speed.as_meters_per_second();
        let energy = Length::from_meters(speed * speed / (2. * GRAVITY));

        let compensated = self.vario.advance(time, altitude + energy);
        debug_assert_eq!(compensated, SampleAcceptance::Accepted);
        acceptance
    }

    /// Clears state that assumes continuity with one altitude source.
    pub fn reset_altitude(&mut self) {
        self.vario = Vario::default();
        self.uncompensated = Vario::default();
    }

    pub fn estimate(&self) -> Estimate {
        let raw_vertical_speed = self.uncompensated.value();
        let vertical_speed = self.uncompensated.smoothed_value();
        let vario = self.measured_air_speed.and(self.vario.smoothed_value());
        Estimate {
            raw_vertical_speed,
            vertical_speed,
            vario,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::SampleAcceptance::{Accepted, Ignored};
    use super::*;
    use approx::assert_abs_diff_eq;
    use claims::{assert_none, assert_some};
    use updraft_units::Length;

    fn meters(value: f64) -> PressureAltitude {
        PressureAltitude::new(Length::from_meters(value))
    }

    fn add_air_speed(estimator: &mut Estimator, time: Duration, speed: Speed) {
        assert_eq!(estimator.air_speed(time, speed), Accepted);
    }

    fn climb(rate: f64, seconds: u64) -> Estimator {
        let mut estimator = Estimator::new();
        for second in 0..seconds {
            assert_eq!(
                estimator.pressure_altitude(
                    Duration::from_secs(second),
                    meters(1000. + rate * second as f64),
                ),
                Accepted
            );
        }
        estimator
    }

    #[test]
    fn first_altitude_produces_no_estimate() {
        let mut estimator = Estimator::new();
        assert_eq!(
            estimator.pressure_altitude(Duration::ZERO, meters(1000.)),
            Accepted
        );

        assert_none!(estimator.estimate().raw_vertical_speed);
    }

    #[test]
    fn steady_climb_produces_its_rate() {
        let estimate = climb(2., 60).estimate();
        let raw_vertical_speed = assert_some!(estimate.raw_vertical_speed);
        let vertical_speed = assert_some!(estimate.vertical_speed);

        assert_abs_diff_eq!(
            raw_vertical_speed,
            Speed::from_meters_per_second(2.),
            epsilon = 0.01
        );
        assert_abs_diff_eq!(
            vertical_speed,
            Speed::from_meters_per_second(2.),
            epsilon = 0.01
        );
    }

    #[test]
    fn altitude_gap_resets_only_above_the_maximum_interval() {
        let estimator = climb(2., 60);
        let mut at_boundary = estimator.clone();
        let mut above_boundary = estimator;

        assert_eq!(
            at_boundary.pressure_altitude(Duration::from_secs(89), meters(1178.)),
            Accepted
        );
        assert_eq!(
            above_boundary.pressure_altitude(
                Duration::from_secs(89) + Duration::from_nanos(1),
                meters(1178.),
            ),
            Accepted
        );

        assert_some!(at_boundary.estimate().raw_vertical_speed);
        assert_none!(above_boundary.estimate().raw_vertical_speed);
    }

    #[test]
    fn repeated_timestamp_keeps_the_first_altitude() {
        let mut estimator = Estimator::new();
        for second in 0..60 {
            let time = Duration::from_secs(second);
            let altitude = meters(1000. + 2. * second as f64);
            assert_eq!(estimator.pressure_altitude(time, altitude), Accepted);
            assert_eq!(estimator.pressure_altitude(time, meters(10_000.)), Ignored);
        }

        let raw_vertical_speed = assert_some!(estimator.estimate().raw_vertical_speed);
        assert_abs_diff_eq!(
            raw_vertical_speed,
            Speed::from_meters_per_second(2.),
            epsilon = 0.01
        );
    }

    #[test]
    fn one_hertz_climb_has_the_fitted_vertical_speed_response() {
        let mut estimator = Estimator::new();
        assert_eq!(
            estimator.pressure_altitude(Duration::ZERO, meters(1000.)),
            Accepted
        );

        let expected = [
            0.309_636_243_492_350_97,
            0.685_243_993_565_065_4,
            1.026_970_418_232_237_4,
            1.303_327_156_625_063_5,
        ];
        for (second, expected) in (1..).zip(expected) {
            assert_eq!(
                estimator.pressure_altitude(
                    Duration::from_secs(second),
                    meters(1000. + 2. * second as f64),
                ),
                Accepted
            );
            assert_abs_diff_eq!(
                assert_some!(estimator.estimate().vertical_speed),
                Speed::from_meters_per_second(expected),
                epsilon = 1e-9
            );
        }
    }

    #[test]
    fn coarse_timestamps_preserve_the_vertical_speed() {
        let mut estimator = Estimator::new();
        for half in 0..120 {
            // A 2 Hz source whose timestamps resolve to whole seconds:
            // every second arrives twice, and the altitude moves between
            // the two.
            let acceptance = estimator
                .pressure_altitude(Duration::from_secs(half / 2), meters(1000. + half as f64));
            assert_eq!(acceptance, if half % 2 == 0 { Accepted } else { Ignored });
        }

        let raw_vertical_speed = assert_some!(estimator.estimate().raw_vertical_speed);
        assert_abs_diff_eq!(
            raw_vertical_speed,
            Speed::from_meters_per_second(2.),
            epsilon = 0.05
        );
    }

    #[test]
    fn older_timestamp_preserves_the_estimate_and_reference() {
        let mut estimator = climb(2., 30);
        let mut control = estimator.clone();
        let before = estimator.estimate();

        assert_eq!(
            estimator.pressure_altitude(Duration::from_secs(10), meters(10_000.)),
            Ignored
        );
        assert_eq!(estimator.estimate(), before);

        let time = Duration::from_secs(30);
        let altitude = meters(1060.);
        assert_eq!(estimator.pressure_altitude(time, altitude), Accepted);
        assert_eq!(control.pressure_altitude(time, altitude), Accepted);

        assert_eq!(estimator.estimate(), control.estimate());
    }

    #[test]
    fn sample_rates_converge_to_the_same_steady_climb() {
        let slow = climb(2., 11).estimate();
        let mut fast = Estimator::new();
        for tenth in 0..=100u64 {
            let time = Duration::from_millis(tenth * 100);
            assert_eq!(
                fast.pressure_altitude(time, meters(1000. + 0.2 * tenth as f64)),
                Accepted
            );
        }
        let fast = fast.estimate();

        let expected = Speed::from_meters_per_second(2.);
        assert_abs_diff_eq!(
            assert_some!(slow.raw_vertical_speed),
            expected,
            epsilon = 1e-9
        );
        assert_abs_diff_eq!(
            assert_some!(fast.raw_vertical_speed),
            expected,
            epsilon = 1e-9
        );
        assert_abs_diff_eq!(
            assert_some!(fast.vertical_speed),
            assert_some!(slow.vertical_speed),
            epsilon = 0.05
        );
    }

    fn pull_up(compensated: bool) -> Estimator {
        let mut estimator = Estimator::new();
        for second in 0..60 {
            let time = Duration::from_secs(second);
            let speed = (120. - second as f64) / 3.6;
            let altitude = meters(1000. + (33.333 * 33.333 - speed * speed) / (2. * GRAVITY));
            if compensated {
                let air_speed = Speed::from_meters_per_second(speed);
                add_air_speed(&mut estimator, time, air_speed);
            }
            let acceptance = estimator.pressure_altitude(time, altitude);
            assert_eq!(acceptance, Accepted);
        }
        estimator
    }

    #[test]
    fn pull_up_trades_airspeed_for_altitude_without_a_climb() {
        let estimate = pull_up(true).estimate();
        let vario = assert_some!(estimate.vario);
        let raw_vertical_speed = assert_some!(estimate.raw_vertical_speed);
        let vertical_speed = assert_some!(estimate.vertical_speed);
        let expected_raw = Speed::from_meters_per_second(0.484);
        let expected_smoothed = Speed::from_meters_per_second(0.508);

        assert_abs_diff_eq!(vario, Speed::ZERO, epsilon = 0.01);
        assert_abs_diff_eq!(raw_vertical_speed, expected_raw, epsilon = 0.01);
        assert_abs_diff_eq!(vertical_speed, expected_smoothed, epsilon = 0.01);
    }

    #[test]
    fn pull_up_without_airspeed_has_no_vario_estimate() {
        let estimate = pull_up(false).estimate();
        let raw_vertical_speed = assert_some!(estimate.raw_vertical_speed);
        let vertical_speed = assert_some!(estimate.vertical_speed);
        let expected_raw = Speed::from_meters_per_second(0.484);
        let expected_smoothed = Speed::from_meters_per_second(0.508);

        assert_none!(estimate.vario);
        assert_abs_diff_eq!(raw_vertical_speed, expected_raw, epsilon = 0.01);
        assert_abs_diff_eq!(vertical_speed, expected_smoothed, epsilon = 0.01);
    }

    #[test]
    fn cleared_airspeed_removes_the_vario_estimate() {
        let mut estimator = Estimator::new();
        let air_speed = Speed::from_kilometers_per_hour(120.);
        let altitude = meters(1000.);
        for second in 0..60 {
            let time = Duration::from_secs(second);
            add_air_speed(&mut estimator, time, air_speed);
            let acceptance = estimator.pressure_altitude(time, altitude);
            assert_eq!(acceptance, Accepted);
        }

        estimator.clear_air_speed();
        assert_none!(estimator.estimate().vario);
        for second in 60..70 {
            let time = Duration::from_secs(second);
            let acceptance = estimator.pressure_altitude(time, altitude);
            assert_eq!(acceptance, Accepted);
        }

        let estimate = estimator.estimate();
        let raw_vertical_speed = assert_some!(estimate.raw_vertical_speed);
        assert_none!(estimate.vario);
        assert_abs_diff_eq!(raw_vertical_speed, Speed::ZERO, epsilon = 0.01);
    }

    #[test]
    fn returning_airspeed_restarts_the_vario_series() {
        let mut estimator = Estimator::new();
        let altitude = meters(1_000.);
        let air_speed = Speed::from_meters_per_second(50.);
        add_air_speed(&mut estimator, Duration::ZERO, air_speed);
        let acceptance = estimator.pressure_altitude(Duration::ZERO, altitude);
        assert_eq!(acceptance, Accepted);
        let time = Duration::from_secs(1);
        let acceptance = estimator.pressure_altitude(time, altitude);
        assert_eq!(acceptance, Accepted);

        estimator.clear_air_speed();
        let altitude = meters(1_100.);
        let time = Duration::from_secs(2);
        let acceptance = estimator.pressure_altitude(time, altitude);
        assert_eq!(acceptance, Accepted);
        let time = Duration::from_secs(3);
        let air_speed = Speed::from_meters_per_second(100.);
        add_air_speed(&mut estimator, time, air_speed);
        let acceptance = estimator.pressure_altitude(time, altitude);
        assert_eq!(acceptance, Accepted);
        assert_none!(estimator.estimate().vario);

        let time = Duration::from_secs(4);
        let acceptance = estimator.pressure_altitude(time, altitude);
        assert_eq!(acceptance, Accepted);

        let vario = assert_some!(estimator.estimate().vario);
        assert_abs_diff_eq!(vario, Speed::ZERO, epsilon = 0.01);
    }

    #[test]
    fn new_airspeed_does_not_change_vertical_speed() {
        let mut estimator = Estimator::new();
        let altitude = meters(1000.);
        for second in 0..60u64 {
            let time = Duration::from_secs(second);
            let acceptance = estimator.pressure_altitude(time, altitude);
            assert_eq!(acceptance, Accepted);
        }
        let before = assert_some!(estimator.estimate().vertical_speed);
        let air_speed = Speed::from_kilometers_per_hour(120.);
        for second in 60..70u64 {
            let time = Duration::from_secs(second);
            add_air_speed(&mut estimator, time, air_speed);
            let acceptance = estimator.pressure_altitude(time, altitude);
            assert_eq!(acceptance, Accepted);
        }

        let estimate = estimator.estimate();
        let vertical_speed = assert_some!(estimate.vertical_speed);
        let vario = assert_some!(estimate.vario);
        assert_eq!(vertical_speed, before);
        assert_abs_diff_eq!(vario, Speed::ZERO, epsilon = 0.01);
    }

    #[test]
    fn raw_vertical_speed_survives_cleared_airspeed() {
        let mut estimator = Estimator::new();
        let air_speed = Speed::from_kilometers_per_hour(120.);
        for second in 0..60u64 {
            let time = Duration::from_secs(second);
            let altitude = meters(1000. + 2. * second as f64);
            add_air_speed(&mut estimator, time, air_speed);
            let acceptance = estimator.pressure_altitude(time, altitude);
            assert_eq!(acceptance, Accepted);
        }

        estimator.clear_air_speed();
        let expected = Speed::from_meters_per_second(2.);
        for second in 60..70u64 {
            let time = Duration::from_secs(second);
            let altitude = meters(1000. + 2. * second as f64);
            let acceptance = estimator.pressure_altitude(time, altitude);
            assert_eq!(acceptance, Accepted);
            assert_none!(estimator.estimate().vario);
            let raw_vertical_speed = assert_some!(estimator.estimate().raw_vertical_speed);
            assert_abs_diff_eq!(raw_vertical_speed, expected, epsilon = 0.01);
        }
    }

    #[test]
    fn older_airspeed_preserves_the_latest_sample() {
        let mut estimator = Estimator::new();
        let air_speed = Speed::from_meters_per_second(50.);
        add_air_speed(&mut estimator, Duration::ZERO, air_speed);
        let acceptance = estimator.pressure_altitude(Duration::ZERO, meters(1_000.));
        assert_eq!(acceptance, Accepted);
        let time = Duration::from_secs(2);
        let air_speed = Speed::from_meters_per_second(40.);
        add_air_speed(&mut estimator, time, air_speed);
        let delayed_speed = Speed::from_meters_per_second(100.);
        let acceptance = estimator.air_speed(Duration::from_secs(1), delayed_speed);
        assert_eq!(acceptance, Ignored);

        let altitude_gain = (50_f64.powi(2) - 40_f64.powi(2)) / (2. * GRAVITY);
        let altitude = meters(1_000. + altitude_gain);
        let acceptance = estimator.pressure_altitude(time, altitude);
        assert_eq!(acceptance, Accepted);
        let vario = assert_some!(estimator.estimate().vario);
        assert_abs_diff_eq!(vario, Speed::ZERO, epsilon = 0.01);
    }
}
