use crate::connection::ExternalDeviceId;
use crate::effect::Effect;
use crate::external_device::{ExternalDevices, InvalidExternalDeviceOrder, UnknownExternalDevice};
use crate::fix::Fix;
use crate::input::{
    AddExternalDevice, Bytes, ConnectionChanged, DeleteExternalDevice, EditExternalDevice, Input,
    InternalGps, ReorderExternalDevices, SetExternalDeviceEnabled, SetLocale, Start, Tick, Update,
};
use crate::ownship::OwnshipState;
use crate::settings::{Settings, SettingsSnapshot};
use crate::time::Timestamp;
use crate::topic::Topic;
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
    ownship: OwnshipState,
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
            ownship: OwnshipState::default(),
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
        ]
    }

    fn decode(&mut self, device_id: ExternalDeviceId, data: &[u8]) -> Vec<Effect> {
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
        for message in messages {
            self.handle_message(device_id, message);
        }

        if self.ownship == before {
            return Vec::new();
        }

        vec![Effect::emit(self.ownship.published().as_topic())]
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

    fn handle_message(&mut self, device_id: ExternalDeviceId, message: Message) {
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
            _ => {}
        }
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

    fn apply_to(self, _core: &mut Core, _at: Timestamp) -> Update<Self::Response> {
        Update::empty()
    }
}

impl Input for Bytes {
    type Response = ();

