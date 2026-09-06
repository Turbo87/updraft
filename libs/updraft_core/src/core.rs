use crate::GlidePerformance;
use crate::connection::ExternalDeviceId;
use crate::effect::Effect;
use crate::external_device::{ExternalDevices, InvalidExternalDeviceOrder, UnknownExternalDevice};
use crate::fix::{Fix, UtcInstant, UtcTime};
use crate::input::{
    ActivateAirspaceDataset, AddExternalDevice, Bytes, ClearAirspaceDataset, ConnectionChanged,
    DeleteExternalDevice, EditExternalDevice, GetAirspaceSnapshot, Input, InternalGps,
    ReorderExternalDevices, SetAirspaceUnavailable, SetArrivalReserve, SetBallast, SetBugs,
    SetExternalDeviceEnabled, SetLocale, SetMacCready, SetPolar, SetUnits, Start, Tick, Update,
};
use crate::ownship::{
    DomainState, GpsCandidate, GpsSnapshot, SourceId, Timed, select_gps_candidate,
    select_pressure_altitude_candidate, select_true_airspeed_candidate,
};
use crate::sensor_fusion::{FusionInputs, SensorFusion};
use crate::settings::{Settings, SettingsSnapshot};
use crate::time::Timestamp;
use crate::topic::{Instruments, Topic};
use crate::traffic::{TrafficChanges, TrafficState, TrafficUpdate, target_from_pflaa};
use crate::{AirspaceSnapshot, AirspaceState, ReplaceAirspaceCatalog};
use std::sync::Arc;
use updraft_egm96::ellipsoidal_to_msl;
use updraft_nmea::{GgaFixQuality, Message, PositioningMode, RmcStatus};
use updraft_units::{MslAltitude, PressureAltitude, Speed};

/// The deterministic application core.
///
/// The same ordered inputs and timestamps always produce the same
/// effects, which is what makes whole-flight scenario tests a plain loop
/// with no runtime, sleeps or wall clock.
#[derive(Debug)]
pub struct Core {
    settings: Settings,
    glide_performance: GlidePerformance,
    external_devices: ExternalDevices,
    airspace: AirspaceState,
    waypoints: Arc<crate::WaypointCatalog>,
    waypoint_generation: u64,
    internal_gps: GpsCandidate,
    gps: DomainState<GpsSnapshot>,
    pressure_altitude: DomainState<PressureAltitude>,
    true_airspeed: DomainState<Speed>,
    sensor_fusion: SensorFusion,
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
        let glide_performance = GlidePerformance::default();
        let mut sensor_fusion = SensorFusion::default();
        sensor_fusion.set_polar(glide_performance.glide_polar(settings.polar));
        Self {
            settings,
            glide_performance,
            external_devices: ExternalDevices::from_device_configs(external_devices),
            airspace,
            waypoints: Arc::default(),
            waypoint_generation: 0,
            internal_gps: GpsCandidate::default(),
            gps: DomainState::Unavailable,
            pressure_altitude: DomainState::Unavailable,
            true_airspeed: DomainState::Unavailable,
            sensor_fusion,
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
            Topic::Waypoints(self.waypoints.status(self.waypoint_generation)),
            Topic::Traffic(TrafficUpdate::Snapshot(self.traffic.published_targets())),
            Topic::GlidePerformance(self.glide_performance),
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
        for (message, ingested_at) in messages {
            self.handle_message(device_id, message, ingested_at, &mut traffic_changes);
        }
        self.reevaluate_flight_data(at);

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

        self.reevaluate_flight_data(at);
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
    ) {
        match message {
            Message::Rmc(rmc)
                if rmc.status == RmcStatus::Active
                    && rmc.mode != Some(PositioningMode::NotValid) =>
            {
                let Some(device) = self.external_devices.get_mut(device_id) else {
                    return;
                };
                if let Some(time) = rmc.utc_time {
                    if let Some(date) = rmc.date {
                        if let Some(fix_time) = UtcInstant::from_nmea_date_time(date, time) {
                            device.gps.fix_time.full = Some(Timed::new(fix_time, at));
                        }
                    } else {
                        let fix_time = UtcTime::from_nmea_time(time);
                        device.gps.fix_time.time_only = Some(Timed::new(fix_time, at));
                    }
                }
                if let Some(position) = rmc.position {
                    device.gps.position = Some(Timed::new(position, at));
                }
                if let Some(course) = rmc.course_over_ground {
                    device.gps.track = Some(Timed::new(course, at));
                }
                if let Some(speed) = rmc.speed_over_ground {
                    device.gps.ground_speed = Some(Timed::new(speed, at));
                }
            }
            Message::Gga(gga) if gga.fix_quality != GgaFixQuality::Invalid => {
                let Some(device) = self.external_devices.get_mut(device_id) else {
                    return;
                };
                if let Some(time) = gga.utc_time {
                    let fix_time = UtcTime::from_nmea_time(time);
                    device.gps.fix_time.time_only = Some(Timed::new(fix_time, at));
                }
                if let Some(position) = gga.position {
                    device.gps.position = Some(Timed::new(position, at));
                }
                if let Some(altitude) = gga.altitude {
                    let altitude = MslAltitude::new(altitude);
                    device.gps.altitude = Some(Timed::new(altitude, at));
                }
            }
            Message::Pgrmz(pgrmz) => {
                let Some(altitude) = pgrmz.altitude else {
                    return;
                };
                let Some(device) = self.external_devices.get_mut(device_id) else {
                    return;
                };
                device.pressure_altitude = Some(Timed::new(PressureAltitude::new(altitude), at));
            }
            Message::Lxwp0(lxwp0) => {
                let Some(true_airspeed) = lxwp0.true_airspeed else {
                    return;
                };
                let Some(device) = self.external_devices.get_mut(device_id) else {
                    return;
                };
                if device
                    .true_airspeed
                    .is_some_and(|previous| at <= previous.ingested_at)
                {
                    return;
                }
                device.true_airspeed = Some(Timed::new(true_airspeed, at));
            }
            Message::Pflaa(pflaa) => {
                let Some(device) = self.external_devices.get(device_id) else {
                    return;
                };
                let same_device = device.gps;
                let displayed = self.displayed_gps();
                let Some(position) = same_device
                    .position
                    .map(|position| position.value)
                    .or(displayed.map(|gps| gps.position))
                else {
                    return;
                };
                let altitude = same_device
                    .altitude
                    .map(|altitude| altitude.value)
                    .or(displayed.and_then(|gps| gps.altitude_msl.map(|altitude| altitude.value)));
                let Some(target) = target_from_pflaa(&pflaa, position, altitude) else {
                    return;
                };
                self.traffic.observe(target, at, traffic_changes);
            }
            _ => {}
        }
    }

