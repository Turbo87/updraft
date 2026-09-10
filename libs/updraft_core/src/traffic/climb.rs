use crate::climb::{ClimbEma, ClimbEstimates, ClimbWindow, EnergyClimb, Velocity};
use crate::ownship::SourceId;
use crate::{ExternalDeviceId, Timestamp};
use std::time::Duration;
use updraft_units::Length;

#[derive(Debug, Default)]
pub struct TrafficClimb {
    previous: Option<(Timestamp, (ExternalDeviceId, SourceId))>,
    energy: EnergyClimb,
    window: ClimbWindow,
    ema: ClimbEma,
    estimates: Option<ClimbEstimates>,
}

impl TrafficClimb {
    pub fn expired(&self, at: Timestamp) -> bool {
        self.previous
            .is_none_or(|(previous, _)| at.saturating_since(previous) > Duration::from_secs(60))
    }

    pub fn observe(
        &mut self,
        source: (ExternalDeviceId, SourceId),
        at: Timestamp,
        altitude: Length,
        velocity: Option<(Duration, Velocity)>,
        wind: Option<Velocity>,
    ) -> Option<ClimbEstimates> {
        if let Some((previous, previous_source)) = self.previous {
            if at <= previous {
                return self.estimates;
            }
            if self.expired(at) || previous_source != source {
                *self = Self::default();
            }
        }
        self.previous = Some((at, source));
        let (time, altitude) = self
            .energy
            .observe(at.since_start(), altitude, velocity, wind)?;
        self.window.observe(time, altitude);
        self.estimates = self.ema.observe(time, altitude).and_then(|normalized_ema| {
            Some(ClimbEstimates {
                average_20s: self.window.average(Duration::from_secs(20))?,
                average_30s: self.window.average(Duration::from_secs(30))?,
                normalized_ema,
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
            Some((Duration::ZERO, fast)),
            wind
        ));
        let gain = Length::from_meters(700. / (2. * 9.80665));
        let estimates = assert_some!(climb.observe(
            source,
            Timestamp::from_millis(10_000),
            gain,
            Some((Duration::from_secs(10), slow)),
            wind
        ));
        approx::assert_abs_diff_eq!(estimates.normalized_ema, Speed::ZERO, epsilon = 1e-12);
        let changed = (ExternalDeviceId(2), SourceId::InternalGps);
        assert_none!(climb.observe(
            changed,
            Timestamp::from_millis(11_000),
            gain,
            Some((Duration::from_secs(11), fast)),
            wind
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
            None,
            None
        ));
        let estimate = assert_some!(climb.observe(
            source,
            Timestamp::from_millis(1_000),
            Length::from_meters(102.0),
            None,
            None
        ));
        assert_some_eq!(
            climb.observe(
                changed,
                Timestamp::from_millis(1_000),
                changed_altitude,
                None,
                None
            ),
            estimate
        );
        assert_some_eq!(
            climb.observe(
                changed,
                Timestamp::from_millis(500),
                changed_altitude,
                None,
                None
            ),
            estimate
        );
        assert_none!(climb.observe(
            changed,
            Timestamp::from_millis(2_000),
            changed_altitude,
            None,
            None
        ));
        assert_some!(climb.observe(
            changed,
            Timestamp::from_millis(62_000),
            Length::from_meters(960.0),
            None,
            None
        ));
        assert_none!(climb.observe(
            changed,
            Timestamp::from_millis(122_001),
            Length::from_meters(980.0),
            None,
            None
        ));
    }
}
