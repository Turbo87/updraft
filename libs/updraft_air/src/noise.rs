/// Second differences held before the floor is trusted. At 1 Hz this is
/// two minutes, and at 25 Hz it is five seconds.
const WINDOW: usize = 128;

/// Which of the sorted second differences is taken as the noise. Real
/// flying only ever adds to a second difference, so the floor is a low
/// order statistic and not a mean.
const PERCENTILE: f64 = 0.25;

/// Longest sample interval the floor can be measured across, in seconds.
///
/// A second difference holds the noise plus `a·Δt²` of whatever the
/// glider is doing. At 25 Hz a vertical acceleration of 0.5 m/s² adds
/// 0.0008 m, far below any barometer. At 1 Hz it adds 0.5 m, which is
/// larger than the noise of a logged pressure altitude, and no order
/// statistic recovers the sensor from that.
const MAX_SAMPLE_INTERVAL: f64 = 0.2;

/// Time constant of the interval tracking, in seconds.
const INTERVAL_TIME_CONSTANT: f64 = 10.;

/// The quarter-point of `|second difference|`, divided by the noise of
/// one sample, for a normal source. A second difference of white noise
/// has `√6` times its standard deviation, and the quarter point of a
/// folded normal is `0.3186` of it.
const FLOOR_GAIN: f64 = 0.780_5;

/// Measures the noise and the resolution of an altitude source.
///
/// The noise is the *floor* of the second difference, not its average. A
/// climb, a gust or a pull-up all raise a second difference and none of
/// them lowers it, so the quietest quarter of a window measures the
/// sensor while the glider does whatever it likes.
///
/// The resolution is the smallest step between two readings that differ.
/// It matters because a source whose noise falls below its own step has
/// had the noise removed by something, and the measurement can no longer
/// say whether the source is quiet or slow. That is the trap the uBLOX
/// LEA-4S, the Galaxy S23 driver and the LG G7 public sensor each set in
/// turn, and the caller has to stay conservative when it is sprung.
#[derive(Clone, Debug, Default)]
pub struct NoiseFloor {
    /// The last two altitudes, for the second difference.
    previous: Option<(f64, f64)>,
    /// Time of the last sample, and the tracked interval between them.
    last_time: Option<f64>,
    interval: Option<f64>,
    differences: Vec<f64>,
    next: usize,
    filled: bool,
    step: Option<f64>,
}

impl NoiseFloor {
    /// Takes the next altitude of the source, in metres. Samples must
    /// arrive at a steady rate, because a second difference across two
    /// unequal intervals measures the intervals.
    pub fn update(&mut self, time: f64, altitude: f64) {
        if !altitude.is_finite() || !time.is_finite() {
            return;
        }
        if let Some(last) = self.last_time {
            let interval = time - last;
            if interval > 0. {
                self.interval = Some(match self.interval {
                    Some(tracked) => {
                        let weight = crate::smoothing_weight(interval, INTERVAL_TIME_CONSTANT);
                        tracked + weight * (interval - tracked)
                    }
                    None => interval,
                });
            }
        }
        self.last_time = Some(time);
        if let Some((older, newer)) = self.previous {
            let difference = (altitude - 2. * newer + older).abs();
            if self.differences.len() < WINDOW {
                self.differences.push(difference);
            } else {
                self.differences[self.next] = difference;
                self.next = (self.next + 1) % WINDOW;
                self.filled = true;
            }

            let step = (altitude - newer).abs();
            if step > 0. {
                self.step = Some(self.step.map_or(step, |smallest: f64| smallest.min(step)));
            }
        }
        self.previous = Some(match self.previous {
            Some((_, newer)) => (newer, altitude),
            None => (altitude, altitude),
        });
    }

    /// The noise of one sample, in metres, once a whole window has
    /// arrived from a source fast enough to measure.
    pub fn noise(&self) -> Option<f64> {
        if !self.filled && self.differences.len() < WINDOW {
            return None;
        }
        if self.interval? > MAX_SAMPLE_INTERVAL {
            return None;
        }
        let mut sorted = self.differences.clone();
        sorted.sort_by(f64::total_cmp);
        let floor = sorted[(sorted.len() as f64 * PERCENTILE) as usize];
        Some(floor / FLOOR_GAIN)
    }

