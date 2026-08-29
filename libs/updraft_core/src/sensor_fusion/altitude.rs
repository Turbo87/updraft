use super::smoothing::smoothing_weight;
use std::time::Duration;
use updraft_units::Length;

/// Time constant of the offset tracking, in seconds. It is the crossover
/// of the complementary filter: the GNSS altitude carries the altitude
/// changes that are slower than this, the pressure altitude the faster
/// ones.
const OFFSET_TIME_CONSTANT: Duration = Duration::from_secs(5);

/// How close in time the two altitudes have to be to be subtracted from
/// each other, in seconds. In a 2 m/s climb, one second apart would put
/// two metres of climb into the offset.
const PAIRING_TOLERANCE: Duration = Duration::from_millis(200);

/// How long a GNSS altitude waits for a pressure altitude to pair with,
/// in seconds.
const MAX_PAIRING_WAIT: Duration = Duration::from_secs(2);

/// Combines the pressure altitude with the GNSS altitude.
///
/// Pressure and GNSS altitude differ by the altimeter setting, the geoid,
/// and the deviation from the ISA temperature. This filter tracks that
/// difference as a slowly moving offset, which makes the GNSS altitude
/// carry the slow altitude changes and the pressure altitude the fast ones.
///
/// The two arrive on their own calls, at their own rates. Pressure drives
/// the altitude, because a barometer can report many times per second where
/// a GNSS receiver reports once. A GNSS altitude only moves the offset,
/// and it waits for a pressure altitude close enough in time to subtract
/// from, whichever of the two arrives first.
#[derive(Clone, Copy, Debug, Default)]
pub struct AltitudeFilter {
    offset: Option<Length>,
    /// Latest pressure altitude and its time.
    pressure: Option<(Duration, Length)>,
    /// GNSS altitude and its time, waiting for a pressure altitude.
    pending: Option<(Duration, Length)>,
    /// Time the last pair was folded in at.
    previous: Option<Duration>,
    /// How far the altitude jumped when the offset was first established,
    /// until [`take_step`](Self::take_step) collects it.
    step: Length,
}

impl AltitudeFilter {
    /// Takes a pressure altitude and returns the altitude to derive vertical
    /// speed from. It shares the pressure altitude's reference,
    /// so only its changes are meaningful.
    pub fn pressure(&mut self, time: Duration, altitude: Length) -> Length {
        self.pressure = Some((time, altitude));
        if let Some((gnss_time, gnss)) = self.pending {
            if gnss_time.abs_diff(time) <= PAIRING_TOLERANCE {
                self.pending = None;
                self.pair(gnss_time, gnss, altitude);
            } else if time
                .checked_sub(gnss_time)
                .is_some_and(|wait| wait > MAX_PAIRING_WAIT)
            {
                self.pending = None;
            }
        }
        altitude + self.offset.unwrap_or_default()
    }

    /// Clears the GNSS reference without introducing a step in the next
    /// pressure-derived vertical speed.
    pub fn clear_gnss_reference(&mut self) {
        if let Some(offset) = self.offset.take() {
            self.step -= offset;
        }
        self.pending = None;
        self.previous = None;
    }

    /// Returns the latest pressure-driven altitude after GNSS establishes
    /// its reference.
    pub fn referenced_altitude(&self) -> Option<Length> {
        let (_, pressure) = self.pressure?;
        Some(pressure + self.offset?)
    }

    /// Takes a GNSS altitude. It moves the offset that
    /// [`pressure`](Self::pressure) adds, and never the altitude directly.
    pub fn gnss(&mut self, time: Duration, altitude: Length) {
        match self.pressure {
            Some((pressure_time, pressure))
                if time.abs_diff(pressure_time) <= PAIRING_TOLERANCE =>
            {
                self.pair(time, altitude, pressure);
            }
            _ => self.pending = Some((time, altitude)),
        }
    }

