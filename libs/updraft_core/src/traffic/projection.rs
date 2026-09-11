use super::TrafficTarget;
use crate::ExternalDeviceId;
use updraft_geo::LatLon;
use updraft_units::{Angle, Length, MslAltitude, Speed};

/// A decoded FLARM prediction, retained independently of the displayed position.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TrafficProjection {
    pub device_id: ExternalDeviceId,
    /// GPS milliseconds, unwrapped across midnight.
    pub epoch: i64,
    pub horizontal: Option<(LatLon, Angle, Speed)>,
    pub vertical: Option<(MslAltitude, Speed)>,
}

impl TrafficProjection {
    pub fn apply(self, target: &mut TrafficTarget, epoch: i64) {
        let seconds = (epoch - self.epoch) as f64 / 1_000.;
        if !(-2.0..=5.0).contains(&seconds) {
            return;
        }
        if let Some((position, track, speed)) = self.horizontal {
            target.position = position.destination(
                track,
                Length::from_meters(speed.as_meters_per_second() * seconds),
            );
        }
        if let Some((altitude, climb)) = self.vertical {
            let change = Length::from_meters(climb.as_meters_per_second() * seconds);
            target.altitude_msl = Some(MslAltitude::new(altitude.into_inner() + change));
        }
    }
}