    fn reevaluate_flight_data(&mut self, at: Timestamp) {
        self.select_gps(at);
        self.select_pressure_altitude(at);
        self.select_true_airspeed(at);
        self.update_sensor_fusion();
    }

    fn update_sensor_fusion(&mut self) {
        self.sensor_fusion.update(FusionInputs {
            gps: self.gps,
            true_airspeed: self.true_airspeed,
            pressure_altitude: self.pressure_altitude,
        });
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

        match selected {
            Some(selected) => self.gps.update(selected),
            None => self.gps.mark_stale(),
        }
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

        match selected {
            Some(selected) => self.pressure_altitude.update(selected),
            None => self.pressure_altitude.mark_stale(),
        }
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

    fn select_true_airspeed(&mut self, at: Timestamp) {
        let selected = self
            .external_devices
            .iter()
            .filter(|device| device.config.enabled)
            .find_map(|device| {
                select_true_airspeed_candidate(
                    SourceId::External(device.device_id),
                    device.true_airspeed,
                    at,
                )
            });

        match selected {
            Some(selected) => self.true_airspeed.update(selected),
            None => self.true_airspeed.mark_stale(),
        }
    }

    fn select_true_airspeed_after_source_reset(&mut self, source: SourceId, at: Timestamp) {
        let selected_source_was_reset = self
            .true_airspeed
            .selected()
            .is_some_and(|selected| selected.source == source);

        self.select_true_airspeed(at);
        if selected_source_was_reset && matches!(self.true_airspeed, DomainState::LastKnown(_)) {
            self.true_airspeed = DomainState::Unavailable;
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
            true_airspeed: self.true_airspeed.published(),
            derived: self.sensor_fusion.instruments().map(Box::new),
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
    type Response = AirspaceSnapshot;

    fn apply_to(self, core: &mut Core, _at: Timestamp) -> Update<Self::Response> {
        Update::empty().with_response(core.airspace.snapshot())
    }
}

impl Input for ReplaceAirspaceCatalog {
    type Response = ();

    fn apply_to(self, core: &mut Core, _at: Timestamp) -> Update<Self::Response> {
        let status = core.airspace.replace(self.0);
        Update::effects(vec![Effect::emit(Topic::Airspace(status))])
    }
}

impl Input for Start {
    type Response = ();

    fn apply_to(self, core: &mut Core, _at: Timestamp) -> Update<Self::Response> {
        let effects = core
            .external_devices
            .iter()
            .filter(|device| device.config.enabled)
            .map(|device| Effect::open(device.device_id, device.config.spec.clone()))
            .collect();
        Update::effects(effects)
    }
}

impl Input for Tick {
    type Response = ();

    fn apply_to(self, core: &mut Core, at: Timestamp) -> Update<Self::Response> {
        let before = core.instruments();
        core.reevaluate_flight_data(at);
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

impl Input for SetPolar {
    type Response = ();

    fn apply_to(self, core: &mut Core, _: Timestamp) -> Update<()> {
        if core.settings.polar == self.polar {
            return Update::empty();
        }
        let before = core.instruments();
        core.settings.polar = self.polar;
        let polar = core.glide_performance.glide_polar(self.polar);
        core.sensor_fusion.set_polar(polar);
        let mut effects = vec![
            Effect::emit(core.settings.as_topic()),
            Effect::persist_settings(core.settings_snapshot()),
        ];
        let after = core.instruments();
        if after != before {
            effects.push(Effect::emit(after.as_topic()));
        }
        Update::effects(effects)
    }
}

impl Input for SetArrivalReserve {
    type Response = ();

    fn apply_to(self, core: &mut Core, _: Timestamp) -> Update<()> {
        if core.settings.arrival_reserve == self.reserve {
            return Update::empty();
        }
        core.settings.arrival_reserve = self.reserve;
        Update::effects(vec![
            Effect::emit(core.settings.as_topic()),
            Effect::persist_settings(core.settings_snapshot()),
        ])
    }
}

impl Input for SetMacCready {
    type Response = ();

    fn apply_to(self, core: &mut Core, _: Timestamp) -> Update<()> {
        if core.glide_performance.mac_cready == self.mac_cready {
            return Update::empty();
        }
        core.glide_performance.mac_cready = self.mac_cready;
        let topic = Topic::GlidePerformance(core.glide_performance);
        Update::effects(vec![Effect::emit(topic)])
    }
}

impl Input for SetBugs {
    type Response = ();

    fn apply_to(self, core: &mut Core, _: Timestamp) -> Update<()> {
        if core.glide_performance.bugs == self.bugs {
            return Update::empty();
        }
        let before = core.instruments();
        core.glide_performance.bugs = self.bugs;
        let polar = core.glide_performance.glide_polar(core.settings.polar);
        core.sensor_fusion.set_polar(polar);
        let topic = Topic::GlidePerformance(core.glide_performance);
        let mut effects = vec![Effect::emit(topic)];
        let after = core.instruments();
        if after != before {
            effects.push(Effect::emit(after.as_topic()));
        }
        Update::effects(effects)
    }
}

impl Input for SetBallast {
    type Response = ();

    fn apply_to(self, core: &mut Core, _: Timestamp) -> Update<()> {
        if core.glide_performance.ballast == self.ballast {
            return Update::empty();
        }
        let before = core.instruments();
        core.glide_performance.ballast = self.ballast;
        let polar = core.glide_performance.glide_polar(core.settings.polar);
        core.sensor_fusion.set_polar(polar);
        let topic = Topic::GlidePerformance(core.glide_performance);
        let mut effects = vec![Effect::emit(topic)];
        let after = core.instruments();
        if after != before {
            effects.push(Effect::emit(after.as_topic()));
        }
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
        core.select_true_airspeed_after_source_reset(SourceId::External(self.device_id), at);
        core.update_sensor_fusion();
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
        core.reevaluate_flight_data(at);

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
        core.select_true_airspeed_after_source_reset(SourceId::External(self.device_id), at);
        core.update_sensor_fusion();
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
        core.select_true_airspeed_after_source_reset(SourceId::External(self.device_id), at);
        core.update_sensor_fusion();
        let after = core.instruments();
        if after != before {
            effects.push(Effect::emit(after.as_topic()));
        }
        effects.push(Effect::emit(core.external_devices.as_topic()));
        effects.push(Effect::persist_settings(core.settings_snapshot()));
        Update::effects(effects).with_response(Ok(()))
    }
}

impl Input for crate::ReplaceWaypointCatalog {
    type Response = ();

    fn apply_to(self, core: &mut Core, _: Timestamp) -> Update<()> {
        core.waypoints = self.0;
        core.waypoint_generation += 1;
        let status = core.waypoints.status(core.waypoint_generation);
        Update::effects(vec![Effect::emit(Topic::Waypoints(status))])
    }
}

impl Input for crate::GetWaypointCatalog {
    type Response = Arc<crate::WaypointCatalog>;

    fn apply_to(self, core: &mut Core, _: Timestamp) -> Update<Self::Response> {
        Update::empty().with_response(core.waypoints.clone())
    }
}

impl Input for crate::GetWaypointSnapshot {
    type Response = crate::WaypointSnapshot;

    fn apply_to(self, core: &mut Core, _: Timestamp) -> Update<Self::Response> {
        Update::empty().with_response(crate::WaypointSnapshot {
            generation: core.waypoint_generation,
            catalog: core.waypoints.clone(),
        })
    }
}

impl Input for crate::GetGlideSnapshot {
    type Response = crate::GlideSnapshot;

    fn apply_to(self, core: &mut Core, _: Timestamp) -> Update<Self::Response> {
        Update::empty().with_response(crate::GlideSnapshot {
            waypoints: crate::WaypointSnapshot {
                generation: core.waypoint_generation,
                catalog: core.waypoints.clone(),
            },
            instruments: core.instruments(),
            polar: core.glide_performance.glide_polar(core.settings.polar),
            mac_cready: core.glide_performance.mac_cready,
            arrival_reserve: core.settings.arrival_reserve,
        })
    }
}

#[cfg(test)]
#[path = "core_tests/mod.rs"]
mod tests;