    /// Folds one pair of altitudes measured at the same moment into the
    /// offset.
    fn pair(&mut self, time: Duration, gnss: Length, pressure: Length) {
        let interval = self
            .previous
            .and_then(|previous| time.checked_sub(previous))
            .filter(|interval| !interval.is_zero());
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
            // this as the first pair would step the altitude by the whole
            // datum difference a second time.
            (Some(offset), None) => offset,
            (None, _) => {
                // The altitude moves by the whole offset here, which is a
                // change of reference and not a climb.
                self.step += difference;
                difference
            }
        });
    }

    /// Collects the step that the last change of reference put into the
    /// altitude, and clears it. The caller has to take it out of the
    /// difference it derives a vertical speed from.
    pub fn take_step(&mut self) -> Length {
        std::mem::take(&mut self.step)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_abs_diff_eq;

    fn at(value: f64) -> Duration {
        Duration::from_secs_f64(value)
    }

    fn meters(value: f64) -> Length {
        Length::from_meters(value)
    }

    /// Climbs at `rate` m/s with the GNSS altitude offset by 200 m.
    /// `fix_first` sends the GNSS altitude before the pressure altitude
    /// of the same second. Returns the altitude change over the last second.
    fn climb(rate: f64, seconds: usize, fix_first: bool) -> Length {
        let mut filter = AltitudeFilter::default();
        let mut last = Length::ZERO;
        let mut previous = Length::ZERO;
        for second in 0..seconds {
            let elapsed = second as f64;
            let time = at(elapsed);
            let gnss = meters(200. + rate * elapsed);
            previous = last;
            if fix_first {
                filter.gnss(time, gnss);
                last = filter.pressure(time, meters(rate * elapsed));
            } else {
                last = filter.pressure(time, meters(rate * elapsed));
                filter.gnss(time, gnss);
            }
        }
        last - previous
    }

    #[test]
    fn matching_gnss_altitude_preserves_altitude_rate() {
        assert_abs_diff_eq!(climb(2., 300, false), meters(2.), epsilon = 0.01);
    }

    #[test]
    fn altitude_arrival_order_does_not_change_altitude_rate() {
        assert_abs_diff_eq!(climb(2., 300, true), meters(2.), epsilon = 0.01);
    }

    #[test]
    fn altitude_offset_does_not_change_altitude_rate() {
        // The two altitudes are 200 m apart and both climb at 2 m/s. The
        // offset holds that difference and the altitude keeps the climb.
        let mut filter = AltitudeFilter::default();
        for second in 0..300 {
            let elapsed = f64::from(second);
            let time = at(elapsed);
            filter.pressure(time, meters(2. * elapsed));
            filter.gnss(time, meters(200. + 2. * elapsed));
        }

        let altitude = filter.pressure(at(300.), meters(600.));
        assert_abs_diff_eq!(altitude, meters(800.), epsilon = 0.5);
    }

    #[test]
    fn repeated_gnss_altitude_does_not_repeat_altitude_step() {
        // A receiver that reports the altitude in two sentences pairs
        // twice against the same second. The offset is established by
        // then, so the second pair must not be taken for the first: that
        // would hand the caller the whole 200 m datum difference as a
        // step, and the vario would rebase by it.
        let mut filter = AltitudeFilter::default();
        for second in 0..60 {
            let time = Duration::from_secs(second);
            filter.pressure(time, meters(1000.));
            filter.gnss(time, meters(1200.));
            filter.gnss(time, meters(1200.));
        }

        assert_abs_diff_eq!(filter.take_step(), meters(200.), epsilon = 0.01);
        assert_abs_diff_eq!(filter.take_step(), Length::ZERO, epsilon = 1e-9);
    }

    #[test]
    fn unpaired_gnss_altitude_expires() {
        let mut filter = AltitudeFilter::default();
        filter.gnss(Duration::ZERO, meters(1200.));
        // Later than `MAX_PAIRING_WAIT`, so the two are not the same
        // moment and subtracting them would put a climb into the offset.
        let altitude = filter.pressure(Duration::from_secs(3), meters(1000.));

        // No offset was established, so the altitude is the pressure
        // altitude alone and no step is owed to the caller.
        assert_abs_diff_eq!(altitude, meters(1000.), epsilon = 1e-9);
        assert_abs_diff_eq!(filter.take_step(), Length::ZERO, epsilon = 1e-9);
    }
}
