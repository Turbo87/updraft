use crate::airspace::{AirspaceDataset, AirspaceState};
use crate::connection::ExternalDeviceId;
use crate::effect::Effect;
use crate::external_device::{ExternalDevices, InvalidExternalDeviceOrder, UnknownExternalDevice};
use crate::fix::Fix;
use crate::input::{
    ActivateAirspaceDataset, AddExternalDevice, Bytes, ClearAirspaceDataset, ConnectionChanged,
    DeleteExternalDevice, EditExternalDevice, GetAirspaceSnapshot, Input, InternalGps,
    ReorderExternalDevices, SetAirspaceUnavailable, SetExternalDeviceEnabled, SetLocale, SetUnits,
    Start, Tick, Update,
};
use crate::ownship::OwnshipState;
use crate::settings::{Settings, SettingsSnapshot};
use crate::time::Timestamp;
use crate::topic::Topic;
use crate::traffic::{TrafficChanges, TrafficState, TrafficUpdate, target_from_pflaa};
use std::sync::Arc;
use updraft_egm96::ellipsoidal_to_msl;
use updraft_nmea::{Message, RmcStatus};
use updraft_units::MslAltitude;

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
    ownship: OwnshipState,
    traffic: TrafficState,
}

impl Core {
    pub fn new(snapshot: SettingsSnapshot) -> Self {
        let SettingsSnapshot {
            settings,
            external_devices,
        } = snapshot;
        Self {
            settings,
            external_devices: ExternalDevices::from_device_configs(external_devices),
            airspace: AirspaceState::default(),
            ownship: OwnshipState::default(),
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
            self.ownship.published().as_topic(),
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
            device.decoder.push(data);

            let mut messages = Vec::new();
            while let Some(message) = device.decoder.next_message() {
                messages.push(message);
            }
            messages
        };

        let before = self.ownship;
        let mut traffic_changes = TrafficChanges::default();
        for message in messages {
            self.handle_message(device_id, message, at, &mut traffic_changes);
        }

        let mut effects = Vec::new();
        if self.ownship != before {
            effects.push(Effect::emit(self.ownship.published().as_topic()));
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

    fn apply_fix(&mut self, fix: Fix) -> Vec<Effect> {
        let before = self.ownship;

        self.ownship.position = Some(fix.position);
        if let Some(altitude) = fix.altitude_ellipsoid {
            self.ownship.altitude_msl = Some(ellipsoidal_to_msl(fix.position, altitude));
        }
        if let Some(track) = fix.track {
            self.ownship.track = Some(track);
        }
        if let Some(speed) = fix.ground_speed {
            self.ownship.ground_speed = Some(speed);
        }

        if self.ownship == before {
            return Vec::new();
        }

        vec![Effect::emit(self.ownship.published().as_topic())]
    }

    fn handle_message(
        &mut self,
        device_id: ExternalDeviceId,
        message: Message,
        at: Timestamp,
        traffic_changes: &mut TrafficChanges,
    ) {
        match message {
            Message::Rmc(rmc) if rmc.status == RmcStatus::Active => {
                let Some(device) = self.external_devices.get_mut(device_id) else {
                    return;
                };
                if let Some(position) = rmc.position {
                    device.ownship.position = Some(position);
                    self.ownship.position = Some(position);
                }
                if let Some(course) = rmc.course_over_ground {
                    device.ownship.track = Some(course);
                    self.ownship.track = Some(course);
                }
                if let Some(speed) = rmc.speed_over_ground {
                    device.ownship.ground_speed = Some(speed);
                    self.ownship.ground_speed = Some(speed);
                }
            }
            Message::Gga(gga) => {
                if let Some(altitude) = gga.altitude {
                    let Some(device) = self.external_devices.get_mut(device_id) else {
                        return;
                    };
                    let altitude = MslAltitude::new(altitude);
                    device.ownship.altitude_msl = Some(altitude);
                    self.ownship.altitude_msl = Some(altitude);
                }
            }
            Message::Pflaa(pflaa) => {
                let Some(device) = self.external_devices.get(device_id) else {
                    return;
                };
                let same_device = device.ownship;
                let Some(position) = same_device.position.or(self.ownship.position) else {
                    return;
                };
                let altitude = same_device.altitude_msl.or(self.ownship.altitude_msl);
                let Some(target) = target_from_pflaa(&pflaa, position, altitude) else {
                    return;
                };
                self.traffic.observe(target, at, traffic_changes);
            }
            _ => {}
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
        let changes = core.traffic.expire(at);
        Update::effects(
            changes
                .into_delta()
                .map(|delta| Effect::emit(Topic::Traffic(TrafficUpdate::Delta(delta))))
                .into_iter()
                .collect(),
        )
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

    fn apply_to(self, core: &mut Core, _at: Timestamp) -> Update<Self::Response> {
        Update::effects(core.apply_fix(self.fix))
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

    fn apply_to(self, core: &mut Core, _at: Timestamp) -> Update<Self::Response> {
        let Some(device) = core.external_devices.remove(self.device_id) else {
            return Update::empty().with_response(Err(UnknownExternalDevice {
                device_id: self.device_id,
            }));
        };
        let mut effects = Vec::new();
        if device.config.enabled {
            effects.push(Effect::close(self.device_id));
        }
        effects.push(Effect::emit(core.external_devices.as_topic()));
        effects.push(Effect::persist_settings(core.settings_snapshot()));
        Update::effects(effects).with_response(Ok(()))
    }
}

impl Input for ReorderExternalDevices {
    type Response = Result<(), InvalidExternalDeviceOrder>;

    fn apply_to(self, core: &mut Core, _at: Timestamp) -> Update<Self::Response> {
        match core.external_devices.reorder(&self.order) {
            Ok(false) => return Update::empty().with_response(Ok(())),
            Ok(true) => {}
            Err(error) => return Update::empty().with_response(Err(error)),
        }
        Update::effects(vec![
            Effect::emit(core.external_devices.as_topic()),
            Effect::persist_settings(core.settings_snapshot()),
        ])
        .with_response(Ok(()))
    }
}

impl Input for EditExternalDevice {
    type Response = Result<(), UnknownExternalDevice>;

    fn apply_to(self, core: &mut Core, _at: Timestamp) -> Update<Self::Response> {
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
        effects.push(Effect::emit(core.external_devices.as_topic()));
        effects.push(Effect::persist_settings(core.settings_snapshot()));
        Update::effects(effects).with_response(Ok(()))
    }
}

impl Input for SetExternalDeviceEnabled {
    type Response = Result<(), UnknownExternalDevice>;

    fn apply_to(self, core: &mut Core, _at: Timestamp) -> Update<Self::Response> {
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
        effects.push(Effect::emit(core.external_devices.as_topic()));
        effects.push(Effect::persist_settings(core.settings_snapshot()));
        Update::effects(effects).with_response(Ok(()))
    }
}

#[cfg(test)]
#[path = "core_tests/mod.rs"]
mod tests;
