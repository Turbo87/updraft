use super::vario::{SampleAcceptance, Vario};
use std::time::Duration;
use updraft_units::{PressureAltitude, Speed};

/// Flight values derived from the available sensor inputs.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Estimate {
    /// Rate of change of pressure altitude. Positive means climbing.
    pub raw_vertical_speed: Option<Speed>,
    /// Smoothed rate of change of pressure altitude. Positive means climbing.
    pub vertical_speed: Option<Speed>,
}

/// Derives flight values from timestamped physical measurements.
///
/// This layer owns numerical estimator state. [`SensorFusion`](super::SensorFusion)
/// owns selected-source continuity, freshness, and protocol projection.
#[derive(Clone, Debug)]
pub struct Estimator {
    uncompensated: Vario,
}

impl Default for Estimator {
    fn default() -> Self {
        Self::new()
    }
}

impl Estimator {
    pub fn new() -> Self {
        Self {
            uncompensated: Vario::default(),
        }
    }

    /// Takes a barometric altitude against the 1013.25 hPa datum.
    pub fn pressure_altitude(
        &mut self,
        time: Duration,
        altitude: PressureAltitude,
    ) -> SampleAcceptance {
        self.uncompensated.advance(time, altitude.into_inner())
    }

    /// Clears state that assumes continuity with one altitude source.
    pub fn reset_altitude(&mut self) {
        self.uncompensated = Vario::default();
    }

    pub fn estimate(&self) -> Estimate {
        let raw_vertical_speed = self.uncompensated.value();
        let vertical_speed = self.uncompensated.smoothed_value();
        Estimate {
            raw_vertical_speed,
            vertical_speed,
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
}
