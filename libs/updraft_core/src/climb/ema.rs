use std::time::Duration;
use updraft_units::{Length, Speed};

const TIME_CONSTANT: Duration = Duration::from_secs(10);

#[derive(Debug, Default)]
pub struct ClimbEma {
    previous: Option<(Duration, Length)>,
    sum: Speed,
    weight: f64,
}

impl ClimbEma {
    pub fn observe(&mut self, time: Duration, altitude: Length) -> Option<Speed> {
        if let Some((at, previous_altitude)) = self.previous {
            if time <= at {
                return None;
            }
            let interval = time - at;
            let climb = (altitude - previous_altitude) / interval;
            let weight = -(-interval.div_duration_f64(TIME_CONSTANT)).exp_m1();
            self.sum = (1.0 - weight) * self.sum + weight * climb;
            self.weight = (1.0 - weight) * self.weight + weight;
        }
        self.previous = Some((time, altitude));
        (self.weight > 0.0).then(|| self.sum / self.weight)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_abs_diff_eq;
    use claims::{assert_none, assert_some, assert_some_eq};

    #[test]
    fn normalizes_startup_weights_and_uses_interval_duration() {
        let mut ema = ClimbEma::default();
        assert_none!(ema.observe(Duration::from_secs(0), Length::from_meters(100.0)));
        assert_some_eq!(
            ema.observe(Duration::from_secs(1), Length::from_meters(106.0)),
            Speed::from_meters_per_second(6.0)
        );
        let second = assert_some!(ema.observe(Duration::from_secs(2), Length::from_meters(108.0)));
        let decay = (-0.1_f64).exp();
        assert_abs_diff_eq!(
            second.as_meters_per_second(),
            (6.0 * decay + 2.0) / (decay + 1.0),
            epsilon = 1e-12
        );
        let third = assert_some!(ema.observe(Duration::from_secs(7), Length::from_meters(103.0)));
        let weight = 1.0 - decay;
        let gap_decay = (-0.5_f64).exp();
        let sum = gap_decay * weight * (6.0 * decay + 2.0) - (1.0 - gap_decay);
        let total = gap_decay * weight * (decay + 1.0) + (1.0 - gap_decay);
        assert_abs_diff_eq!(third.as_meters_per_second(), sum / total, epsilon = 1e-12);
    }

    #[test]
    fn ignores_old_samples_and_restarts_from_a_new_baseline() {
        let mut ema = ClimbEma::default();
        assert_none!(ema.observe(Duration::from_secs(10), Length::from_meters(100.0)));
        assert_none!(ema.observe(Duration::from_secs(10), Length::from_meters(900.0)));
        assert_none!(ema.observe(Duration::from_secs(5), Length::from_meters(900.0)));
        assert_some_eq!(
            ema.observe(Duration::from_secs(20), Length::from_meters(120.0)),
            Speed::from_meters_per_second(2.0)
        );
        assert_some_eq!(
            ema.observe(Duration::from_secs(25), Length::from_meters(130.0)),
            Speed::from_meters_per_second(2.0)
        );
        ema = ClimbEma::default();
        assert_none!(ema.observe(Duration::from_secs(100), Length::from_meters(900.0)));
        assert_some_eq!(
            ema.observe(Duration::from_secs(110), Length::from_meters(880.0)),
            Speed::from_meters_per_second(-2.0)
        );
    }
}
