use super::velocity::TrafficVelocity;
use crate::climb::{
    ClimbEma, ClimbEstimates, ClimbWindow, EnergyClimb, SmoothedClimbWindow, Velocity,
};
use crate::ownship::SourceId;
use crate::{ExternalDeviceId, Timestamp};
use std::time::Duration;
use updraft_geo::LatLon;
use updraft_units::Length;

#[derive(Clone, Copy, Debug, Default)]
pub struct TrafficMotion {
    /// Same-device GPS time, unwrapped across midnight, for corrected altitude samples.
    pub gps_time: Option<Duration>,
    pub velocity: Option<Velocity>,
    pub wind: Option<Velocity>,
    pub position: Option<(SourceId, LatLon)>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SampleClock {
    Gps,
    Reception,
}

#[derive(Clone, Copy, Debug)]
struct SampleTiming {
    received_at: Timestamp,
    source: (ExternalDeviceId, SourceId),
    time: Duration,
    clock: SampleClock,
}

#[derive(Debug, Default)]
pub struct TrafficClimb {
    previous: Option<SampleTiming>,
    energy: EnergyClimb,
    velocity: TrafficVelocity,
    window: ClimbWindow,
    ema: ClimbEma,
    smoothed: SmoothedClimbWindow,
    estimates: Option<ClimbEstimates>,
}

impl TrafficClimb {
    pub fn expired(&self, at: Timestamp) -> bool {
        self.previous.is_none_or(|previous| {
            at.saturating_since(previous.received_at) > Duration::from_secs(60)
        })
    }

    pub fn observe(
        &mut self,
        source: (ExternalDeviceId, SourceId),
        at: Timestamp,
        altitude: Length,
        motion: TrafficMotion,
    ) -> Option<ClimbEstimates> {
        let time = motion.gps_time.unwrap_or(at.since_start());
        let clock = if motion.gps_time.is_some() {
            SampleClock::Gps
        } else {
            SampleClock::Reception
        };
        if let Some(previous) = self.previous {
            if at < previous.received_at {
                return self.estimates;
            }
            let same_clock = previous.clock == clock;
            if same_clock
                && !self.expired(at)
                && (time == previous.time
                    || (clock == SampleClock::Reception && time < previous.time))
            {
                return self.estimates;
            }
            if self.expired(at)
                || previous.source != source
                || !same_clock
                || time < previous.time
                || time.saturating_sub(previous.time) > Duration::from_secs(60)
            {
                *self = Self::default();
            }
        }
        self.previous = Some(SampleTiming {
            received_at: at,
            source,
            time,
            clock,
        });
        let velocity = self
            .velocity
            .observe(time, motion.position, motion.velocity);
        let (time, altitude) = self.energy.observe(time, altitude, velocity, motion.wind)?;
        self.window.observe(time, altitude);
        let smoothed_20s = self.smoothed.observe(time, altitude);
        self.estimates = self.ema.observe(time, altitude).and_then(|normalized_ema| {
            Some(ClimbEstimates {
                average_20s: self.window.average(Duration::from_secs(20))?,
                average_30s: self.window.average(Duration::from_secs(30))?,
                normalized_ema,
                smoothed_20s: smoothed_20s?,
            })
        });
        self.estimates
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use claims::{assert_none, assert_some, assert_some_eq};

    #[test]
    fn gps_sample_clock_ignores_duplicates_and_resets_on_rewind_or_clock_change() {
        let source = (ExternalDeviceId(1), SourceId::InternalGps);
        let mut climb = TrafficClimb::default();
        let mut observe = |received, sample: Option<u64>, height| {
            climb.observe(
                source,
                Timestamp::from_millis(received),
                Length::from_meters(height),
                TrafficMotion {
                    gps_time: sample.map(Duration::from_millis),
                    ..TrafficMotion::default()
                },
            )
        };
        assert_none!(observe(0, Some(1_000), 100.));
        let estimates = assert_some!(observe(100, Some(2_000), 102.));
        assert_eq!(
            estimates.average_20s,
            updraft_units::Speed::from_meters_per_second(2.)
        );
        assert_some_eq!(observe(200, Some(2_000), 900.), estimates);
        assert_none!(observe(300, Some(1_000), 100.));
        assert_some_eq!(observe(400, Some(2_000), 102.), estimates);
        assert_none!(observe(500, None, 104.));
        assert_some_eq!(observe(1_500, None, 106.), estimates);
        assert_none!(observe(1_600, Some(2_000), 102.));
        assert_some!(observe(1_700, Some(62_000), 222.));
        assert_none!(observe(1_800, Some(122_001), 342.));
    }

    #[test]
    fn compensates_reported_velocity_and_resets_with_the_altitude_source() {
        use crate::climb::Velocity;
        use updraft_units::{Angle, Speed};
        let source = (ExternalDeviceId(1), SourceId::InternalGps);
        let mut climb = TrafficClimb::default();
        let fast = Velocity::from_track(Angle::ZERO, Speed::from_meters_per_second(40.));
        let slow = Velocity::from_track(Angle::ZERO, Speed::from_meters_per_second(30.));
        let wind = Some(Velocity::default());
        assert_none!(climb.observe(
            source,
            Timestamp::from_millis(0),
            Length::ZERO,
            TrafficMotion {
                velocity: Some(fast),
                wind,
                ..TrafficMotion::default()
            }
        ));
        let gain = Length::from_meters(700. / (2. * 9.80665));
        let estimates = assert_some!(climb.observe(
            source,
            Timestamp::from_millis(10_000),
            gain,
            TrafficMotion {
                velocity: Some(slow),
                wind,
                ..TrafficMotion::default()
            }
        ));
        approx::assert_abs_diff_eq!(estimates.normalized_ema, Speed::ZERO, epsilon = 1e-12);
        let changed = (ExternalDeviceId(2), SourceId::InternalGps);
        assert_none!(climb.observe(
            changed,
            Timestamp::from_millis(11_000),
            gain,
            TrafficMotion {
                velocity: Some(fast),
                wind,
                ..TrafficMotion::default()
            }
        ));
    }

    #[test]
    fn ignores_old_samples_before_checking_source_and_resets_on_altitude_source_change() {
        let mut climb = TrafficClimb::default();
        let source = (ExternalDeviceId(1), SourceId::InternalGps);
        let changed = (ExternalDeviceId(1), SourceId::External(ExternalDeviceId(2)));
        let changed_altitude = Length::from_meters(900.0);
        assert_none!(climb.observe(
            source,
            Timestamp::from_millis(0),
            Length::from_meters(100.0),
            TrafficMotion::default()
        ));
        let estimate = assert_some!(climb.observe(
            source,
            Timestamp::from_millis(1_000),
            Length::from_meters(102.0),
            TrafficMotion::default()
        ));
        assert_some_eq!(
            climb.observe(
                changed,
                Timestamp::from_millis(1_000),
                changed_altitude,
                TrafficMotion::default()
            ),
            estimate
        );
        assert_some_eq!(
            climb.observe(
                changed,
                Timestamp::from_millis(500),
                changed_altitude,
                TrafficMotion::default()
            ),
            estimate
        );
        assert_none!(climb.observe(
            changed,
            Timestamp::from_millis(2_000),
            changed_altitude,
            TrafficMotion::default()
        ));
        assert_some!(climb.observe(
            changed,
            Timestamp::from_millis(62_000),
            Length::from_meters(960.0),
            TrafficMotion::default()
        ));
        assert_none!(climb.observe(
            changed,
            Timestamp::from_millis(122_001),
            Length::from_meters(980.0),
            TrafficMotion::default()
        ));
    }
}
