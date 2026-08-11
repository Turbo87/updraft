use crate::smoothing_weight;

/// Time constant of the offset tracking, in seconds. It is the crossover
/// of the complementary filter: the GNSS altitude carries the height
/// changes that are slower than this, the pressure altitude the faster
/// ones.
const OFFSET_TIME_CONSTANT: f64 = 5.;

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
#[derive(Clone, Copy, Debug, Default)]
pub struct HeightFilter {
    offset: Option<f64>,
    /// Latest pressure altitude and its time.
    pressure: Option<(f64, f64)>,
    /// GNSS altitude and its time, waiting for a pressure altitude.
    pending: Option<(f64, f64)>,
    /// Time the last pair was folded in at.
    previous: Option<f64>,
    /// How far the height jumped when the offset was first established,
    /// until [`take_step`](Self::take_step) collects it.
    step: f64,
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
    /// offset.
    fn pair(&mut self, time: f64, gnss: f64, pressure: f64) {
        let interval = self
            .previous
            .map(|previous| time - previous)
            .filter(|interval| *interval > 0.);
        self.previous = Some(time);

        let difference = gnss - pressure;
        self.offset = Some(match (self.offset, interval) {
            (Some(offset), Some(interval)) => {
                let weight = smoothing_weight(interval, OFFSET_TIME_CONSTANT);
                offset + weight * (difference - offset)
            }
            // A second pair at the same time, or one that arrived out of
            // order, carries no elapsed time to smooth over. The offset
            // is already established, so it keeps its level: treating
            // this as the first pair would step the height by the whole
            // datum difference a second time.
            (Some(offset), None) => offset,
            (None, _) => {
                // The height moves by the whole offset here, which is a
                // change of reference and not a climb.
                self.step += difference;
                difference
            }
        });
    }

    /// Collects the step that the last change of reference put into the
    /// height, and clears it. The caller has to take it out of the
    /// difference it derives a vertical speed from.
    pub fn take_step(&mut self) -> f64 {
        std::mem::take(&mut self.step)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_abs_diff_eq;

    /// Climbs at `rate` m/s with the GNSS altitude offset by 200 m.
    /// `fix_first` sends the GNSS altitude before the pressure altitude
    /// of the same second. Returns the height rate over the last second.
    fn climb(rate: f64, seconds: usize, fix_first: bool) -> f64 {
        let mut filter = HeightFilter::default();
        let mut last = 0.;
        let mut previous = 0.;
        for second in 0..seconds {
            let time = second as f64;
            let gnss = 200. + rate * time;
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
        assert_abs_diff_eq!(climb(2., 300, false), 2., epsilon = 0.01);
    }

    #[test]
    fn the_order_the_two_altitudes_arrive_in_does_not_matter() {
        assert_abs_diff_eq!(climb(2., 300, true), 2., epsilon = 0.01);
    }

    #[test]
    fn the_offset_does_not_leak_into_the_height_rate() {
        // The two altitudes are 200 m apart and both climb at 2 m/s. The
        // offset holds that difference and the height keeps the climb.
        let mut filter = HeightFilter::default();
        for second in 0..300 {
            let time = f64::from(second);
            filter.pressure(time, 2. * time);
            filter.gnss(time, 200. + 2. * time);
        }

        let height = filter.pressure(300., 600.);
        assert_abs_diff_eq!(height, 800., epsilon = 0.5);
    }

    #[test]
    fn a_repeated_gnss_altitude_does_not_step_the_height_again() {
        // A receiver that reports the altitude in two sentences pairs
        // twice against the same second. The offset is established by
        // then, so the second pair must not be taken for the first: that
        // would hand the caller the whole 200 m datum difference as a
        // step, and the vario would rebase by it.
        let mut filter = HeightFilter::default();
        for second in 0..60 {
            let time = f64::from(second);
            filter.pressure(time, 1000.);
            filter.gnss(time, 1200.);
            filter.gnss(time, 1200.);
        }

        assert_abs_diff_eq!(filter.take_step(), 200., epsilon = 0.01);
        assert_abs_diff_eq!(filter.take_step(), 0., epsilon = 1e-9);
    }

    #[test]
    fn a_gnss_altitude_with_no_pressure_altitude_to_pair_with_expires() {
        let mut filter = HeightFilter::default();
        filter.gnss(0., 1200.);
        // Later than `MAX_PAIRING_WAIT`, so the two are not the same
        // moment and subtracting them would put a climb into the offset.
        let height = filter.pressure(3., 1000.);

        // No offset was established, so the height is the pressure
        // altitude alone and no step is owed to the caller.
        assert_abs_diff_eq!(height, 1000., epsilon = 1e-9);
        assert_abs_diff_eq!(filter.take_step(), 0., epsilon = 1e-9);
    }
}
