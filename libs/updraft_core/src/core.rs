use crate::connection::{ConnectionSpec, ExternalDeviceId};
use crate::effect::Effect;
use crate::external_device::ExternalDevices;
use crate::fix::Fix;
use crate::input::Input;
use crate::settings::Settings;
use crate::time::Timestamp;
use crate::topic::{Instruments, LatLon, Topic};
use updraft_nmea::{Message, RmcStatus};

/// Static configuration the core is built with.
///
/// `connections` is temporary. It will move into runtime-mutable settings.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CoreConfig {
    pub settings: Settings,
    pub connections: Vec<(ExternalDeviceId, ConnectionSpec)>,
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
    instruments: Instruments,
}

impl Core {
    pub fn new(config: CoreConfig) -> Self {
        let CoreConfig {
            settings,
            connections,
        } = config;
        Self {
            settings,
            external_devices: ExternalDevices::from_connections(connections),
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
                .map(|device| Effect::open(device.device_id, device.spec.clone()))
                .collect(),
            Input::Bytes { device_id, data } => self.decode(device_id, &data),
            Input::ConnectionChanged { device_id, state } => {
                let Some(device) = self.external_devices.get_mut(device_id) else {
                    return Vec::new();
                };
                device.diagnostics.changed(device_id, &device.spec, state);
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
                    Effect::persist_settings(self.settings),
                ]
            }
        }
    }

    /// The current value of every topic, for a client that has just
    /// subscribed and holds no state yet.
    pub fn topics(&self) -> Vec<Topic> {
        vec![
            Topic::Instruments(self.instruments),
            Topic::Settings(self.settings),
        ]
    }

    fn decode(&mut self, device_id: ExternalDeviceId, data: &[u8]) -> Vec<Effect> {
        let messages = {
            let Some(device) = self.external_devices.get_mut(device_id) else {
                return Vec::new();
            };

            device
                .diagnostics
                .bytes(device_id, &device.spec, data.len());
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
    use crate::connection::ConnectionState;
    use crate::settings::Locale;
    use approx::assert_abs_diff_eq;
    use claims::{assert_some, assert_some_eq};
    use std::assert_matches;
    use tracing_test::traced_test;

    const RMC: &[u8] = b"$GPRMC,120000.00,A,5049.38,N,00611.16,E,45.0,270.0,010126,,,A\r\n";
    const LINK: ExternalDeviceId = ExternalDeviceId(1);
    const SPP_LINK: ExternalDeviceId = ExternalDeviceId(2);
    const TRACE_TIMESTAMP_FILTER: (&str, &str) =
        (r"\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d+)?Z", "[TIME]");

    fn config() -> CoreConfig {
        CoreConfig {
            connections: vec![(LINK, ConnectionSpec::tcp("127.0.0.1", 4353))],
            ..CoreConfig::default()
        }
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
    fn start_opens_every_configured_connection() {
        let tcp = ConnectionSpec::tcp("127.0.0.1", 4353);
        let spp = ConnectionSpec::bluetooth_spp("00:00:00:00:00:00");
        let mut core = Core::new(CoreConfig {
            connections: vec![(LINK, tcp.clone()), (SPP_LINK, spp.clone())],
            ..CoreConfig::default()
        });

        let effects = core.apply(Input::Start, at(0));

        assert_eq!(
            effects,
            vec![Effect::open(LINK, tcp), Effect::open(SPP_LINK, spp)]
        );
    }

    #[test]
    fn bytes_are_decoded_by_their_configured_device() {
        let mut core = Core::new(CoreConfig {
            connections: vec![
                (LINK, ConnectionSpec::tcp("127.0.0.1", 4353)),
                (SPP_LINK, ConnectionSpec::bluetooth_spp("00:11:22:33:44:55")),
            ],
            ..CoreConfig::default()
        });

        assert!(core.apply(Input::bytes(LINK, &RMC[..24]), at(0)).is_empty());
        assert!(
            core.apply(Input::bytes(SPP_LINK, &RMC[24..]), at(1))
                .is_empty()
        );
        let effects = core.apply(Input::bytes(LINK, &RMC[24..]), at(2));
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
        let mut core = Core::new(CoreConfig {
            connections: vec![(SPP_LINK, ConnectionSpec::bluetooth_spp("00:00:00:00:00:00"))],
            ..CoreConfig::default()
        });

        core.apply(
            Input::connection_changed(SPP_LINK, ConnectionState::Connecting),
            at(0),
        );

        assert!(logs_contain("ExternalDeviceId(2)"));
        assert!(logs_contain("00:00:00:00:00:00"));
    }

    #[test]
    #[traced_test]
    fn duplicate_device_ids_use_the_last_configured_connection() {
        let mut core = Core::new(CoreConfig {
            connections: vec![
                (LINK, ConnectionSpec::tcp("127.0.0.1", 4353)),
                (LINK, ConnectionSpec::bluetooth_spp("00:00:00:00:00:00")),
            ],
            ..CoreConfig::default()
        });

        core.apply(
            Input::connection_changed(LINK, ConnectionState::Connecting),
            at(0),
        );

        assert!(logs_contain("00:00:00:00:00:00"));
    }

    #[test]
    #[traced_test]
    fn connection_lifecycle_reports_endpoint_and_delivered_bytes() {
        let mut core = Core::new(config());

        core.apply(
            Input::connection_changed(LINK, ConnectionState::Connecting),
            at(0),
        );
        core.apply(
            Input::connection_changed(LINK, ConnectionState::Connected),
            at(1),
        );
        core.apply(Input::bytes(LINK, b"abc"), at(2));
        core.apply(Input::bytes(LINK, b"de"), at(3));
        core.apply(
            Input::connection_changed(LINK, ConnectionState::Disconnected),
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
        let mut core = Core::new(config());

        core.apply(
            Input::connection_changed(LINK, ConnectionState::Disconnected),
            at(0),
        );
        for (millis, bytes) in [(1, b"abc".as_slice()), (4, b"de".as_slice())] {
            core.apply(
                Input::connection_changed(LINK, ConnectionState::Connecting),
                at(millis),
            );
            core.apply(
                Input::connection_changed(LINK, ConnectionState::Connected),
                at(millis + 1),
            );
            core.apply(Input::bytes(LINK, bytes), at(millis + 2));
            core.apply(
                Input::connection_changed(LINK, ConnectionState::Disconnected),
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
        let mut core = Core::new(config());

        core.apply(Input::bytes(ExternalDeviceId(99), b"abc"), at(0));
        core.apply(Input::bytes(LINK, b""), at(1));

        assert!(!logs_contain("First bytes"));
    }

    #[test]
    #[traced_test]
    fn removed_connection_produces_no_further_diagnostics() {
        let mut core = Core::new(config());

        core.apply(
            Input::connection_changed(LINK, ConnectionState::Connected),
            at(0),
        );
        core.apply(Input::bytes(LINK, b"abc"), at(1));
        assert!(core.external_devices.remove(LINK).is_some());
        core.apply(
            Input::connection_changed(LINK, ConnectionState::Connecting),
            at(2),
        );
        core.apply(Input::bytes(LINK, b"de"), at(3));

        logs_assert(|lines| {
            insta::with_settings!({ filters => vec![TRACE_TIMESTAMP_FILTER] }, {
                insta::assert_snapshot!(lines.join("\n"));
            });
            Ok(())
        });
    }

    #[test]
    fn fix_emits_instruments_immediately() {
        let mut core = Core::new(config());

        let effects = core.apply(Input::bytes(LINK, RMC), at(100));

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
        let mut core = Core::new(config());
        let mut emissions = 0;

        for _ in 0..5 {
            emissions += core.apply(Input::bytes(LINK, RMC), at(100)).len();
        }

        assert_eq!(emissions, 1, "only the first sentence changed any value");
    }

    #[test]
    fn tick_emits_nothing() {
        let mut core = Core::new(config());
        core.apply(Input::bytes(LINK, RMC), at(100));

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

        let mut core = Core::new(config());

        let effects = core.apply(
            Input::bytes(
                LINK,
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
        let [Topic::Instruments(instruments), Topic::Settings(_)] = topics.as_slice() else {
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
    fn topics_include_configured_settings() {
        let settings = Settings {
            locale: Some(Locale::De),
        };
        let core = Core::new(CoreConfig {
            settings,
            ..CoreConfig::default()
        });

        assert_eq!(
            core.topics(),
            vec![
                Topic::Instruments(Instruments::default()),
                Topic::Settings(settings),
            ]
        );
    }

    #[test]
    fn setting_locale_updates_the_topic_and_requests_persistence() {
        let mut core = Core::new(CoreConfig::default());
        let settings = Settings {
            locale: Some(Locale::De),
        };

        assert_eq!(
            core.apply(Input::SetLocale(Locale::De), at(0)),
            vec![
                Effect::emit(Topic::Settings(settings)),
                Effect::persist_settings(settings),
            ]
        );
        assert_eq!(
            core.topics(),
            vec![
                Topic::Instruments(Instruments::default()),
                Topic::Settings(settings),
            ]
        );
    }

    #[test]
    fn setting_the_active_explicit_locale_is_a_no_op() {
        let mut core = Core::new(CoreConfig {
            settings: Settings {
                locale: Some(Locale::De),
            },
            ..CoreConfig::default()
        });

        assert_eq!(core.apply(Input::SetLocale(Locale::De), at(0)), vec![]);
    }
}
