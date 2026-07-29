use crate::connection::ExternalDeviceId;
use crate::effect::Effect;
use crate::external_device::ExternalDevices;
use crate::fix::Fix;
use crate::input::Input;
use crate::settings::{Settings, SettingsSnapshot};
use crate::time::Timestamp;
use crate::topic::{Instruments, LatLon, Topic};
use updraft_nmea::{Message, RmcStatus};

/// The deterministic application core.
///
/// The same ordered inputs and timestamps always produce the same
/// effects, which is what makes whole-flight scenario tests a plain loop
/// with no runtime, sleeps or wall clock.
#[derive(Debug)]
pub struct Core {
    settings: Settings,
    external_devices: ExternalDevices,
    instruments: Instruments,
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
            instruments: Instruments::default(),
        }
    }

    /// Applies one input and returns the work it requires.
    ///
    /// `at` is supplied by the shell rather than read, which is what keeps
    /// the core deterministic.
    pub fn apply(&mut self, input: Input, at: Timestamp) -> Vec<Effect> {
        let _ = at;

        match input {
            Input::Start => self
                .external_devices
                .iter()
                .filter(|device| device.config.enabled)
                .map(|device| Effect::open(device.device_id, device.config.spec.clone()))
                .collect(),
            Input::Bytes { device_id, data } => self.decode(device_id, &data),
            Input::ConnectionChanged { device_id, state } => {
                let Some(device) = self.external_devices.get_mut(device_id) else {
                    return Vec::new();
                };
                if !device.config.enabled {
                    return Vec::new();
                }
                device
                    .diagnostics
                    .changed(device_id, &device.config.spec, state);
                Vec::new()
            }
            Input::Tick => Vec::new(),
            Input::InternalGps(fix) => self.apply_fix(fix),
            Input::SetLocale(locale) => {
                if self.settings.locale == Some(locale) {
                    return Vec::new();
                }

                self.settings.locale = Some(locale);
                vec![
                    Effect::emit(Topic::Settings(self.settings)),
                    Effect::persist_settings(self.settings_snapshot()),
                ]
            }
            Input::AddExternalDevice { spec } => {
                let device_id = self.external_devices.add(spec.clone());
                let mut effects = vec![Effect::open(device_id, spec)];
                effects.push(Effect::emit(Topic::ExternalDevices(
                    self.external_devices.published_devices(),
                )));
                effects.push(Effect::persist_settings(self.settings_snapshot()));
                effects
            }
            Input::DeleteExternalDevice(device_id) => {
                let Some(device) = self.external_devices.remove(device_id) else {
                    tracing::warn!(device_id = ?device_id, "Unknown external device");
                    return Vec::new();
                };
                let mut effects = Vec::new();
                if device.config.enabled {
                    effects.push(Effect::close(device_id));
                }
                effects.push(Effect::emit(Topic::ExternalDevices(
                    self.external_devices.published_devices(),
                )));
                effects.push(Effect::persist_settings(self.settings_snapshot()));
                effects
            }
            Input::ReorderExternalDevices(order) => {
                match self.external_devices.reorder(&order) {
                    Ok(false) => return Vec::new(),
                    Ok(true) => {}
                    Err(error) => {
                        tracing::warn!(?error, "Invalid external device order");
                        return Vec::new();
                    }
                }
                vec![
                    Effect::emit(Topic::ExternalDevices(
                        self.external_devices.published_devices(),
                    )),
                    Effect::persist_settings(self.settings_snapshot()),
                ]
            }
            Input::EditExternalDevice { device_id, spec } => {
                let Some(device) = self.external_devices.get_mut(device_id) else {
                    tracing::warn!(device_id = ?device_id, "Unknown external device");
                    return Vec::new();
                };
                if device.config.spec == spec {
                    return Vec::new();
                }
                let enabled = device.config.enabled;
                device.config.spec = spec.clone();
                device.reset_runtime();

                let mut effects = Vec::new();
                if enabled {
                    effects.push(Effect::close(device_id));
                    effects.push(Effect::open(device_id, spec));
                }
                effects.push(Effect::emit(Topic::ExternalDevices(
                    self.external_devices.published_devices(),
                )));
                effects.push(Effect::persist_settings(self.settings_snapshot()));
                effects
            }
            Input::SetExternalDeviceEnabled { device_id, enabled } => {
                let Some(device) = self.external_devices.get_mut(device_id) else {
                    tracing::warn!(device_id = ?device_id, "Unknown external device");
                    return Vec::new();
                };
                if device.config.enabled == enabled {
                    return Vec::new();
                }
                device.config.enabled = enabled;
                device.reset_runtime();
                let spec = device.config.spec.clone();

                let mut effects = if enabled {
                    vec![Effect::open(device_id, spec)]
                } else {
                    vec![Effect::close(device_id)]
                };
                effects.push(Effect::emit(Topic::ExternalDevices(
                    self.external_devices.published_devices(),
                )));
                effects.push(Effect::persist_settings(self.settings_snapshot()));
                effects
            }
        }
    }

    /// The current value of every topic, for a client that has just
    /// subscribed and holds no state yet.
    pub fn topics(&self) -> Vec<Topic> {
        vec![
            Topic::Instruments(self.instruments),
            Topic::Settings(self.settings),
            Topic::ExternalDevices(self.external_devices.published_devices()),
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

        let before = self.instruments;
        for message in messages {
            self.handle_message(message);
        }

        if self.instruments == before {
            return Vec::new();
        }

        vec![Effect::emit(Topic::Instruments(self.instruments))]
    }

    fn settings_snapshot(&self) -> SettingsSnapshot {
        SettingsSnapshot {
            settings: self.settings,
            external_devices: self.external_devices.device_configs(),
        }
    }

    fn apply_fix(&mut self, fix: Fix) -> Vec<Effect> {
        let before = self.instruments;

        self.instruments.position = Some(fix.position);
        if let Some(ellipsoidal) = fix.altitude_ellipsoid_meters {
            self.instruments.altitude_msl_meters = Some(msl_meters(fix.position, ellipsoidal));
        }
        if let Some(track) = fix.track_degrees {
            self.instruments.track_degrees = Some(track);
        }
        if let Some(speed) = fix.ground_speed_meters_per_second {
            self.instruments.ground_speed_meters_per_second = Some(speed);
        }

        if self.instruments == before {
            return Vec::new();
        }

        vec![Effect::emit(Topic::Instruments(self.instruments))]
    }

    fn handle_message(&mut self, message: Message) {
        match message {
            Message::Rmc(rmc) if rmc.status == RmcStatus::Active => {
                if let Some(position) = rmc.position {
                    self.instruments.position = Some(LatLon {
                        latitude_degrees: position.latitude().as_degrees(),
                        longitude_degrees: position.longitude().as_degrees(),
                    });
                }
                if let Some(course) = rmc.course_over_ground {
                    self.instruments.track_degrees = Some(course.as_degrees());
                }
                if let Some(speed) = rmc.speed_over_ground {
                    self.instruments.ground_speed_meters_per_second =
                        Some(speed.as_meters_per_second());
                }
            }
            Message::Gga(gga) => {
                if let Some(altitude) = gga.altitude {
                    self.instruments.altitude_msl_meters = Some(altitude.as_meters());
                }
            }
            _ => {}
        }
    }
}

