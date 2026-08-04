use crate::airspace::{AirspaceDataset, AirspaceState};
use crate::connection::ExternalDeviceId;
use crate::effect::Effect;
use crate::external_device::{ExternalDevices, InvalidExternalDeviceOrder, UnknownExternalDevice};
use crate::fix::{Fix, UtcInstant, UtcTime};
use crate::input::{
    ActivateAirspaceDataset, AddExternalDevice, Bytes, ClearAirspaceDataset, ConnectionChanged,
    DeleteExternalDevice, EditExternalDevice, GetAirspaceSnapshot, Input, InternalGps,
    ReorderExternalDevices, SetAirspaceUnavailable, SetExternalDeviceEnabled, SetLocale, SetUnits,
    Start, Tick, Update,
};
use crate::ownship::{
    DomainState, GpsCandidate, GpsSnapshot, SourceId, Timed, select_gps_candidate,
    select_pressure_altitude_candidate,
};
use crate::settings::{Settings, SettingsSnapshot};
use crate::time::Timestamp;
use crate::topic::{Instruments, Topic};
use crate::traffic::{TrafficChanges, TrafficState, TrafficUpdate, target_from_pflaa};
use std::sync::Arc;
use updraft_egm96::ellipsoidal_to_msl;
use updraft_nmea::{GgaFixQuality, Message, PositioningMode, RmcStatus};
use updraft_units::{MslAltitude, PressureAltitude};

enum UpdatedDomain {
    Gps,
    PressureAltitude,
}

/// The deterministic application core.
///
/// The same ordered inputs and timestamps always produce the same
/// effects, which is what makes whole-flight scenario tests a plain loop
/// with no runtime, sleeps or wall clock.
#[derive(Debug)]
pub struct Core {
    settings: Settings,
    external_devices: ExternalDevices,
    airspace: AirspaceState,
    internal_gps: GpsCandidate,
    gps: DomainState<GpsSnapshot>,
    pressure_altitude: DomainState<PressureAltitude>,
    traffic: TrafficState,
}

impl Core {
    pub fn new(snapshot: SettingsSnapshot) -> Self {
        Self::with_airspace(snapshot, AirspaceState::none_at_startup())
    }

    /// Creates the core with an explicit process-local airspace state.
    pub fn with_airspace(snapshot: SettingsSnapshot, airspace: AirspaceState) -> Self {
        let SettingsSnapshot {
            settings,
            external_devices,
        } = snapshot;
        Self {
            settings,
            external_devices: ExternalDevices::from_device_configs(external_devices),
            airspace,
            internal_gps: GpsCandidate::default(),
            gps: DomainState::Unavailable,
            pressure_altitude: DomainState::Unavailable,
            traffic: TrafficState::default(),
        }
    }

    /// Applies one input and returns the work it requires.
    ///
    /// `at` is supplied by the shell rather than read, which is what keeps
    /// the core deterministic.
    pub fn apply<I: Input>(&mut self, input: I, at: Timestamp) -> Update<I::Response> {
        input.apply_to(self, at)
    }

    /// The current value of every topic, for a client that has just
    /// subscribed and holds no state yet.
    pub fn topics(&self) -> Vec<Topic> {
        vec![
            self.instruments().as_topic(),
            self.settings.as_topic(),
            self.external_devices.as_topic(),
            Topic::Airspace(self.airspace.status()),
            Topic::Traffic(TrafficUpdate::Snapshot(self.traffic.published_targets())),
        ]
    }

