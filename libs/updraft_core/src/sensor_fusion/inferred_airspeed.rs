use super::smoothing::smoothing_weight;
use std::time::Duration;
use updraft_units::Speed;

const CRUISE_TIME_CONSTANT: Duration = Duration::from_secs(2);
const TURN_TIME_CONSTANT: Duration = Duration::from_secs(5);
const MAX_INTERVAL: Duration = Duration::from_secs(5);

#[derive(Clone, Copy, Debug)]
struct Sample {
    time: Duration,
    raw: Speed,
    filtered: Speed,
}

/// Smooths airspeed inferred from GNSS velocity and wind.
#[derive(Clone, Debug, Default)]
pub struct InferredAirspeed {
    sample: Option<Sample>,
}

impl InferredAirspeed {
    pub fn update(&mut self, time: Duration, value: Speed, turning: bool) -> Speed {
        let Some(previous) = self.sample.filter(|previous| time >= previous.time) else {
            self.sample = Some(Sample {
                time,
                raw: value,
                filtered: value,
            });
            return value;
        };
        let interval = time - previous.time;
        if interval.is_zero() {
            return previous.filtered;
        }
        let value = if interval > MAX_INTERVAL {
            value
        } else {
            let time_constant = match turning {
                true => TURN_TIME_CONSTANT,
                false => CRUISE_TIME_CONSTANT,
            };
            let weight = smoothing_weight(interval, time_constant);
            previous.filtered + weight * (value - previous.filtered)
        };
        self.sample = Some(Sample {
            time,
            raw: value,
            filtered: value,
        });
        value
    }

    pub fn latest_raw(&self) -> Option<Speed> {
        self.sample.map(|sample| sample.raw)
    }

    pub fn fresh_at(&self, now: Duration) -> Option<Speed> {
        let sample = self.sample?;
        now.checked_sub(sample.time)
            .filter(|age| *age <= MAX_INTERVAL)
            .map(|_| sample.filtered)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_abs_diff_eq;
    use claims::assert_none;

    #[test]
    fn turning_airspeed_changes_more_slowly() {
        let mut cruise = InferredAirspeed::default();
        let mut turning = InferredAirspeed::default();
        cruise.update(Duration::ZERO, Speed::from_meters_per_second(30.), false);
        turning.update(Duration::ZERO, Speed::from_meters_per_second(30.), true);

        let speed = Speed::from_meters_per_second(20.);
        let cruise = cruise.update(Duration::from_secs(1), speed, false);
        let turning = turning.update(Duration::from_secs(1), speed, true);

        assert_abs_diff_eq!(
            cruise,
            Speed::from_meters_per_second(26.065306597126334),
            epsilon = 1e-12
        );
        assert_abs_diff_eq!(
            turning,
            Speed::from_meters_per_second(28.187307530779817),
            epsilon = 1e-12
        );
    }

    #[test]
    fn stale_airspeed_is_not_reported() {
        let mut airspeed = InferredAirspeed::default();
        airspeed.update(Duration::ZERO, Speed::from_meters_per_second(30.), false);

        assert_none!(airspeed.fresh_at(MAX_INTERVAL + Duration::from_nanos(1)));
    }
}
