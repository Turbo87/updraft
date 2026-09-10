#[derive(Debug, Default)]
pub struct ClimbEma {
    previous: Option<(f64, f64)>,
    sum: f64,
    weight: f64,
}

impl ClimbEma {
    pub fn observe(&mut self, seconds: f64, altitude: f64) -> Option<f64> {
        if let Some((at, previous_altitude)) = self.previous {
            if seconds <= at {
                return None;
            }
            let interval = seconds - at;
            let climb = (altitude - previous_altitude) / interval;
            let weight = -(-interval / 10.0).exp_m1();
            self.sum = (1.0 - weight) * self.sum + weight * climb;
            self.weight = (1.0 - weight) * self.weight + weight;
        }
        self.previous = Some((seconds, altitude));
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
        assert_none!(ema.observe(0.0, 100.0));
        assert_some_eq!(ema.observe(1.0, 106.0), 6.0);
        let second = assert_some!(ema.observe(2.0, 108.0));
        let decay = (-0.1_f64).exp();
        assert_abs_diff_eq!(second, (6.0 * decay + 2.0) / (decay + 1.0), epsilon = 1e-12);
        let third = assert_some!(ema.observe(7.0, 103.0));
        let weight = 1.0 - decay;
        let gap_decay = (-0.5_f64).exp();
        let sum = gap_decay * weight * (6.0 * decay + 2.0) - (1.0 - gap_decay);
        let total = gap_decay * weight * (decay + 1.0) + (1.0 - gap_decay);
        assert_abs_diff_eq!(third, sum / total, epsilon = 1e-12);
    }

    #[test]
    fn ignores_old_samples_and_restarts_from_a_new_baseline() {
        let mut ema = ClimbEma::default();
        assert_none!(ema.observe(10.0, 100.0));
        assert_none!(ema.observe(10.0, 900.0));
        assert_none!(ema.observe(5.0, 900.0));
        assert_some_eq!(ema.observe(20.0, 120.0), 2.0);
        assert_some_eq!(ema.observe(25.0, 130.0), 2.0);
        ema = ClimbEma::default();
        assert_none!(ema.observe(100.0, 900.0));
        assert_some_eq!(ema.observe(110.0, 880.0), -2.0);
    }
}
