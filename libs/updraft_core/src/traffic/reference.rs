use crate::ownship::Timed;
use crate::time::Timestamp;
use std::time::Duration;
use updraft_geo::LatLon;
use updraft_nmea::{GgaFixQuality, Message, PositioningMode, RmcStatus, Time};
use updraft_units::{Length, MslAltitude};

/// Same-device GPS references for the experimental one-second FLARM cycle model.
#[derive(Debug, Default)]
pub struct FlarmReference {
    epoch: Option<i64>,
    cycle: Option<i64>,
    cycle_gps: Option<i64>,
    pending_status: bool,
    latest_fix: Option<(i64, Timed<LatLon>)>,
    active_reference: Option<Timed<LatLon>>,
    latest_altitude: Option<(i64, Timed<MslAltitude>)>,
    projected_altitude: Option<Timed<MslAltitude>>,
    active_altitude: Option<Timed<MslAltitude>>,
    updated_at: Option<Timestamp>,
}

impl FlarmReference {
    pub fn epoch(&self) -> Option<i64> {
        self.epoch
    }

    pub fn prediction_epoch(&self) -> Option<i64> {
        Some(self.cycle? + 2_000)
    }

    pub fn observe(&mut self, message: &Message, at: Timestamp) {
        if self.updated_at.is_some_and(|previous| {
            at < previous || at.saturating_since(previous) >= Duration::from_secs(3)
        }) {
            *self = Self::default();
        }
        self.updated_at = Some(at);
        match message {
            Message::Rmc(rmc) => {
                if rmc.status != RmcStatus::Active || rmc.mode == Some(PositioningMode::NotValid) {
                    *self = Self::default();
                    return;
                }
                let Some(time) = rmc.utc_time else {
                    *self = Self::default();
                    return;
                };
                let epoch = self.observe_time(time);
                self.latest_fix = None;
                if self.cycle == Some(epoch) {
                    self.active_reference = None;
                }
                let Some((position, speed)) = rmc.position.zip(rmc.speed_over_ground) else {
                    return;
                };
                let speed = speed.as_meters_per_second();
                if !speed.is_finite() || speed < 0.0 {
                    return;
                }
                let projected = if speed == 0.0 {
                    position
                } else {
                    let Some(track) = rmc.course_over_ground else {
                        return;
                    };
                    if !track.as_radians().is_finite() {
                        return;
                    }
                    position.destination(track, Length::from_meters(2.0 * speed))
                };
                let reference = Timed::new(projected, at);
                self.latest_fix = Some((epoch, reference));
                if self.cycle == Some(epoch) {
                    self.active_reference = Some(reference);
                }
            }
            Message::Gga(gga) => {
                if gga.fix_quality == GgaFixQuality::Invalid {
                    *self = Self::default();
                    return;
                }
                let epoch = gga.utc_time.map(|time| self.observe_time(time));
                let altitude = gga.altitude.filter(|value| value.as_meters().is_finite());
                if let Some((epoch, altitude)) = epoch.zip(altitude) {
                    self.observe_altitude(epoch, MslAltitude::new(altitude), at);
                } else {
                    self.latest_altitude = None;
                    self.projected_altitude = None;
                    self.active_altitude = None;
                }
            }
            Message::Pflau(_) => {
                self.start_cycle();
                self.pending_status = true;
            }
            Message::Pgrmz(_) => {
                if !self.pending_status {
                    self.start_cycle();
                }
                self.pending_status = false;
            }
            _ => {}
        }
    }

    fn observe_altitude(&mut self, epoch: i64, altitude: MslAltitude, at: Timestamp) {
        if let Some((previous_epoch, previous)) = self.latest_altitude
            && epoch == previous_epoch
            && altitude == previous.value
        {
            return;
        }
        self.projected_altitude = self.latest_altitude.and_then(|(previous_epoch, previous)| {
            let elapsed = epoch - previous_epoch;
            if !(1_000..=3_000).contains(&elapsed) || previous.fresh(at).is_none() {
                return None;
            }
            let change = altitude.into_inner() - previous.value.into_inner();
            let projected = altitude.into_inner() + change * (2_000. / elapsed as f64);
            Some(Timed::new(MslAltitude::new(projected), at))
        });
        self.latest_altitude = Some((epoch, Timed::new(altitude, at)));
        if self.cycle == Some(epoch) {
            self.active_altitude = self.projected_altitude;
        }
    }

    fn observe_time(&mut self, time: Time) -> i64 {
        let raw = i64::from(time.milliseconds_since_midnight());
        let mut epoch = raw;
        if let Some(previous) = self.epoch {
            let mut elapsed = raw - previous.rem_euclid(86_400_000);
            if elapsed < -43_200_000 {
                elapsed += 86_400_000;
            }
            if !(0..=3_000).contains(&elapsed) {
                *self = Self::default();
            } else {
                epoch = previous + elapsed;
            }
        }
        self.epoch = Some(epoch);
        // Traffic before the next marker still belongs to the preceding cycle.
        if let Some(cycle) = self.cycle {
            self.select_cycle(cycle.max(epoch - 1_000));
        }
        epoch
    }

    fn start_cycle(&mut self) {
        let Some(epoch) = self.epoch else { return };
        let cycle = match self.cycle {
            Some(cycle) if self.cycle_gps == Some(epoch) => cycle + 1_000,
            _ => epoch,
        };
        self.select_cycle(cycle);
        self.cycle_gps = Some(epoch);
    }

    fn select_cycle(&mut self, cycle: i64) {
        if self.cycle != Some(cycle) {
            self.active_reference = self
                .latest_fix
                .filter(|(epoch, _)| *epoch == cycle)
                .map(|(_, reference)| reference);
            self.active_altitude = self
                .latest_altitude
                .filter(|(epoch, _)| *epoch == cycle)
                .and(self.projected_altitude);
            self.cycle = Some(cycle);
        }
    }

    pub fn altitude(&self, at: Timestamp) -> Option<Timed<MslAltitude>> {
        self.active_altitude?.fresh(at)
    }

    pub fn position(&self, at: Timestamp) -> Option<Timed<LatLon>> {
        self.active_reference?.fresh(at)
    }
}
