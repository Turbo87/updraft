use crate::smoothing_weight;

/// Time constant of the offset tracking, in seconds. It is the crossover
/// of the complementary filter: the GNSS altitude carries the height
/// changes that are slower than this, the pressure altitude the faster
/// ones.
const OFFSET_TIME_CONSTANT: f64 = 5.;

/// Time constant of the lag check, in seconds. It has to average over
/// many climbs and glides, because a single one can favour either
/// alignment by chance.
const LAG_TIME_CONSTANT: f64 = 120.;

/// Combines the pressure altitude with the GNSS altitude.
///
/// Both measure the same height change through two unrelated error
/// sources, so averaging them is worth about 10% of the vertical-speed
/// error. They cannot be averaged directly: they differ by the altimeter
/// setting, the geoid, and the deviation from the ISA temperature. This
/// filter tracks that difference as a slowly moving offset, which makes
/// the GNSS altitude carry the slow height changes and the pressure
/// altitude the fast ones.
///
/// Some GNSS receivers smooth their altitude output and report it a
/// second late. Averaging a delayed copy of the same signal is worse than
/// not averaging at all, so the filter compares the GNSS height rate
/// against the current and the previous pressure height rate, and stops
/// using the GNSS altitude while the previous one fits better.
#[derive(Clone, Copy, Debug, Default)]
pub struct HeightFilter {
    offset: Option<f64>,
    previous: Option<Previous>,
    /// Mean square of the height-rate difference at the current sample.
    matched: f64,
    /// Mean square of the height-rate difference one sample back.
    delayed: f64,
}

#[derive(Clone, Copy, Debug)]
struct Previous {
    pressure: f64,
    gnss: Option<f64>,
    /// Pressure height rate leading up to the previous sample.
    pressure_rate: Option<f64>,
}

impl HeightFilter {
    /// The height to derive vertical speed from, in metres. It shares the
    /// pressure altitude's reference, so only its changes are meaningful.
    pub fn update(&mut self, interval: Option<f64>, pressure: f64, gnss: Option<f64>) -> f64 {
        let previous = self.previous.take();
        let rate = interval
            .zip(previous)
            .map(|(interval, previous)| (pressure - previous.pressure) / interval);
        self.check_lag(interval, gnss, rate, previous);
        self.previous = Some(Previous {
            pressure,
            gnss,
            pressure_rate: rate,
        });

        // A delayed GNSS altitude fits the previous pressure rate better.
        let usable = gnss.filter(|_| self.delayed >= self.matched);
        if let Some(gnss) = usable {
            let difference = gnss - pressure;
            self.offset = Some(match self.offset {
                Some(offset) => {
                    let weight = interval.map_or(1., |i| smoothing_weight(i, OFFSET_TIME_CONSTANT));
                    offset + weight * (difference - offset)
                }
                None => difference,
            });
        }
        pressure + self.offset.unwrap_or(0.)
    }

    fn check_lag(
        &mut self,
        interval: Option<f64>,
        gnss: Option<f64>,
        rate: Option<f64>,
        previous: Option<Previous>,
    ) {
        let Some(previous) = previous else { return };
        let (Some(interval), Some(rate), Some(gnss), Some(previous_gnss), Some(earlier_rate)) =
            (interval, rate, gnss, previous.gnss, previous.pressure_rate)
        else {
            return;
        };

        let gnss_rate = (gnss - previous_gnss) / interval;
        let weight = smoothing_weight(interval, LAG_TIME_CONSTANT);
        self.matched += weight * ((gnss_rate - rate).powi(2) - self.matched);
        self.delayed += weight * ((gnss_rate - earlier_rate).powi(2) - self.delayed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_abs_diff_eq;

    /// Runs a climb at `rate` m/s, with the GNSS altitude offset by 200 m
    /// and delayed by `delay` samples. Returns the height rate the filter
    /// produces over the last second.
    fn climb(rate: f64, delay: usize, seconds: usize) -> f64 {
        let mut filter = HeightFilter::default();
        let mut last = 0.;
        let mut previous = 0.;
        for second in 0..seconds {
            let pressure = rate * second as f64;
            let gnss = 200. + rate * second.saturating_sub(delay) as f64;
            previous = last;
            last = filter.update((second > 0).then_some(1.), pressure, Some(gnss));
        }
        last - previous
    }

    #[test]
    fn a_matching_gnss_altitude_keeps_the_height_rate() {
        assert_abs_diff_eq!(climb(2., 0, 300), 2., epsilon = 0.01);
    }

    #[test]
    fn the_offset_does_not_leak_into_the_height_rate() {
        let mut filter = HeightFilter::default();
        let first = filter.update(None, 1000., Some(1200.));
        let second = filter.update(Some(1.), 1002., Some(1202.));

        // The 200 m offset is absorbed, so the rate is the 2 m/s climb.
        assert_abs_diff_eq!(second - first, 2., epsilon = 0.01);
    }

    #[test]
    fn a_delayed_gnss_altitude_is_dropped() {
        // A one-sample delay biases the rate if it is averaged in. The
        // lag check needs a while to settle, so the early samples still
        // use it.
        assert_abs_diff_eq!(climb(2., 1, 3000), 2., epsilon = 0.01);
    }

    #[test]
    fn a_missing_gnss_altitude_falls_back_to_the_pressure_altitude() {
        let mut filter = HeightFilter::default();
        filter.update(None, 1000., None);
        let first = filter.update(Some(1.), 1002., None);
        let second = filter.update(Some(1.), 1004., None);

        assert_abs_diff_eq!(second - first, 2., epsilon = 1e-9);
    }
}