    fn apply_to(self, core: &mut Core, _at: Timestamp) -> Update<Self::Response> {
        Update::effects(core.decode(self.device_id, &self.data))
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
mod tests {
    use super::*;
    use crate::connection::{ConnectionSpec, ConnectionState};
    use crate::external_device::ExternalDeviceConfig;
    use crate::settings::{Locale, SettingsSnapshot};
    use crate::topic::{Instruments, LatLon as TopicLatLon};
    use approx::assert_abs_diff_eq;
    use claims::{assert_some, assert_some_eq};
    use std::assert_matches;
    use tracing_test::traced_test;
    use updraft_geo::LatLon;
    use updraft_units::{Angle, EllipsoidAltitude, Length, MslAltitude, Speed};

    const RMC: &[u8] = b"$GPRMC,120000.00,A,5049.38,N,00611.16,E,45.0,270.0,010126,,,A\r\n";
    const RMC_SECOND_DEVICE: &[u8] =
        b"$GPRMC,120000.00,A,5100.00,N,00700.00,E,40.0,180.0,010126,,,A\r\n";
    const GGA: &[u8] = b"$GPGGA,120000.00,5049.38,N,00611.16,E,1,08,0.9,200.0,M,0.0,M,,\r\n";
    const GGA_SECOND_DEVICE: &[u8] =
        b"$GPGGA,120000.00,5100.00,N,00700.00,E,1,08,0.9,300.0,M,0.0,M,,\r\n";
    const TRACE_TIMESTAMP_FILTER: (&str, &str) =
        (r"\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d+)?Z", "[TIME]");

    fn device_config(enabled: bool, spec: ConnectionSpec) -> ExternalDeviceConfig {
        ExternalDeviceConfig { enabled, spec }
    }

    fn config() -> SettingsSnapshot {
        SettingsSnapshot {
            settings: Settings::default(),
            external_devices: vec![device_config(true, ConnectionSpec::tcp("127.0.0.1", 4353))],
        }
    }

    fn device_id(core: &Core, index: usize) -> ExternalDeviceId {
        let topics = core.topics();
        let Some(Topic::ExternalDevices(devices)) = topics.last() else {
            panic!("the configured external devices topic should be published");
        };
        devices[index].device_id
    }

    fn core_with_external_device() -> (Core, ExternalDeviceId) {
        let core = Core::new(config());
        let device_id = device_id(&core, 0);
        (core, device_id)
    }

    fn at(millis: u64) -> Timestamp {
        Timestamp::from_millis(millis)
    }

    fn fix(latitude_degrees: f64, longitude_degrees: f64) -> Fix {
        Fix {
            position: LatLon::from_degrees(latitude_degrees, longitude_degrees),
            altitude_ellipsoid: Some(EllipsoidAltitude::new(Length::from_meters(247.0))),
            track: Some(Angle::from_degrees(90.0)),
            ground_speed: Some(Speed::from_meters_per_second(30.0)),
        }
    }

    #[test]
    fn start_opens_only_enabled_external_devices() {
        let tcp = ConnectionSpec::tcp("127.0.0.1", 4353);
        let bluetooth = ConnectionSpec::bluetooth_spp("00:11:22:33:44:55");
        let mut core = Core::new(SettingsSnapshot {
            settings: Settings {
                locale: Some(Locale::De),
            },
            external_devices: vec![
                ExternalDeviceConfig {
                    enabled: false,
                    spec: tcp.clone(),
                },
                ExternalDeviceConfig {
                    enabled: true,
                    spec: bluetooth.clone(),
                },
            ],
        });

        let topics = core.topics();
        let Some(Topic::ExternalDevices(devices)) = topics.last() else {
            panic!("the configured external devices topic should be published");
        };
        assert_eq!(devices.len(), 2);
        assert_eq!(
            (devices[0].config.enabled, &devices[0].config.spec),
            (false, &tcp)
        );
        assert_eq!(
            (devices[1].config.enabled, &devices[1].config.spec),
            (true, &bluetooth),
        );
        assert_ne!(devices[0].device_id, devices[1].device_id);

        let enabled_device_id = devices[1].device_id;
        assert_eq!(
            core.apply(Start, at(0)).effects,
            vec![Effect::open(enabled_device_id, bluetooth)]
        );
    }

    #[test]
    fn bytes_are_decoded_by_their_configured_device() {
        let mut core = Core::new(SettingsSnapshot {
            settings: Settings::default(),
            external_devices: vec![
                device_config(true, ConnectionSpec::tcp("127.0.0.1", 4353)),
                device_config(true, ConnectionSpec::bluetooth_spp("00:11:22:33:44:55")),
            ],
        });
        let tcp_device_id = device_id(&core, 0);
        let bluetooth_device_id = device_id(&core, 1);

        let input = Bytes::new(tcp_device_id, &RMC[..24]);
        assert!(core.apply(input, at(0)).effects.is_empty());

        let input = Bytes::new(bluetooth_device_id, &RMC[24..]);
        assert!(core.apply(input, at(1)).effects.is_empty());

        let input = Bytes::new(tcp_device_id, &RMC[24..]);
        let effects = core.apply(input, at(2)).effects;
        let [Effect::Emit(Topic::Instruments(instruments))] = effects.as_slice() else {
            panic!("the completed sentence should emit instruments");
        };
        let position = instruments.position.expect("RMC position");
        assert_abs_diff_eq!(position.latitude_degrees, 50.823, epsilon = 1e-3);
        assert_abs_diff_eq!(position.longitude_degrees, 6.186, epsilon = 1e-3);
    }

    #[test]
    #[traced_test]
    fn spp_lifecycle_reports_the_mac_address() {
        let mut core = Core::new(SettingsSnapshot {
            settings: Settings::default(),
            external_devices: vec![device_config(
                true,
                ConnectionSpec::bluetooth_spp("00:00:00:00:00:00"),
            )],
        });
        let device_id = device_id(&core, 0);

        let input = ConnectionChanged::new(device_id, ConnectionState::Connecting);
        core.apply(input, at(0));

        assert!(logs_contain(&format!("{device_id:?}")));
        assert!(logs_contain("00:00:00:00:00:00"));
    }

    #[test]
    #[traced_test]
    fn connection_lifecycle_reports_endpoint_and_delivered_bytes() {
        let (mut core, device_id) = core_with_external_device();

        let input = ConnectionChanged::new(device_id, ConnectionState::Connecting);
        core.apply(input, at(0));

        let input = ConnectionChanged::new(device_id, ConnectionState::Connected);
        core.apply(input, at(1));

        core.apply(Bytes::new(device_id, b"abc"), at(2));
        core.apply(Bytes::new(device_id, b"de"), at(3));

        let input = ConnectionChanged::new(device_id, ConnectionState::Disconnected);
        core.apply(input, at(4));

        logs_assert(|lines| {
            insta::with_settings!({ filters => vec![TRACE_TIMESTAMP_FILTER] }, {
                insta::assert_snapshot!(lines.join("\n"));
            });
            Ok(())
        });
    }

    #[test]
    #[traced_test]
    fn failed_attempt_is_debug_and_counters_reset_on_reconnect() {
        let (mut core, device_id) = core_with_external_device();

        let input = ConnectionChanged::new(device_id, ConnectionState::Disconnected);
        core.apply(input, at(0));

        for (millis, bytes) in [(1, b"abc".as_slice()), (4, b"de".as_slice())] {
            let input = ConnectionChanged::new(device_id, ConnectionState::Connecting);
            core.apply(input, at(millis));

            let input = ConnectionChanged::new(device_id, ConnectionState::Connected);
            core.apply(input, at(millis + 1));

            core.apply(Bytes::new(device_id, bytes), at(millis + 2));

            let input = ConnectionChanged::new(device_id, ConnectionState::Disconnected);
            core.apply(input, at(millis + 3));
        }

        logs_assert(|lines| {
            insta::with_settings!({ filters => vec![TRACE_TIMESTAMP_FILTER] }, {
                insta::assert_snapshot!(lines.join("\n"));
            });
            Ok(())
        });
    }

    #[test]
    #[traced_test]
    fn unknown_and_empty_bytes_produce_no_delivery_log() {
        let (mut core, device_id) = core_with_external_device();

        core.apply(Bytes::new(ExternalDeviceId(99), b"abc"), at(0));
        core.apply(Bytes::new(device_id, b""), at(1));

        assert!(!logs_contain("First bytes"));
    }

    #[test]
    #[traced_test]
    fn removed_connection_produces_no_further_diagnostics() {
        let (mut core, device_id) = core_with_external_device();

        let input = ConnectionChanged::new(device_id, ConnectionState::Connected);
        core.apply(input, at(0));

        core.apply(Bytes::new(device_id, b"abc"), at(1));
        assert!(core.external_devices.remove(device_id).is_some());

        let input = ConnectionChanged::new(device_id, ConnectionState::Connecting);
        core.apply(input, at(2));

        core.apply(Bytes::new(device_id, b"de"), at(3));

        logs_assert(|lines| {
            insta::with_settings!({ filters => vec![TRACE_TIMESTAMP_FILTER] }, {
                insta::assert_snapshot!(lines.join("\n"));
            });
            Ok(())
        });
    }

    #[test]
    fn fix_emits_instruments_immediately() {
        let (mut core, device_id) = core_with_external_device();

        let effects = core.apply(Bytes::new(device_id, RMC), at(100)).effects;

        assert_matches!(effects.as_slice(), [Effect::Emit(Topic::Instruments(_))]);
        let [Effect::Emit(Topic::Instruments(instruments))] = effects.as_slice() else {
            unreachable!()
        };
        let position = assert_some!(instruments.position);
        assert_abs_diff_eq!(position.latitude_degrees, 50.823, epsilon = 1e-3);
        assert_abs_diff_eq!(position.longitude_degrees, 6.186, epsilon = 1e-3);
        assert_some_eq!(instruments.track_degrees, 270.0);
    }

    #[test]
    fn repeated_identical_sentences_emit_only_once() {
        let (mut core, device_id) = core_with_external_device();
        let mut emissions = 0;

        for _ in 0..5 {
            let input = Bytes::new(device_id, RMC);
            emissions += core.apply(input, at(100)).effects.len();
        }

        assert_eq!(emissions, 1, "only the first sentence changed any value");
    }

    #[test]
    fn external_devices_keep_their_ownship_values() {
        let mut core = Core::new(SettingsSnapshot {
            settings: Settings::default(),
            external_devices: vec![
                device_config(true, ConnectionSpec::tcp("127.0.0.1", 4353)),
                device_config(true, ConnectionSpec::tcp("127.0.0.1", 4354)),
            ],
        });
        let first_device_id = device_id(&core, 0);
        let second_device_id = device_id(&core, 1);

        core.apply(Bytes::new(first_device_id, RMC), at(0));
        core.apply(Bytes::new(first_device_id, GGA), at(1));
        core.apply(Bytes::new(second_device_id, RMC_SECOND_DEVICE), at(2));
        core.apply(Bytes::new(second_device_id, GGA_SECOND_DEVICE), at(3));

        let first = core
            .external_devices
            .iter()
            .next()
            .expect("the first configured external device");
        let first_position = assert_some!(first.ownship.position);
        assert_abs_diff_eq!(
            first_position.latitude().as_degrees(),
            50.823,
            epsilon = 1e-3
        );
        assert_abs_diff_eq!(
            first_position.longitude().as_degrees(),
            6.186,
            epsilon = 1e-3
        );
        assert_some_eq!(
            first.ownship.altitude_msl,
            MslAltitude::new(Length::from_meters(200.0))
        );

        let second = core
            .external_devices
            .iter()
            .nth(1)
            .expect("the second configured external device");
        let second_position = assert_some!(second.ownship.position);
        assert_abs_diff_eq!(
            second_position.latitude().as_degrees(),
            51.0,
            epsilon = 1e-3
        );
        assert_abs_diff_eq!(
            second_position.longitude().as_degrees(),
            7.0,
            epsilon = 1e-3
        );
        assert_some_eq!(
            second.ownship.altitude_msl,
            MslAltitude::new(Length::from_meters(300.0))
        );

        let topics = core.topics();
        let [Topic::Instruments(instruments), ..] = topics.as_slice() else {
            unreachable!()
        };
        assert_some_eq!(
            instruments.position,
            TopicLatLon {
                latitude_degrees: 51.0,
                longitude_degrees: 7.0,
            }
        );
        assert_some_eq!(instruments.altitude_msl_meters, 300.0);
    }

    #[test]
    fn tick_emits_nothing() {
        let (mut core, device_id) = core_with_external_device();
        core.apply(Bytes::new(device_id, RMC), at(100));

        assert_eq!(core.apply(Tick, at(200)).effects, vec![]);
    }

    #[test]
    fn bytes_from_an_unknown_connection_are_ignored() {
        let mut core = Core::new(config());

        let input = Bytes::new(ExternalDeviceId(99), RMC);
        let effects = core.apply(input, at(100)).effects;

        assert_eq!(effects, vec![]);
    }

    #[test]
    fn invalid_fix_does_not_publish_a_position() {
        // Fields are populated exactly as in a valid fix, so only the `V`
        // status can be what suppresses the emission.

        let (mut core, device_id) = core_with_external_device();

        let input = Bytes::new(
            device_id,
            b"$GPRMC,120000.00,V,5049.38,N,00611.16,E,45.0,270.0,010126,,,N\r\n".as_slice(),
        );
        let effects = core.apply(input, at(100)).effects;

        assert_eq!(effects, vec![]);
    }

    #[test]
    fn internal_gps_emits_instruments_immediately() {
        let mut core = Core::new(config());

        let input = InternalGps::new(fix(50.823, 6.186));
        let effects = core.apply(input, at(100)).effects;

        assert_matches!(effects.as_slice(), [Effect::Emit(Topic::Instruments(_))]);
        let [Effect::Emit(Topic::Instruments(instruments))] = effects.as_slice() else {
            unreachable!()
        };
        let position = assert_some!(instruments.position);
        assert_abs_diff_eq!(position.latitude_degrees, 50.823, epsilon = 1e-9);
        assert_some_eq!(instruments.track_degrees, 90.0);
    }

    #[test]
    fn internal_gps_altitude_is_converted_to_msl() {
        let mut core = Core::new(config());

        let input = InternalGps::new(fix(50.823, 6.186));
        core.apply(input, at(100));

        let topics = core.topics();
        let [
            Topic::Instruments(instruments),
            Topic::Settings(_),
            Topic::ExternalDevices(_),
        ] = topics.as_slice()
        else {
            unreachable!()
        };
        // The geoid sits 46.54 m above the ellipsoid at this position, so the
        // 247 m the fix carries lands here. Pinned to the centimetre: a change
        // in what the pilot reads as altitude is a change worth seeing.
        assert_abs_diff_eq!(
            assert_some!(instruments.altitude_msl_meters),
            200.46,
            epsilon = 0.01
        );
    }

    #[test]
    fn repeated_identical_fixes_emit_only_once() {
        let mut core = Core::new(config());
        let mut emissions = 0;

        for _ in 0..5 {
            let input = InternalGps::new(fix(50.823, 6.186));
            emissions += core.apply(input, at(100)).effects.len();
        }

        assert_eq!(emissions, 1, "only the first fix changed any value");
    }

    #[test]
    fn topics_include_settings_and_external_devices() {
        let settings = Settings {
            locale: Some(Locale::De),
        };
        let tcp = ConnectionSpec::tcp("127.0.0.1", 4353);
        let bluetooth = ConnectionSpec::bluetooth_spp("00:11:22:33:44:55");
        let core = Core::new(SettingsSnapshot {
            settings,
            external_devices: vec![
                ExternalDeviceConfig {
                    enabled: true,
                    spec: tcp.clone(),
                },
                ExternalDeviceConfig {
                    enabled: false,
                    spec: bluetooth.clone(),
                },
            ],
        });

        let topics = core.topics();
        assert_eq!(topics.len(), 3);
        assert_eq!(topics[0], Topic::Instruments(Instruments::default()));
        assert_eq!(topics[1], Topic::Settings(settings));
        let Topic::ExternalDevices(devices) = &topics[2] else {
            panic!("the configured external devices topic should be published");
        };
        assert_eq!(devices.len(), 2);
        assert_eq!(
            (devices[0].config.enabled, &devices[0].config.spec),
            (true, &tcp)
        );
        assert_eq!(
            (devices[1].config.enabled, &devices[1].config.spec),
            (false, &bluetooth),
        );
        assert_ne!(devices[0].device_id, devices[1].device_id);
    }

    #[test]
    fn setting_locale_updates_the_topic_and_requests_persistence() {
        let mut core = Core::new(SettingsSnapshot::default());
        let settings = Settings {
            locale: Some(Locale::De),
        };
        let snapshot = SettingsSnapshot {
            settings,
            external_devices: Vec::new(),
        };

        let input = SetLocale::new(Locale::De);
        assert_eq!(
            core.apply(input, at(0)).effects,
            vec![
                Effect::emit(Topic::Settings(settings)),
                Effect::persist_settings(snapshot),
            ]
        );
        assert_eq!(
            core.topics(),
            vec![
                Topic::Instruments(Instruments::default()),
                Topic::Settings(settings),
                Topic::ExternalDevices(Vec::new()),
            ]
        );
    }

    #[test]
    fn setting_the_active_explicit_locale_is_a_no_op() {
        let mut core = Core::new(SettingsSnapshot {
            settings: Settings {
                locale: Some(Locale::De),
            },
            external_devices: Vec::new(),
        });

        let input = SetLocale::new(Locale::De);
        assert_eq!(core.apply(input, at(0)).effects, vec![]);
    }

    #[test]
    #[traced_test]
    fn disabled_external_device_ignores_lifecycle_events() {
        let mut core = Core::new(SettingsSnapshot {
            settings: Settings::default(),
            external_devices: vec![device_config(false, ConnectionSpec::tcp("127.0.0.1", 4353))],
        });
        let device_id = device_id(&core, 0);

        for (millis, state) in [
            (0, ConnectionState::Connecting),
            (1, ConnectionState::Connected),
            (2, ConnectionState::Disconnected),
        ] {
            let input = ConnectionChanged::new(device_id, state);
            assert!(core.apply(input, at(millis)).effects.is_empty());
        }

        logs_assert(|lines| {
            if lines.is_empty() {
                Ok(())
            } else {
                Err(format!("disabled device produced diagnostics: {lines:?}"))
            }
        });
    }

    #[test]
    fn disabled_external_device_ignores_bytes() {
        let mut core = Core::new(SettingsSnapshot {
            settings: Settings::default(),
            external_devices: vec![device_config(false, ConnectionSpec::tcp("127.0.0.1", 4353))],
        });
        let device_id = device_id(&core, 0);

        let input = Bytes::new(device_id, RMC);
        assert!(core.apply(input, at(0)).effects.is_empty());
        assert_eq!(core.topics()[0], Topic::Instruments(Instruments::default()));
    }

    #[test]
    fn reorder_external_devices_preserves_partial_decoder_state() {
        let mut core = Core::new(SettingsSnapshot {
            settings: Settings::default(),
            external_devices: vec![
                device_config(true, ConnectionSpec::tcp("127.0.0.1", 4353)),
                device_config(true, ConnectionSpec::bluetooth_spp("00:11:22:33:44:55")),
            ],
        });
        let first = device_id(&core, 0);
        let second = device_id(&core, 1);

        let input = Bytes::new(first, &RMC[..24]);
        assert!(core.apply(input, at(0)).effects.is_empty());

        let input = ReorderExternalDevices::new(vec![second, first]);
        core.apply(input, at(1));

        let input = Bytes::new(first, &RMC[24..]);
        let effects = core.apply(input, at(2)).effects;

        assert_matches!(effects.as_slice(), [Effect::Emit(Topic::Instruments(_))]);
    }

    #[test]
    #[traced_test]
    fn reorder_external_devices_preserves_diagnostics_state() {
        let mut core = Core::new(SettingsSnapshot {
            settings: Settings::default(),
            external_devices: vec![
                device_config(true, ConnectionSpec::tcp("127.0.0.1", 4353)),
                device_config(true, ConnectionSpec::bluetooth_spp("00:11:22:33:44:55")),
            ],
        });
        let first = device_id(&core, 0);
        let second = device_id(&core, 1);

        let input = ConnectionChanged::new(first, ConnectionState::Connected);
        core.apply(input, at(0));

        core.apply(Bytes::new(first, b"abc"), at(1));

        let input = ReorderExternalDevices::new(vec![second, first]);
        core.apply(input, at(2));

        let input = ConnectionChanged::new(first, ConnectionState::Disconnected);
        core.apply(input, at(3));

        logs_assert(|lines| {
            insta::with_settings!({ filters => vec![TRACE_TIMESTAMP_FILTER] }, {
                insta::assert_snapshot!(lines.join("\n"));
            });
            Ok(())
        });
    }

    #[test]
    fn edit_external_device_resets_partial_decoder_state() {
        let (mut core, device_id) = core_with_external_device();

        let input = Bytes::new(device_id, &RMC[..24]);
        assert!(core.apply(input, at(0)).effects.is_empty());

        let input = EditExternalDevice::new(device_id, ConnectionSpec::tcp("192.0.2.1", 10110));
        core.apply(input, at(1));

        let input = Bytes::new(device_id, &RMC[24..]);
        assert!(core.apply(input, at(2)).effects.is_empty());
        assert_eq!(core.topics()[0], Topic::Instruments(Instruments::default()));
    }

    #[test]
    fn edit_external_device_resets_ownship_state() {
        let (mut core, device_id) = core_with_external_device();

        core.apply(Bytes::new(device_id, RMC), at(0));
        core.apply(Bytes::new(device_id, GGA), at(1));

        let input = EditExternalDevice::new(device_id, ConnectionSpec::tcp("192.0.2.1", 10110));
        core.apply(input, at(2));

        let device = core
            .external_devices
            .iter()
            .find(|device| device.device_id == device_id)
            .expect("the edited external device");
        assert_eq!(device.ownship, OwnshipState::default());
    }

    #[test]
    #[traced_test]
    fn edit_external_device_resets_diagnostics_state() {
        let (mut core, device_id) = core_with_external_device();

        let input = ConnectionChanged::new(device_id, ConnectionState::Connected);
        core.apply(input, at(0));

        core.apply(Bytes::new(device_id, b"abc"), at(1));

        let input = EditExternalDevice::new(device_id, ConnectionSpec::tcp("192.0.2.1", 10110));
        core.apply(input, at(2));

        let input = ConnectionChanged::new(device_id, ConnectionState::Disconnected);
        core.apply(input, at(3));

        logs_assert(|lines| {
            insta::with_settings!({ filters => vec![TRACE_TIMESTAMP_FILTER] }, {
                insta::assert_snapshot!(lines.join("\n"));
            });
            Ok(())
        });
    }

    #[test]
    fn set_external_device_enabled_resets_partial_decoder_state() {
        let (mut core, device_id) = core_with_external_device();

        let input = Bytes::new(device_id, &RMC[..24]);
        assert!(core.apply(input, at(0)).effects.is_empty());

        core.apply(SetExternalDeviceEnabled::disabled(device_id), at(1));
        core.apply(SetExternalDeviceEnabled::enabled(device_id), at(2));

        let input = Bytes::new(device_id, &RMC[24..]);
        assert!(core.apply(input, at(3)).effects.is_empty());
        assert_eq!(core.topics()[0], Topic::Instruments(Instruments::default()));
    }

    #[test]
    #[traced_test]
    fn set_external_device_enabled_resets_diagnostics_state() {
        let (mut core, device_id) = core_with_external_device();

        let input = ConnectionChanged::new(device_id, ConnectionState::Connected);
        core.apply(input, at(0));

        core.apply(Bytes::new(device_id, b"abc"), at(1));
        core.apply(SetExternalDeviceEnabled::disabled(device_id), at(2));
        core.apply(SetExternalDeviceEnabled::enabled(device_id), at(3));

        let input = ConnectionChanged::new(device_id, ConnectionState::Disconnected);
        core.apply(input, at(4));

        logs_assert(|lines| {
            insta::with_settings!({ filters => vec![TRACE_TIMESTAMP_FILTER] }, {
                insta::assert_snapshot!(lines.join("\n"));
            });
            Ok(())
        });
    }
}