    fn decode(&mut self, device_id: ExternalDeviceId, data: &[u8], at: Timestamp) -> Vec<Effect> {
        let messages = {
            let Some(device) = self.external_devices.get_mut(device_id) else {
                return Vec::new();
            };
            if !device.config.enabled {
                return Vec::new();
            }

            device
                .diagnostics
                .bytes(device_id, &device.config.spec, data.len());
            device.decoder.push(data, at);

            let mut messages = Vec::new();
            while let Some((message, ingested_at)) = device.decoder.next_message() {
                messages.push((message, ingested_at));
            }
            messages
        };

        let before = self.instruments();
        let mut traffic_changes = TrafficChanges::default();
        let mut gps_updated = false;
        let mut pressure_altitude_updated = false;
        for (message, ingested_at) in messages {
            match self.handle_message(device_id, message, ingested_at, &mut traffic_changes) {
                Some(UpdatedDomain::Gps) => gps_updated = true,
                Some(UpdatedDomain::PressureAltitude) => pressure_altitude_updated = true,
                None => {}
            }
        }
        if gps_updated {
            self.select_gps(at);
        }
        if pressure_altitude_updated {
            self.select_pressure_altitude(at);
        }

        let mut effects = Vec::new();
        let after = self.instruments();
        if after != before {
            effects.push(Effect::emit(after.as_topic()));
        }
        if let Some(delta) = traffic_changes.into_delta() {
            effects.push(Effect::emit(Topic::Traffic(TrafficUpdate::Delta(delta))));
        }

        effects
    }

    fn settings_snapshot(&self) -> SettingsSnapshot {
        SettingsSnapshot {
            settings: self.settings,
            external_devices: self.external_devices.device_configs(),
        }
    }

    fn apply_fix(&mut self, fix: Fix, at: Timestamp) -> Vec<Effect> {
        let before = self.instruments();

        self.internal_gps.position = Some(Timed::new(fix.position, at));
        if let Some(altitude) = fix.altitude_ellipsoid {
            let altitude = ellipsoidal_to_msl(fix.position, altitude);
            self.internal_gps.altitude = Some(Timed::new(altitude, at));
        }
        if let Some(track) = fix.track {
            self.internal_gps.track = Some(Timed::new(track, at));
        }
        if let Some(speed) = fix.ground_speed {
            self.internal_gps.ground_speed = Some(Timed::new(speed, at));
        }
        if let Some(fix_time) = fix.fix_time {
            self.internal_gps.fix_time.full = Some(Timed::new(fix_time, at));
        }

        self.select_gps(at);
        let after = self.instruments();
        if after == before {
            return Vec::new();
        }

        vec![Effect::emit(after.as_topic())]
    }

    fn handle_message(
        &mut self,
        device_id: ExternalDeviceId,
        message: Message,
        at: Timestamp,
        traffic_changes: &mut TrafficChanges,
    ) -> Option<UpdatedDomain> {
        match message {
            Message::Rmc(rmc)
                if rmc.status == RmcStatus::Active
                    && rmc.mode != Some(PositioningMode::NotValid) =>
            {
                let device = self.external_devices.get_mut(device_id)?;
                let mut updated = false;
                if let Some(time) = rmc.utc_time {
                    if let Some(date) = rmc.date {
                        if let Some(fix_time) = UtcInstant::from_nmea_date_time(date, time) {
                            device.gps.fix_time.full = Some(Timed::new(fix_time, at));
                            updated = true;
                        }
                    } else {
                        let fix_time = UtcTime::from_nmea_time(time);
                        device.gps.fix_time.time_only = Some(Timed::new(fix_time, at));
                        updated = true;
                    }
                }
                if let Some(position) = rmc.position {
                    device.gps.position = Some(Timed::new(position, at));
                    updated = true;
                }
                if let Some(course) = rmc.course_over_ground {
                    device.gps.track = Some(Timed::new(course, at));
                    updated = true;
                }
                if let Some(speed) = rmc.speed_over_ground {
                    device.gps.ground_speed = Some(Timed::new(speed, at));
                    updated = true;
                }
                updated.then_some(UpdatedDomain::Gps)
            }
            Message::Gga(gga) if gga.fix_quality != GgaFixQuality::Invalid => {
                let device = self.external_devices.get_mut(device_id)?;
                let mut updated = false;
                if let Some(time) = gga.utc_time {
                    let fix_time = UtcTime::from_nmea_time(time);
                    device.gps.fix_time.time_only = Some(Timed::new(fix_time, at));
                    updated = true;
                }
                if let Some(position) = gga.position {
                    device.gps.position = Some(Timed::new(position, at));
                    updated = true;
                }
                if let Some(altitude) = gga.altitude {
                    let altitude = MslAltitude::new(altitude);
                    device.gps.altitude = Some(Timed::new(altitude, at));
                    updated = true;
                }
                updated.then_some(UpdatedDomain::Gps)
            }
            Message::Pgrmz(pgrmz) => {
                let altitude = pgrmz.altitude?;
                let device = self.external_devices.get_mut(device_id)?;
                device.pressure_altitude = Some(Timed::new(PressureAltitude::new(altitude), at));
                Some(UpdatedDomain::PressureAltitude)
            }
            Message::Pflaa(pflaa) => {
                let device = self.external_devices.get(device_id)?;
                let same_device = device.gps;
                let displayed = self.displayed_gps();
                let position = same_device
                    .position
                    .map(|position| position.value)
                    .or(displayed.map(|gps| gps.position))?;
                let altitude = same_device
                    .altitude
                    .map(|altitude| altitude.value)
                    .or(displayed.and_then(|gps| gps.altitude_msl));
                let target = target_from_pflaa(&pflaa, position, altitude)?;
                self.traffic.observe(target, at, traffic_changes);
                None
            }
            _ => None,
        }
    }

