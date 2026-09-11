use super::ClimbWindow;
use std::time::Duration;
use updraft_units::{Length, Speed};

#[derive(Debug, Default)]
pub struct SmoothedClimbWindow {
    previous: Option<(Duration, Length)>,
    window: ClimbWindow,
}

impl SmoothedClimbWindow {
    pub fn observe(&mut self, time: Duration, altitude: Length) -> Option<Speed> {
        let filtered = if let Some((at, previous)) = self.previous {
            if time <= at {
                return None;
            }
            let weight = -(-(time - at).div_duration_f64(Duration::from_millis(7_500))).exp_m1();
            previous + (altitude - previous) * weight
        } else {
            altitude
        };
        self.previous = Some((time, filtered));
        self.window.observe(time, filtered);
        self.window.average(Duration::from_secs(20))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_abs_diff_eq;
    use claims::{assert_none, assert_some};

    #[test]
    fn smooths_height_before_averaging_with_partial_history_and_boundary_interpolation() {
        let mut climb = SmoothedClimbWindow::default();
        assert_none!(climb.observe(Duration::ZERO, Length::from_meters(100.0)));
        let first = assert_some!(climb.observe(Duration::from_secs(5), Length::from_meters(130.0)));
        let height5 = 100.0 + 30.0 * (1.0 - (-5.0_f64 / 7.5).exp());
        assert_abs_diff_eq!(
            first.as_meters_per_second(),
            (height5 - 100.0) / 5.0,
            epsilon = 1e-12
        );
        assert_none!(climb.observe(Duration::from_secs(5), Length::from_meters(900.0)));
        assert_none!(climb.observe(Duration::from_secs(4), Length::from_meters(900.0)));
        let second =
            assert_some!(climb.observe(Duration::from_secs(23), Length::from_meters(90.0)));
        let height23 = height5 + (90.0 - height5) * (1.0 - (-18.0_f64 / 7.5).exp());
        let baseline = 100.0 + (height5 - 100.0) * 3.0 / 5.0;
        assert_abs_diff_eq!(
            second.as_meters_per_second(),
            (height23 - baseline) / 20.0,
            epsilon = 1e-12
        );
    }
}
