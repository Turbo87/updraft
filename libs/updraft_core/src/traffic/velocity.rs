use crate::Timestamp;
use crate::climb::Velocity;
use crate::ownship::SourceId;
use std::time::Duration;
use updraft_geo::LatLon;

#[derive(Debug, Default)]
pub struct TrafficVelocity {
    previous: Option<(Timestamp, SourceId, LatLon)>,
}

impl TrafficVelocity {
    pub fn observe(
        &mut self,
        at: Timestamp,
        position: Option<(SourceId, LatLon)>,
        reported: Option<Velocity>,
    ) -> Option<(Duration, Velocity)> {
        if self.previous.is_some_and(|(previous, _, _)| at <= previous) {
            return None;
        }
        let previous = self.previous;
        self.previous = position.map(|(source, point)| (at, source, point));
        if let Some(reported) = reported {
            return Some((at.since_start(), reported));
        }
        let (previous_at, previous_source, previous_position) = previous?;
        let (source, position) = position?;
        let interval = at.saturating_since(previous_at);
        if source != previous_source || interval > Duration::from_secs(5) {
            return None;
        }
        let (distance, track) = previous_position.distance_bearing(position);
        if !distance.as_meters().is_finite() || !track.as_radians().is_finite() {
            return None;
        }
        let velocity = Velocity::from_track(track, distance / interval);
        Some((previous_at.since_start() + interval / 2, velocity))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_abs_diff_eq;
    use claims::{assert_none, assert_some};
    use updraft_units::{Angle, Length, Speed};

    #[test]
    fn invalid_coordinates_do_not_poison_recovered_velocity() {
        let mut velocity = TrafficVelocity::default();
        let source = SourceId::InternalGps;
        let valid = LatLon::from_degrees(50., 6.);
        let invalid = LatLon::from_degrees(95., 6.);
        for (millis, position) in [(0, valid), (1000, invalid), (2000, valid)] {
            assert_none!(velocity.observe(
                Timestamp::from_millis(millis),
                Some((source, position)),
                None
            ));
        }
        let (_, recovered) = assert_some!(velocity.observe(
            Timestamp::from_millis(3000),
            Some((source, valid)),
            None
        ));
        assert_eq!(recovered.north, Speed::ZERO);
        assert_eq!(recovered.east, Speed::ZERO);
    }

    #[test]
    fn derives_midpoint_velocity_with_a_five_second_limit_and_source_continuity() {
        let mut velocity = TrafficVelocity::default();
        let start = LatLon::from_degrees(50., 6.);
        let end = start.destination(Angle::ZERO, Length::from_meters(150.));
        let source = SourceId::InternalGps;
        assert_none!(velocity.observe(Timestamp::from_millis(0), Some((source, start)), None));
        let (time, derived) =
            assert_some!(velocity.observe(Timestamp::from_millis(5000), Some((source, end)), None));
        assert_eq!(time, Duration::from_millis(2500));
        assert_abs_diff_eq!(
            derived.north,
            Speed::from_meters_per_second(30.),
            epsilon = 1e-8
        );
        assert_none!(velocity.observe(Timestamp::from_millis(10_001), Some((source, start)), None));
        let changed = SourceId::External(crate::ExternalDeviceId(1));
        assert_none!(velocity.observe(Timestamp::from_millis(11_000), Some((changed, end)), None));
        assert_none!(velocity.observe(Timestamp::from_millis(12_000), None, None));
        assert_none!(velocity.observe(Timestamp::from_millis(13_000), Some((changed, end)), None));
    }

    #[test]
    fn prefers_reported_velocity_and_ignores_old_positions() {
        let mut velocity = TrafficVelocity::default();
        let position = Some((SourceId::InternalGps, LatLon::from_degrees(50., 6.)));
        let reported = Velocity::from_track(Angle::ZERO, Speed::from_meters_per_second(40.));
        let (_, current) =
            assert_some!(velocity.observe(Timestamp::from_millis(1000), position, Some(reported)));
        assert_eq!(current.north, reported.north);
        assert_none!(velocity.observe(Timestamp::from_millis(1000), position, None));
        assert_none!(velocity.observe(Timestamp::from_millis(500), position, None));
        let (_, derived) =
            assert_some!(velocity.observe(Timestamp::from_millis(2000), position, None));
        assert_eq!(derived.north, Speed::ZERO);
    }
}
