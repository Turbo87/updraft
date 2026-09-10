use crate::climb::{ClimbEma, ClimbEstimates, ClimbWindow};
use crate::ownship::SourceId;
use crate::{ExternalDeviceId, Timestamp};
use std::time::Duration;
use updraft_units::Length;

#[derive(Debug, Default)]
pub struct TrafficClimb {
    previous: Option<(Timestamp, (ExternalDeviceId, SourceId))>,
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
        let time = at.since_start();
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
    fn ignores_old_samples_before_checking_source_and_resets_on_altitude_source_change() {
        let mut climb = TrafficClimb::default();
        let source = (ExternalDeviceId(1), SourceId::InternalGps);
        let changed = (ExternalDeviceId(1), SourceId::External(ExternalDeviceId(2)));
        let changed_altitude = Length::from_meters(900.0);
        assert_none!(climb.observe(
            source,
            Timestamp::from_millis(0),
            Length::from_meters(100.0)
        ));
        let estimate = assert_some!(climb.observe(
            source,
            Timestamp::from_millis(1_000),
            Length::from_meters(102.0)
        ));
        assert_some_eq!(
            climb.observe(changed, Timestamp::from_millis(1_000), changed_altitude),
            estimate
        );
        assert_some_eq!(
            climb.observe(changed, Timestamp::from_millis(500), changed_altitude),
            estimate
        );
        assert_none!(climb.observe(changed, Timestamp::from_millis(2_000), changed_altitude));
        assert_some!(climb.observe(
            changed,
            Timestamp::from_millis(62_000),
            Length::from_meters(960.0)
        ));
        assert_none!(climb.observe(
            changed,
            Timestamp::from_millis(122_001),
            Length::from_meters(980.0)
        ));
    }
}
