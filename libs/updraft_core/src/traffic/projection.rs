use super::TrafficTarget;
use updraft_units::{Angle, Length, MslAltitude, Speed};

/// Aligns a received prediction with the current GPS epoch.
pub fn align_target(
    target: &mut TrafficTarget,
    seconds: f64,
    horizontal: Option<(Angle, Speed)>,
    vertical: Option<Speed>,
) {
    if !(-2.0..=5.0).contains(&seconds) {
        return;
    }
    if let Some((track, speed)) = horizontal {
        let distance = Length::from_meters(speed.as_meters_per_second() * seconds);
        target.position = target.position.destination(track, distance);
    }
    if let Some((altitude, climb)) = target.altitude_msl.zip(vertical) {
        let change = Length::from_meters(climb.as_meters_per_second() * seconds);
        target.altitude_msl = Some(MslAltitude::new(altitude.into_inner() + change));
    }
}
