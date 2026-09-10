use crate::climb::{ClimbEma, ClimbEstimates, ClimbWindow};
use crate::ownship::SourceId;
use crate::{ExternalDeviceId, Timestamp};
use std::time::Duration;

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
        altitude: f64,
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
        let seconds = at.since_start().as_secs_f64();
        self.window.observe(seconds, altitude);
        self.estimates = self
            .ema
            .observe(seconds, altitude)
            .and_then(|normalized_ema| {
                Some(ClimbEstimates {
                    average_20s: self.window.average(20.0)?,
                    average_30s: self.window.average(30.0)?,
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
        assert_none!(climb.observe(source, Timestamp::from_millis(0), 100.0));
        let estimate = assert_some!(climb.observe(source, Timestamp::from_millis(1_000), 102.0));
        assert_some_eq!(
            climb.observe(changed, Timestamp::from_millis(1_000), 900.0),
            estimate
        );
        assert_some_eq!(
            climb.observe(changed, Timestamp::from_millis(500), 900.0),
            estimate
        );
        assert_none!(climb.observe(changed, Timestamp::from_millis(2_000), 900.0));
        assert_some!(climb.observe(changed, Timestamp::from_millis(62_000), 960.0));
        assert_none!(climb.observe(changed, Timestamp::from_millis(122_001), 980.0));
    }
}