/// GNSS receivers report height above the WGS84 ellipsoid. The geoid differs
/// from it by up to about 107 m, far more than any altimetry the app will do
/// can tolerate.
fn msl_meters(position: LatLon, ellipsoidal_meters: f64) -> f64 {
    let at =
        updraft_geo::LatLon::from_degrees(position.latitude_degrees, position.longitude_degrees);
    let ellipsoidal = updraft_units::EllipsoidAltitude::new(updraft_units::Length::from_meters(
        ellipsoidal_meters,
    ));

    updraft_egm96::ellipsoidal_to_msl(at, ellipsoidal)
        .into_inner()
        .as_meters()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connection::{ConnectionSpec, ConnectionState};
    use crate::external_device::ExternalDeviceConfig;
    use crate::settings::{Locale, SettingsSnapshot};
    use approx::assert_abs_diff_eq;
    use claims::{assert_some, assert_some_eq};
    use std::assert_matches;
    use tracing_test::traced_test;

    const RMC: &[u8] = b"$GPRMC,120000.00,A,5049.38,N,00611.16,E,45.0,270.0,010126,,,A\r\n";
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
            position: LatLon {
                latitude_degrees,
                longitude_degrees,
            },
            altitude_ellipsoid_meters: Some(247.0),
            track_degrees: Some(90.0),
            ground_speed_meters_per_second: Some(30.0),
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
            core.apply(Input::Start, at(0)),
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

        assert!(
            core.apply(Input::bytes(tcp_device_id, &RMC[..24]), at(0))
                .is_empty()
        );
        assert!(
            core.apply(Input::bytes(bluetooth_device_id, &RMC[24..]), at(1))
                .is_empty()
        );
        let effects = core.apply(Input::bytes(tcp_device_id, &RMC[24..]), at(2));
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

        core.apply(
            Input::connection_changed(device_id, ConnectionState::Connecting),
            at(0),
        );

        assert!(logs_contain(&format!("{device_id:?}")));
        assert!(logs_contain("00:00:00:00:00:00"));
    }

    #[test]
    #[traced_test]
    fn connection_lifecycle_reports_endpoint_and_delivered_bytes() {
        let (mut core, device_id) = core_with_external_device();

        core.apply(
            Input::connection_changed(device_id, ConnectionState::Connecting),
            at(0),
        );
        core.apply(
            Input::connection_changed(device_id, ConnectionState::Connected),
            at(1),
        );
        core.apply(Input::bytes(device_id, b"abc"), at(2));
        core.apply(Input::bytes(device_id, b"de"), at(3));
        core.apply(
            Input::connection_changed(device_id, ConnectionState::Disconnected),
            at(4),
        );

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

        core.apply(
            Input::connection_changed(device_id, ConnectionState::Disconnected),
            at(0),
        );
        for (millis, bytes) in [(1, b"abc".as_slice()), (4, b"de".as_slice())] {
            core.apply(
                Input::connection_changed(device_id, ConnectionState::Connecting),
                at(millis),
            );
            core.apply(
                Input::connection_changed(device_id, ConnectionState::Connected),
                at(millis + 1),
            );
            core.apply(Input::bytes(device_id, bytes), at(millis + 2));
            core.apply(
                Input::connection_changed(device_id, ConnectionState::Disconnected),
                at(millis + 3),
            );
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

        core.apply(Input::bytes(ExternalDeviceId(99), b"abc"), at(0));
        core.apply(Input::bytes(device_id, b""), at(1));

        assert!(!logs_contain("First bytes"));
    }

    #[test]
    #[traced_test]
    fn removed_connection_produces_no_further_diagnostics() {
        let (mut core, device_id) = core_with_external_device();

        core.apply(
            Input::connection_changed(device_id, ConnectionState::Connected),
            at(0),
        );
        core.apply(Input::bytes(device_id, b"abc"), at(1));
        assert!(core.external_devices.remove(device_id).is_some());
        core.apply(
            Input::connection_changed(device_id, ConnectionState::Connecting),
            at(2),
        );
        core.apply(Input::bytes(device_id, b"de"), at(3));

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

        let effects = core.apply(Input::bytes(device_id, RMC), at(100));

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
            emissions += core.apply(Input::bytes(device_id, RMC), at(100)).len();
        }

        assert_eq!(emissions, 1, "only the first sentence changed any value");
    }

    #[test]
    fn tick_emits_nothing() {
        let (mut core, device_id) = core_with_external_device();
        core.apply(Input::bytes(device_id, RMC), at(100));

        assert_eq!(core.apply(Input::Tick, at(200)), vec![]);
    }

    #[test]
    fn bytes_from_an_unknown_connection_are_ignored() {
        let mut core = Core::new(config());

        let effects = core.apply(Input::bytes(ExternalDeviceId(99), RMC), at(100));

        assert_eq!(effects, vec![]);
    }

    #[test]
    fn invalid_fix_does_not_publish_a_position() {
        // Fields are populated exactly as in a valid fix, so only the `V`
        // status can be what suppresses the emission.

        let (mut core, device_id) = core_with_external_device();

        let effects = core.apply(
            Input::bytes(
                device_id,
                b"$GPRMC,120000.00,V,5049.38,N,00611.16,E,45.0,270.0,010126,,,N\r\n".as_slice(),
            ),
            at(100),
        );

        assert_eq!(effects, vec![]);
    }

    #[test]
    fn internal_gps_emits_instruments_immediately() {
        let mut core = Core::new(config());

        let effects = core.apply(Input::InternalGps(fix(50.823, 6.186)), at(100));

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

        core.apply(Input::InternalGps(fix(50.823, 6.186)), at(100));

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
            emissions += core
                .apply(Input::InternalGps(fix(50.823, 6.186)), at(100))
                .len();
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

        assert_eq!(
            core.apply(Input::SetLocale(Locale::De), at(0)),
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

        assert_eq!(core.apply(Input::SetLocale(Locale::De), at(0)), vec![]);
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
            assert!(
                core.apply(Input::connection_changed(device_id, state), at(millis))
                    .is_empty()
            );
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

        assert!(core.apply(Input::bytes(device_id, RMC), at(0)).is_empty());
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

        assert!(
            core.apply(Input::bytes(first, &RMC[..24]), at(0))
                .is_empty()
        );
        core.apply(Input::ReorderExternalDevices(vec![second, first]), at(1));
        let effects = core.apply(Input::bytes(first, &RMC[24..]), at(2));

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

        core.apply(
            Input::connection_changed(first, ConnectionState::Connected),
            at(0),
        );
        core.apply(Input::bytes(first, b"abc"), at(1));
        core.apply(Input::ReorderExternalDevices(vec![second, first]), at(2));
        core.apply(
            Input::connection_changed(first, ConnectionState::Disconnected),
            at(3),
        );

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

        assert!(
            core.apply(Input::bytes(device_id, &RMC[..24]), at(0))
                .is_empty()
        );
        core.apply(
            Input::EditExternalDevice {
                device_id,
                spec: ConnectionSpec::tcp("192.0.2.1", 10110),
            },
            at(1),
        );

        assert!(
            core.apply(Input::bytes(device_id, &RMC[24..]), at(2))
                .is_empty()
        );
        assert_eq!(core.topics()[0], Topic::Instruments(Instruments::default()));
    }

    #[test]
    #[traced_test]
    fn edit_external_device_resets_diagnostics_state() {
        let (mut core, device_id) = core_with_external_device();

        core.apply(
            Input::connection_changed(device_id, ConnectionState::Connected),
            at(0),
        );
        core.apply(Input::bytes(device_id, b"abc"), at(1));
        core.apply(
            Input::EditExternalDevice {
                device_id,
                spec: ConnectionSpec::tcp("192.0.2.1", 10110),
            },
            at(2),
        );
        core.apply(
            Input::connection_changed(device_id, ConnectionState::Disconnected),
            at(3),
        );

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

        assert!(
            core.apply(Input::bytes(device_id, &RMC[..24]), at(0))
                .is_empty()
        );
        core.apply(
            Input::SetExternalDeviceEnabled {
                device_id,
                enabled: false,
            },
            at(1),
        );
        core.apply(
            Input::SetExternalDeviceEnabled {
                device_id,
                enabled: true,
            },
            at(2),
        );

        assert!(
            core.apply(Input::bytes(device_id, &RMC[24..]), at(3))
                .is_empty()
        );
        assert_eq!(core.topics()[0], Topic::Instruments(Instruments::default()));
    }

    #[test]
    #[traced_test]
    fn set_external_device_enabled_resets_diagnostics_state() {
        let (mut core, device_id) = core_with_external_device();

        core.apply(
            Input::connection_changed(device_id, ConnectionState::Connected),
            at(0),
        );
        core.apply(Input::bytes(device_id, b"abc"), at(1));
        core.apply(
            Input::SetExternalDeviceEnabled {
                device_id,
                enabled: false,
            },
            at(2),
        );
        core.apply(
            Input::SetExternalDeviceEnabled {
                device_id,
                enabled: true,
            },
            at(3),
        );
        core.apply(
            Input::connection_changed(device_id, ConnectionState::Disconnected),
            at(4),
        );

        logs_assert(|lines| {
            insta::with_settings!({ filters => vec![TRACE_TIMESTAMP_FILTER] }, {
                insta::assert_snapshot!(lines.join("\n"));
            });
            Ok(())
        });
    }

    #[test]
    #[traced_test]
    fn invalid_external_device_mutations_warn() {
        let mut core = Core::new(SettingsSnapshot::default());
        let unknown = ExternalDeviceId(u32::MAX);

        core.apply(Input::DeleteExternalDevice(unknown), at(0));
        core.apply(
            Input::EditExternalDevice {
                device_id: unknown,
                spec: ConnectionSpec::tcp("127.0.0.1", 4353),
            },
            at(1),
        );
        core.apply(
            Input::SetExternalDeviceEnabled {
                device_id: unknown,
                enabled: true,
            },
            at(2),
        );

        logs_assert(|lines| {
            insta::with_settings!({ filters => vec![TRACE_TIMESTAMP_FILTER] }, {
                insta::assert_snapshot!(lines.join("\n"));
            });
            Ok(())
        });
    }

    #[test]
    #[traced_test]
    fn reorder_external_devices_with_an_unknown_id_warns_once() {
        let mut core = Core::new(SettingsSnapshot {
            settings: Settings::default(),
            external_devices: vec![
                device_config(true, ConnectionSpec::tcp("127.0.0.1", 4353)),
                device_config(false, ConnectionSpec::bluetooth_spp("00:11:22:33:44:55")),
            ],
        });
        let first = device_id(&core, 0);

        assert!(
            core.apply(
                Input::ReorderExternalDevices(vec![first, ExternalDeviceId(u32::MAX)]),
                at(0),
            )
            .is_empty()
        );
        logs_assert(|lines| {
            if lines.len() == 1 && lines[0].contains("Invalid external device order") {
                Ok(())
            } else {
                Err(format!("expected one invalid-order warning, got {lines:?}"))
            }
        });
    }

    #[test]
    #[traced_test]
    fn reorder_external_devices_with_a_missing_id_warns_once() {
        let mut core = Core::new(SettingsSnapshot {
            settings: Settings::default(),
            external_devices: vec![
                device_config(true, ConnectionSpec::tcp("127.0.0.1", 4353)),
                device_config(false, ConnectionSpec::bluetooth_spp("00:11:22:33:44:55")),
            ],
        });
        let first = device_id(&core, 0);

        assert!(
            core.apply(Input::ReorderExternalDevices(vec![first]), at(0))
                .is_empty()
        );
        logs_assert(|lines| {
            if lines.len() == 1 && lines[0].contains("Invalid external device order") {
                Ok(())
            } else {
                Err(format!("expected one invalid-order warning, got {lines:?}"))
            }
        });
    }

    #[test]
    #[traced_test]
    fn reorder_external_devices_with_a_duplicate_id_warns_once() {
        let mut core = Core::new(SettingsSnapshot {
            settings: Settings::default(),
            external_devices: vec![
                device_config(true, ConnectionSpec::tcp("127.0.0.1", 4353)),
                device_config(false, ConnectionSpec::bluetooth_spp("00:11:22:33:44:55")),
            ],
        });
        let first = device_id(&core, 0);

        assert!(
            core.apply(Input::ReorderExternalDevices(vec![first, first]), at(0),)
                .is_empty()
        );
        logs_assert(|lines| {
            if lines.len() == 1 && lines[0].contains("Invalid external device order") {
                Ok(())
            } else {
                Err(format!("expected one invalid-order warning, got {lines:?}"))
            }
        });
    }
}
