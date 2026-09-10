use std::collections::VecDeque;

#[derive(Debug, Default)]
pub struct ClimbWindow {
    samples: VecDeque<(f64, f64)>,
}

impl ClimbWindow {
    pub fn observe(&mut self, seconds: f64, altitude: f64) {
        if self.samples.back().is_some_and(|&(at, _)| seconds <= at) {
            return;
        }
        self.samples.push_back((seconds, altitude));
        while self.samples.len() > 2 && self.samples[1].0 <= seconds - 30.0 {
            self.samples.pop_front();
        }
    }

    pub fn average(&self, window: f64) -> Option<f64> {
        let &(end, altitude) = self.samples.back()?;
        let start = (end - window).max(self.samples.front()?.0);
        let index = self.samples.partition_point(|&(at, _)| at <= start);
        let &(left, low) = self.samples.get(index.checked_sub(1)?)?;
        let &(right, high) = self.samples.get(index)?;
        let baseline = low + (high - low) * (start - left) / (right - left);
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
        assert_none!(window.average(20.0));
        window.observe(0.0, 100.0);
        assert_none!(window.average(20.0));
        window.observe(10.0, 120.0);
        assert_some_eq!(window.average(30.0), 2.0);
        window.observe(35.0, 95.0);
        assert_some_eq!(window.average(20.0), -1.0);
        assert_some_eq!(window.average(30.0), -0.5);
        window.observe(65.0, 155.0);
        assert_some_eq!(window.average(30.0), 2.0);
        assert_some_eq!(window.average(20.0), 2.0);
        assert_eq!(window.samples.len(), 2);
    }

    #[test]
    fn ignores_duplicate_and_reversed_timestamps() {
        let mut window = ClimbWindow::default();
        window.observe(10.0, 100.0);
        window.observe(10.0, 900.0);
        window.observe(5.0, 900.0);
        window.observe(20.0, 120.0);
        assert_some_eq!(window.average(20.0), 2.0);
    }
}