    fn select_gps(&mut self, at: Timestamp) {
        let selected = self
            .external_devices
            .iter()
            .filter(|device| device.config.enabled)
            .find_map(|device| {
                select_gps_candidate(SourceId::External(device.device_id), device.gps, at)
            })
            .or_else(|| select_gps_candidate(SourceId::InternalGps, self.internal_gps, at));

        self.gps = match selected {
            Some(selected) => DomainState::Current(selected),
            None => match self.gps {
                DomainState::Unavailable => DomainState::Unavailable,
                DomainState::Current(selected) | DomainState::LastKnown(selected) => {
                    DomainState::LastKnown(selected)
                }
            },
        };
    }

    fn select_pressure_altitude(&mut self, at: Timestamp) {
        let selected = self
            .external_devices
            .iter()
            .filter(|device| device.config.enabled)
            .find_map(|device| {
                select_pressure_altitude_candidate(
                    SourceId::External(device.device_id),
                    device.pressure_altitude,
                    at,
                )
            });

        self.pressure_altitude = match selected {
            Some(selected) => DomainState::Current(selected),
            None => match self.pressure_altitude {
                DomainState::Unavailable => DomainState::Unavailable,
                DomainState::Current(selected) | DomainState::LastKnown(selected) => {
                    DomainState::LastKnown(selected)
                }
            },
        };
    }

    fn select_pressure_altitude_after_source_reset(&mut self, source: SourceId, at: Timestamp) {
        let selected_source_was_reset = self
            .pressure_altitude
            .selected()
            .is_some_and(|selected| selected.source == source);

        self.select_pressure_altitude(at);
        if selected_source_was_reset && matches!(self.pressure_altitude, DomainState::LastKnown(_))
        {
            self.pressure_altitude = DomainState::Unavailable;
        }
    }

    fn select_gps_after_source_reset(&mut self, source: SourceId, at: Timestamp) {
        let selected_source_was_reset = self
            .gps
            .selected()
            .is_some_and(|selected| selected.source == source);

        self.select_gps(at);
        if selected_source_was_reset && matches!(self.gps, DomainState::LastKnown(_)) {
            self.gps = DomainState::Unavailable;
        }
    }

    fn displayed_gps(&self) -> Option<GpsSnapshot> {
        self.gps.selected().map(|selected| selected.value)
    }

    fn instruments(&self) -> Instruments {
        Instruments {
            gps: self.gps.published(),
            pressure_altitude: self.pressure_altitude.published(),
        }
    }
}

impl Input for ActivateAirspaceDataset {
    type Response = ();

    fn apply_to(self, core: &mut Core, _at: Timestamp) -> Update<Self::Response> {
        let status = core.airspace.activate(self.dataset, self.source_name);
        Update::effects(vec![Effect::emit(Topic::Airspace(status))])
    }
}

impl Input for ClearAirspaceDataset {
    type Response = ();

    fn apply_to(self, core: &mut Core, _at: Timestamp) -> Update<Self::Response> {
        let status = core.airspace.clear();
        Update::effects(vec![Effect::emit(Topic::Airspace(status))])
    }
}

impl Input for SetAirspaceUnavailable {
    type Response = ();

