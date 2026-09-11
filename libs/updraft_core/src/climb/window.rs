use std::collections::VecDeque;
use std::time::Duration;
use updraft_units::{Length, Speed};

#[derive(Debug, Default)]
pub struct ClimbWindow {
    samples: VecDeque<(Duration, Length)>,
}

impl ClimbWindow {
    pub fn observe(&mut self, time: Duration, altitude: Length) {
        if self.samples.back().is_some_and(|&(at, _)| time <= at) {
            return;
        }
        self.samples.push_back((time, altitude));
        while self.samples.len() > 2
            && self.samples[1].0 <= time.saturating_sub(Duration::from_secs(30))
        {
            self.samples.pop_front();
        }
    }

    pub fn average(&self, window: Duration) -> Option<Speed> {
        let &(end, altitude) = self.samples.back()?;
        let start = end.saturating_sub(window).max(self.samples.front()?.0);
        let index = self.samples.partition_point(|&(at, _)| at <= start);
        let &(left, low) = self.samples.get(index.checked_sub(1)?)?;
        let &(right, high) = self.samples.get(index)?;
        let baseline = low + (high - low) * (start - left).div_duration_f64(right - left);
        Some((altitude - baseline) / (end - start))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use claims::{assert_none, assert_some_eq};

    #[test]
    fn averages_partial_history_and_interpolates_window_boundaries() {
        let mut window = ClimbWindow::default();
        assert_none!(window.average(Duration::from_secs(20)));
        window.observe(Duration::from_secs(0), Length::from_meters(100.0));
        assert_none!(window.average(Duration::from_secs(20)));
        window.observe(Duration::from_secs(10), Length::from_meters(120.0));
        assert_some_eq!(
            window.average(Duration::from_secs(30)),
            Speed::from_meters_per_second(2.0)
        );
        window.observe(Duration::from_secs(35), Length::from_meters(95.0));
        assert_some_eq!(
            window.average(Duration::from_secs(20)),
            Speed::from_meters_per_second(-1.0)
        );
        assert_some_eq!(
            window.average(Duration::from_secs(30)),
            Speed::from_meters_per_second(-0.5)
        );
        window.observe(Duration::from_secs(65), Length::from_meters(155.0));
        assert_some_eq!(
            window.average(Duration::from_secs(30)),
            Speed::from_meters_per_second(2.0)
        );
        assert_some_eq!(
            window.average(Duration::from_secs(20)),
            Speed::from_meters_per_second(2.0)
        );
        assert_eq!(window.samples.len(), 2);
    }

    #[test]
    fn ignores_duplicate_and_reversed_timestamps() {
        let mut window = ClimbWindow::default();
        window.observe(Duration::from_secs(10), Length::from_meters(100.0));
        window.observe(Duration::from_secs(10), Length::from_meters(900.0));
        window.observe(Duration::from_secs(5), Length::from_meters(900.0));
        window.observe(Duration::from_secs(20), Length::from_meters(120.0));
        assert_some_eq!(
            window.average(Duration::from_secs(20)),
            Speed::from_meters_per_second(2.0)
        );
    }
}