    /// Whether the measured noise is larger than the resolution of the
    /// source, so that it measures the sensor rather than the quantiser.
    pub fn resolves_its_own_noise(&self) -> bool {
        match (self.noise(), self.step) {
            (Some(noise), Some(step)) => noise > step,
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use claims::{assert_gt, assert_lt, assert_none, assert_some};

    /// A deterministic sequence that is flat in the mean, with noise of
    /// standard deviation `sigma`. Three uniforms are summed, because the
    /// floor is calibrated for a normal source.
    fn noisy(sigma: f64, count: usize) -> Vec<f64> {
        let uniform = |seed: f64| {
            let x = (seed * 12.9898).sin() * 43758.5453;
            x - x.floor() - 0.5
        };
        (0..count)
            .map(|i| {
                let i = i as f64;
                // Three uniforms have a standard deviation of 0.5.
                1000. + 2. * sigma * (uniform(i) + uniform(i + 977.) + uniform(i + 1949.))
            })
            .collect()
    }

    #[test]
    fn the_floor_recovers_the_noise_it_was_given() {
        for sigma in [0.05, 0.2, 0.8] {
            let mut floor = NoiseFloor::default();
            for (index, altitude) in noisy(sigma, 4000).into_iter().enumerate() {
                floor.update(index as f64 * 0.04, altitude);
            }

            let measured = assert_some!(floor.noise());
            assert_lt!((measured - sigma).abs() / sigma, 0.3);
        }
    }

    #[test]
    fn a_slow_source_cannot_be_measured() {
        let mut floor = NoiseFloor::default();
        for (index, altitude) in noisy(0.4, 400).into_iter().enumerate() {
            floor.update(index as f64, altitude);
        }

        // One sample per second: a second difference spans two seconds,
        // and what the glider did in them is larger than the sensor.
        assert_none!(floor.noise());
    }

    #[test]
    fn a_short_history_measures_nothing() {
        let mut floor = NoiseFloor::default();
        for (index, altitude) in noisy(1., 20).into_iter().enumerate() {
            floor.update(index as f64 * 0.04, altitude);
        }

        assert_none!(floor.noise());
    }

    #[test]
    fn a_quiet_source_reads_below_a_noisy_one() {
        let mut quiet = NoiseFloor::default();
        let mut loud = NoiseFloor::default();
        for (index, altitude) in noisy(0.1, 400).into_iter().enumerate() {
            quiet.update(index as f64 * 0.04, altitude);
        }
        for (index, altitude) in noisy(2., 400).into_iter().enumerate() {
            loud.update(index as f64 * 0.04, altitude);
        }

        assert_lt!(assert_some!(quiet.noise()), assert_some!(loud.noise()));
    }

    #[test]
    fn a_climb_does_not_count_as_noise() {
        let mut level = NoiseFloor::default();
        let mut climbing = NoiseFloor::default();
        for (index, altitude) in noisy(0.5, 400).into_iter().enumerate() {
            let time = index as f64 * 0.04;
            level.update(time, altitude);
            climbing.update(time, altitude + 0.08 * index as f64);
        }

        // A steady climb has no second difference at all, so the floor
        // has to be the same.
        let (a, b) = (assert_some!(level.noise()), assert_some!(climbing.noise()));
        assert_lt!((a - b).abs(), 1e-9);
    }

    #[test]
    fn a_quantised_source_does_not_resolve_its_own_noise() {
        let mut coarse = NoiseFloor::default();
        for (index, altitude) in noisy(0.4, 400).into_iter().enumerate() {
            // One metre steps, far larger than the noise.
            coarse.update(index as f64 * 0.04, altitude.round());
        }

        assert!(!coarse.resolves_its_own_noise());
    }

    #[test]
    fn a_fine_source_resolves_its_own_noise() {
        let mut fine = NoiseFloor::default();
        for (index, altitude) in noisy(0.4, 400).into_iter().enumerate() {
            fine.update(index as f64 * 0.04, (altitude * 100.).round() / 100.);
        }

        assert!(fine.resolves_its_own_noise());
        assert_gt!(assert_some!(fine.noise()), 0.1);
    }
}