    fn apply_to(self, core: &mut Core, _at: Timestamp) -> Update<Self::Response> {
        let status = core.airspace.mark_unavailable(self.source_name, self.error);
        Update::effects(vec![Effect::emit(Topic::Airspace(status))])
    }
}

impl Input for GetAirspaceSnapshot {
    type Response = Option<Arc<AirspaceDataset>>;

    fn apply_to(self, core: &mut Core, _at: Timestamp) -> Update<Self::Response> {
        Update::empty().with_response(core.airspace.snapshot())
    }
}

impl Input for Start {
    type Response = ();

    fn apply_to(self, core: &mut Core, _at: Timestamp) -> Update<Self::Response> {
        Update::effects(
            core.external_devices
                .iter()
                .filter(|device| device.config.enabled)
                .map(|device| Effect::open(device.device_id, device.config.spec.clone()))
                .collect(),
        )
    }
}

impl Input for Tick {
    type Response = ();

    fn apply_to(self, core: &mut Core, at: Timestamp) -> Update<Self::Response> {
        let before = core.instruments();
        core.select_gps(at);
        core.select_pressure_altitude(at);
        let after = core.instruments();

        let mut effects = Vec::new();
        if after != before {
            effects.push(Effect::emit(after.as_topic()));
        }
        let changes = core.traffic.expire(at);
        if let Some(delta) = changes.into_delta() {
            effects.push(Effect::emit(Topic::Traffic(TrafficUpdate::Delta(delta))));
        }
        Update::effects(effects)
    }
}

impl Input for Bytes {
    type Response = ();

    fn apply_to(self, core: &mut Core, at: Timestamp) -> Update<Self::Response> {
        Update::effects(core.decode(self.device_id, &self.data, at))
    }
}

impl Input for ConnectionChanged {
    type Response = ();

    fn apply_to(self, core: &mut Core, _at: Timestamp) -> Update<Self::Response> {
        let Some(device) = core.external_devices.get_mut(self.device_id) else {
            return Update::empty();
        };
        if !device.config.enabled {
            return Update::empty();
        }
        device
            .diagnostics
            .changed(self.device_id, &device.config.spec, self.state);
        Update::empty()
    }
}

impl Input for InternalGps {
    type Response = ();

    fn apply_to(self, core: &mut Core, at: Timestamp) -> Update<Self::Response> {
        Update::effects(core.apply_fix(self.fix, at))
    }
}

impl Input for SetLocale {
    type Response = ();

    fn apply_to(self, core: &mut Core, _at: Timestamp) -> Update<Self::Response> {
        let effects = if core.settings.locale == Some(self.locale) {
            Vec::new()
        } else {
            core.settings.locale = Some(self.locale);
            vec![
                Effect::emit(core.settings.as_topic()),
                Effect::persist_settings(core.settings_snapshot()),
            ]
        };
        Update::effects(effects)
    }
}

impl Input for SetUnits {
    type Response = ();

    fn apply_to(self, core: &mut Core, _at: Timestamp) -> Update<Self::Response> {
        let effects = if core.settings.units == self.units {
            Vec::new()
        } else {
            core.settings.units = self.units;
            vec![
                Effect::emit(core.settings.as_topic()),
                Effect::persist_settings(core.settings_snapshot()),
            ]
        };
        Update::effects(effects)
    }
}

impl Input for AddExternalDevice {
    type Response = ExternalDeviceId;

    fn apply_to(self, core: &mut Core, _at: Timestamp) -> Update<Self::Response> {
        let device_id = core.external_devices.add(self.spec.clone());
        let mut effects = vec![Effect::open(device_id, self.spec)];
        effects.push(Effect::emit(core.external_devices.as_topic()));
        effects.push(Effect::persist_settings(core.settings_snapshot()));
        Update::effects(effects).with_response(device_id)
    }
}

impl Input for DeleteExternalDevice {
    type Response = Result<(), UnknownExternalDevice>;

