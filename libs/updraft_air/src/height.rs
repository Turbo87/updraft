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

/// How close in time the two altitudes have to be to be subtracted from
/// each other, in seconds. In a 2 m/s climb, one second apart would put
/// two metres of climb into the offset.
const PAIRING_TOLERANCE: f64 = 0.2;

/// How long a GNSS altitude waits for a pressure altitude to pair with,
/// in seconds.
const MAX_PAIRING_WAIT: f64 = 2.;

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
/// The two arrive on their own calls, at their own rates. Pressure drives
/// the height, because a barometer can report many times per second where
/// a GNSS receiver reports once. A GNSS altitude only moves the offset,
/// and it waits for a pressure altitude close enough in time to subtract
/// from, whichever of the two arrives first.
///
/// Some GNSS receivers smooth their altitude output and report it a
/// second late. Averaging a delayed copy of the same signal is worse than
/// not averaging at all, so the filter compares the GNSS height rate
/// against the pressure height rate over the same interval and over the
/// one before it, and stops using the GNSS altitude while the earlier one
/// fits better.
#[derive(Clone, Copy, Debug, Default)]
pub struct HeightFilter {
    offset: Option<f64>,
    /// Latest pressure altitude and its time.
    pressure: Option<(f64, f64)>,
    /// GNSS altitude and its time, waiting for a pressure altitude.
    pending: Option<(f64, f64)>,
    previous: Option<Previous>,
    /// Mean square of the height-rate difference over the same interval.
    matched: f64,
    /// Mean square of the height-rate difference one interval back.
    delayed: f64,
}

/// What the previous paired GNSS altitude saw, for the lag check.
#[derive(Clone, Copy, Debug)]
struct Previous {
    time: f64,
    gnss: f64,
    pressure: f64,
    /// Pressure height rate over the interval that ended there.
    pressure_rate: Option<f64>,
}

impl HeightFilter {
    /// Takes a pressure altitude and returns the height to derive vertical
    /// speed from, in metres. It shares the pressure altitude's reference,
    /// so only its changes are meaningful.
    pub fn pressure(&mut self, time: f64, altitude: f64) -> f64 {
        self.pressure = Some((time, altitude));
        if let Some((gnss_time, gnss)) = self.pending {
            if (gnss_time - time).abs() <= PAIRING_TOLERANCE {
                self.pending = None;
                self.pair(gnss_time, gnss, altitude);
            } else if time - gnss_time > MAX_PAIRING_WAIT {
                self.pending = None;
            }
        }
        altitude + self.offset.unwrap_or(0.)
    }

    /// Whether the offset between the two altitudes is established. The
    /// height steps by that offset when it first is, so the caller has to
    /// treat the heights either side of that as unrelated.
    pub fn is_fused(&self) -> bool {
        self.offset.is_some()
    }

    /// Takes a GNSS altitude. It moves the offset that
    /// [`pressure`](Self::pressure) adds, and never the height directly.
    pub fn gnss(&mut self, time: f64, altitude: f64) {
        match self.pressure {
            Some((pressure_time, pressure))
                if (time - pressure_time).abs() <= PAIRING_TOLERANCE =>
            {
                self.pair(time, altitude, pressure);
            }
            _ => self.pending = Some((time, altitude)),
        }
    }

    /// Folds one pair of altitudes measured at the same moment into the
    /// lag check and the offset.
    fn pair(&mut self, time: f64, gnss: f64, pressure: f64) {
        let interval = self
            .previous
            .map(|previous| time - previous.time)
            .filter(|interval| *interval > 0.);
        let rate = self.check_lag(time, gnss, pressure);
        self.previous = Some(Previous {
            time,
            gnss,
            pressure,
            pressure_rate: rate,
        });

        // A delayed GNSS altitude fits the earlier pressure rate better.
        if self.delayed < self.matched {
            return;
        }
        let difference = gnss - pressure;
        self.offset = Some(match self.offset.zip(interval) {
            Some((offset, interval)) => {
                let weight = smoothing_weight(interval, OFFSET_TIME_CONSTANT);
                offset + weight * (difference - offset)
            }
            None => difference,
        });
    }

