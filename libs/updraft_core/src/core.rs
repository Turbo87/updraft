use crate::connection::{ConnectionId, ConnectionSpec};
use crate::decoder::Decoder;
use crate::effect::Effect;
use crate::fix::Fix;
use crate::input::Input;
use crate::time::Timestamp;
use crate::topic::{Instruments, LatLon, Topic};
use std::collections::BTreeMap;
use updraft_nmea::{Message, RmcStatus};

/// Static configuration the core is built with.
///
/// `connections` is temporary. It becomes runtime-mutable through a core
/// input driven by the settings UI in milestone 5.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CoreConfig {
    pub connections: Vec<(ConnectionId, ConnectionSpec)>,
}

/// The deterministic application core.
///
/// The same ordered inputs and timestamps always produce the same
/// effects, which is what makes whole-flight scenario tests a plain loop
/// with no runtime, sleeps or wall clock.
#[derive(Debug)]
pub struct Core {
    config: CoreConfig,
    decoders: BTreeMap<ConnectionId, Decoder>,
    instruments: Instruments,
}

impl Core {
    pub fn new(config: CoreConfig) -> Self {
        let decoders = config
            .connections
            .iter()
            .map(|(id, _)| (*id, Decoder::default()))
            .collect();

        Self {
            config,
            decoders,
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
                .config
                .connections
                .iter()
                .map(|(connection, spec)| Effect::open(*connection, spec.clone()))
                .collect(),
            Input::Bytes { connection, data } => self.decode(connection, &data),
            Input::ConnectionChanged { .. } => Vec::new(),
            Input::Tick => Vec::new(),
            Input::InternalGps(fix) => self.apply_fix(fix),
        }
    }

    /// The current value of every topic, for a client that has just
    /// subscribed and holds no state yet.
    pub fn topics(&self) -> Vec<Topic> {
        vec![Topic::Instruments(self.instruments)]
    }

    fn decode(&mut self, connection: ConnectionId, data: &[u8]) -> Vec<Effect> {
        let Some(decoder) = self.decoders.get_mut(&connection) else {
            return Vec::new();
        };

        decoder.push(data);

        let mut messages = Vec::new();
        while let Some(message) = decoder.next_message() {
            messages.push(message);
        }

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
    use approx::assert_abs_diff_eq;
    use claims::{assert_some, assert_some_eq};
    use std::assert_matches;

    const RMC: &[u8] = b"$GPRMC,120000.00,A,5049.38,N,00611.16,E,45.0,270.0,010126,,,A\r\n";
    const LINK: ConnectionId = ConnectionId(1);

    fn config() -> CoreConfig {
        CoreConfig {
            connections: vec![(LINK, ConnectionSpec::tcp("127.0.0.1", 4353))],
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
        let mut core = Core::new(config());

        let effects = core.apply(Input::Start, at(0));

        assert_eq!(
            effects,
            vec![Effect::open(LINK, ConnectionSpec::tcp("127.0.0.1", 4353))]
        );
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

        let effects = core.apply(Input::bytes(ConnectionId(99), RMC), at(100));

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
        let [Topic::Instruments(instruments)] = topics.as_slice() else {
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
}