    fn apply_to(self, core: &mut Core, at: Timestamp) -> Update<Self::Response> {
        let before = core.instruments();
        let Some(device) = core.external_devices.remove(self.device_id) else {
            return Update::empty().with_response(Err(UnknownExternalDevice {
                device_id: self.device_id,
            }));
        };
        let mut effects = Vec::new();
        if device.config.enabled {
            effects.push(Effect::close(self.device_id));
        }
        core.select_gps_after_source_reset(SourceId::External(self.device_id), at);
        core.select_pressure_altitude_after_source_reset(SourceId::External(self.device_id), at);
        let after = core.instruments();
        if after != before {
            effects.push(Effect::emit(after.as_topic()));
        }
        effects.push(Effect::emit(core.external_devices.as_topic()));
        effects.push(Effect::persist_settings(core.settings_snapshot()));
        Update::effects(effects).with_response(Ok(()))
    }
}

impl Input for ReorderExternalDevices {
    type Response = Result<(), InvalidExternalDeviceOrder>;

    fn apply_to(self, core: &mut Core, at: Timestamp) -> Update<Self::Response> {
        let before = core.instruments();
        match core.external_devices.reorder(&self.order) {
            Ok(false) => return Update::empty().with_response(Ok(())),
            Ok(true) => {}
            Err(error) => return Update::empty().with_response(Err(error)),
        }
        core.select_gps(at);
        core.select_pressure_altitude(at);

        let mut effects = Vec::new();
        let after = core.instruments();
        if after != before {
            effects.push(Effect::emit(after.as_topic()));
        }
        effects.push(Effect::emit(core.external_devices.as_topic()));
        effects.push(Effect::persist_settings(core.settings_snapshot()));
        Update::effects(effects).with_response(Ok(()))
    }
}

impl Input for EditExternalDevice {
    type Response = Result<(), UnknownExternalDevice>;

    fn apply_to(self, core: &mut Core, at: Timestamp) -> Update<Self::Response> {
        let before = core.instruments();
        let Some(device) = core.external_devices.get_mut(self.device_id) else {
            return Update::empty().with_response(Err(UnknownExternalDevice {
                device_id: self.device_id,
            }));
        };
        if device.config.spec == self.spec {
            return Update::empty().with_response(Ok(()));
        }
        let enabled = device.config.enabled;
        device.config.spec = self.spec.clone();
        device.reset_runtime();

        let mut effects = Vec::new();
        if enabled {
            effects.push(Effect::close(self.device_id));
            effects.push(Effect::open(self.device_id, self.spec));
        }
        core.select_gps_after_source_reset(SourceId::External(self.device_id), at);
        core.select_pressure_altitude_after_source_reset(SourceId::External(self.device_id), at);
        let after = core.instruments();
        if after != before {
            effects.push(Effect::emit(after.as_topic()));
        }
        effects.push(Effect::emit(core.external_devices.as_topic()));
        effects.push(Effect::persist_settings(core.settings_snapshot()));
        Update::effects(effects).with_response(Ok(()))
    }
}

impl Input for SetExternalDeviceEnabled {
    type Response = Result<(), UnknownExternalDevice>;

    fn apply_to(self, core: &mut Core, at: Timestamp) -> Update<Self::Response> {
        let before = core.instruments();
        let Some(device) = core.external_devices.get_mut(self.device_id) else {
            return Update::empty().with_response(Err(UnknownExternalDevice {
                device_id: self.device_id,
            }));
        };
        if device.config.enabled == self.enabled {
            return Update::empty().with_response(Ok(()));
        }
        device.config.enabled = self.enabled;
        device.reset_runtime();
        let spec = device.config.spec.clone();

        let mut effects = if self.enabled {
            vec![Effect::open(self.device_id, spec)]
        } else {
            vec![Effect::close(self.device_id)]
        };
        core.select_gps_after_source_reset(SourceId::External(self.device_id), at);
        core.select_pressure_altitude_after_source_reset(SourceId::External(self.device_id), at);
        let after = core.instruments();
        if after != before {
            effects.push(Effect::emit(after.as_topic()));
        }
        effects.push(Effect::emit(core.external_devices.as_topic()));
        effects.push(Effect::persist_settings(core.settings_snapshot()));
        Update::effects(effects).with_response(Ok(()))
    }
}

#[cfg(test)]
#[path = "core_tests/mod.rs"]
mod tests;