    /// Folds this pair into the lag check, and returns the pressure height
    /// rate over the interval that ends here.
    fn check_lag(&mut self, time: f64, gnss: f64, pressure: f64) -> Option<f64> {
        let previous = self.previous?;
        let interval = time - previous.time;
        if interval <= 0. {
            return None;
        }

        let rate = (pressure - previous.pressure) / interval;
        if let Some(earlier_rate) = previous.pressure_rate {
            let gnss_rate = (gnss - previous.gnss) / interval;
            let weight = smoothing_weight(interval, LAG_TIME_CONSTANT);
            self.matched += weight * ((gnss_rate - rate).powi(2) - self.matched);
            self.delayed += weight * ((gnss_rate - earlier_rate).powi(2) - self.delayed);
        }
        Some(rate)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_abs_diff_eq;

    /// Climbs at `rate` m/s with the GNSS altitude offset by 200 m and
    /// delayed by `delay` seconds. `fix_first` sends the GNSS altitude
    /// before the pressure altitude of the same second. Returns the height
    /// rate over the last second.
    fn climb(rate: f64, delay: f64, seconds: usize, fix_first: bool) -> f64 {
        let mut filter = HeightFilter::default();
        let mut last = 0.;
        let mut previous = 0.;
        for second in 0..seconds {
            let time = second as f64;
            let gnss = 200. + rate * (time - delay).max(0.);
            previous = last;
            if fix_first {
                filter.gnss(time, gnss);
                last = filter.pressure(time, rate * time);
            } else {
                last = filter.pressure(time, rate * time);
                filter.gnss(time, gnss);
            }
        }
        last - previous
    }

    #[test]
    fn a_matching_gnss_altitude_keeps_the_height_rate() {
        assert_abs_diff_eq!(climb(2., 0., 300, false), 2., epsilon = 0.01);
    }

    #[test]
    fn the_order_the_two_altitudes_arrive_in_does_not_matter() {
        assert_abs_diff_eq!(climb(2., 0., 300, true), 2., epsilon = 0.01);
    }

    #[test]
    fn a_delayed_gnss_altitude_is_dropped() {
        assert_abs_diff_eq!(climb(2., 1., 3000, false), 2., epsilon = 0.01);
    }

    #[test]
    fn the_offset_does_not_leak_into_the_height_rate() {
        let mut filter = HeightFilter::default();
        filter.pressure(0., 1000.);
        filter.gnss(0., 1200.);

        let first = filter.pressure(1., 1000.);
        filter.gnss(1., 1200.);
        let second = filter.pressure(2., 1002.);

        // The 200 m offset is absorbed, so the rate is the 2 m/s climb.
        assert_abs_diff_eq!(second - first, 2., epsilon = 0.01);
    }

    #[test]
    fn a_missing_gnss_altitude_falls_back_to_the_pressure_altitude() {
        let mut filter = HeightFilter::default();
        let first = filter.pressure(0., 1002.);
        let second = filter.pressure(1., 1004.);

        assert_abs_diff_eq!(second - first, 2., epsilon = 1e-9);
    }

    #[test]
    fn pressure_between_two_gnss_altitudes_carries_the_height() {
        let mut filter = HeightFilter::default();
        filter.pressure(0., 1000.);
        filter.gnss(0., 1200.);

        // Ten pressure samples per GNSS sample, climbing at 2 m/s.
        let mut height = filter.pressure(1., 1000.);
        for tenth in 1..=10 {
            let time = 1. + 0.1 * f64::from(tenth);
            height = filter.pressure(time, 1000. + 0.2 * f64::from(tenth));
        }

        assert_abs_diff_eq!(height - 1200., 2., epsilon = 1e-9);
    }
}
