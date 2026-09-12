use super::reference::FlarmReference;
use super::{TrafficPositionReference, TrafficTarget};
use crate::ownship::Timed;
use crate::{ExternalDeviceId, Timestamp};
use std::time::Duration;
use updraft_geo::LatLon;
use updraft_nmea::{FlarmSource, Pflaa};
use updraft_units::{Angle, Length, MslAltitude, Speed};

/// References and timing selected together for one received FLARM report.
pub struct FlarmCorrection {
    pub position: Option<Timed<LatLon>>,
    pub altitude: Option<Timed<MslAltitude>>,
    pub position_reference: TrafficPositionReference,
    pub altitude_time: Option<Duration>,
    seconds: Option<f64>,
    horizontal: Option<(Angle, Speed)>,
    vertical: Option<Speed>,
}

impl FlarmCorrection {
    pub fn new(
        report: &Pflaa,
        source: ExternalDeviceId,
        reference: &FlarmReference,
        enabled: bool,
        at: Timestamp,
    ) -> Self {
        let applicable = enabled && matches!(report.source, None | Some(FlarmSource::Flarm));
        let horizontal = report.ground_speed.and_then(|speed| {
            let track = if speed == Speed::ZERO {
                Some(Angle::ZERO)
            } else {
                report.track
            }?;
            (speed.as_meters_per_second().is_finite()
                && speed >= Speed::ZERO
                && track.as_radians().is_finite())
            .then_some((track, speed))
        });
        let vertical = report
            .climb_rate
            .filter(|v| v.as_meters_per_second().is_finite());
        let position = (applicable && horizontal.is_some())
            .then(|| reference.position(at))
            .flatten();
        let altitude = applicable
            .then(|| vertical.and(reference.altitude(at)))
            .flatten();
        let position_reference = match (applicable, position.and(reference.epoch())) {
            (false, _) => TrafficPositionReference::Uncorrected,
            (true, None) => TrafficPositionReference::Unavailable { source },
            (true, Some(epoch)) => TrafficPositionReference::Aligned { source, epoch },
        };
        Self {
            position,
            altitude,
            position_reference,
            altitude_time: altitude
                .and(reference.epoch())
                .and_then(|epoch| u64::try_from(epoch).ok())
                .map(Duration::from_millis),
            seconds: reference
                .prediction_epoch()
                .zip(reference.epoch())
                .map(|(prediction, epoch)| (epoch - prediction) as f64 / 1_000.),
            horizontal: position.and(horizontal),
            vertical: altitude.and(vertical),
        }
    }

    pub fn align(&self, target: &mut TrafficTarget) {
        let Some(seconds) = self.seconds else { return };
        if !(-2.0..=5.0).contains(&seconds) {
            return;
        }
        if let Some((track, speed)) = self.horizontal {
            let distance = Length::from_meters(speed.as_meters_per_second() * seconds);
            target.position = target.position.destination(track, distance);
        }
        if let Some((altitude, climb)) = target.altitude_msl.zip(self.vertical) {
            let change = Length::from_meters(climb.as_meters_per_second() * seconds);
            target.altitude_msl = Some(MslAltitude::new(altitude.into_inner() + change));
        }
    }
}
