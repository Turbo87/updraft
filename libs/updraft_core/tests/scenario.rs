use updraft_core::{
    ConnectionSpec, Core, CoreConfig, Effect, ExternalDeviceId, Fix, Input, LatLon, Timestamp,
    Topic,
};

const LINK: ExternalDeviceId = ExternalDeviceId(1);
const FIXTURE: &str = include_str!("../../../testdata/nmea/basic.nmea");
/// Sentences the core must not act on: a verbatim repeat of the last line
/// of `basic.nmea`, then a `V`-status fix carrying plausible values.
const IGNORED: &str = include_str!("../../../testdata/nmea/ignored.nmea");

/// Rounds to a quantity-appropriate precision so snapshots record real
/// behaviour changes and not last-bit float differences.
fn describe(effect: &Effect) -> String {
    fn number(value: Option<f64>, decimals: usize) -> String {
        value.map_or_else(|| "none".to_owned(), |v| format!("{v:.decimals$}"))
    }

    match effect {
        Effect::Emit(Topic::Instruments(instruments)) => {
            let position = instruments.position.map_or_else(
                || "none".to_owned(),
                |p| format!("{:.5},{:.5}", p.latitude_degrees, p.longitude_degrees),
            );

            format!(
                "instruments pos={position} track={} gs={} alt={}",
                number(instruments.track_degrees, 2),
                number(instruments.ground_speed_meters_per_second, 2),
                number(instruments.altitude_msl_meters, 1),
            )
        }
        Effect::Emit(Topic::Settings(settings)) => format!("settings {settings:?}"),
        Effect::OpenConnection { device_id, spec } => format!("open {device_id:?} {spec:?}"),
        Effect::CloseConnection { device_id } => format!("close {device_id:?}"),
        Effect::PersistSettings(settings) => format!("persist settings {settings:?}"),
    }
}

/// Replays `sentences` through a fresh core and returns the whole effect
/// stream, rendered.
fn replay(sentences: &str) -> Vec<String> {
    let mut core = Core::new(CoreConfig {
        connections: vec![(LINK, ConnectionSpec::tcp("127.0.0.1", 4353))],
        ..CoreConfig::default()
    });

    let mut log: Vec<String> = core
        .apply(Input::Start, Timestamp::from_millis(0))
        .iter()
        .map(describe)
        .collect();

    for (index, line) in sentences.lines().enumerate() {
        let at = Timestamp::from_millis(1_000 + index as u64 * 1_000);
        let sentence = format!("{line}\r\n");
        log.extend(
            core.apply(Input::bytes(LINK, sentence.into_bytes()), at)
                .iter()
                .map(describe),
        );
    }

    log
}

#[test]
fn replaying_a_flight_produces_a_stable_effect_stream() {
    insta::assert_snapshot!(replay(FIXTURE).join("\n"));
}

#[test]
fn same_inputs_produce_same_effects() {
    assert_eq!(replay(FIXTURE), replay(FIXTURE));
}

/// Pins that neither guard can be removed without the snapshot noticing:
/// a repeated sentence must not re-emit, and a `V`-status fix must not be
/// applied at all.
#[test]
fn sentences_the_core_ignores_produce_no_effects() {
    let combined = format!("{FIXTURE}{IGNORED}");
    let with_ignored = replay(&combined);

    assert_eq!(
        with_ignored,
        replay(FIXTURE),
        "the ignored sentences changed the effect stream"
    );
}

/// A GNSS fix and the equivalent NMEA sentence must leave the core in the
/// same state, or the two position sources disagree about what the aircraft
/// is doing.
#[test]
fn gnss_fix_and_equivalent_sentence_agree() {
    let mut from_sentence = Core::new(CoreConfig {
        connections: vec![(LINK, ConnectionSpec::tcp("127.0.0.1", 4353))],
        ..CoreConfig::default()
    });
    let effects = from_sentence.apply(
        Input::bytes(
            LINK,
            b"$GPRMC,120000.00,A,5049.38,N,00611.16,E,45.0,270.0,010126,,,A\r\n".as_slice(),
        ),
        Timestamp::from_millis(0),
    );

    let mut from_fix = Core::new(CoreConfig::default());
    let equivalent = from_fix.apply(
        Input::InternalGps(Fix {
            position: LatLon {
                latitude_degrees: 50.823,
                longitude_degrees: 6.186,
            },
            // RMC carries no altitude, so neither may this fix.
            altitude_ellipsoid_meters: None,
            track_degrees: Some(270.0),
            ground_speed_meters_per_second: Some(45.0 * 1852.0 / 3600.0),
        }),
        Timestamp::from_millis(0),
    );

    let rendered = |effects: &[Effect]| effects.iter().map(describe).collect::<Vec<_>>();
    assert_eq!(rendered(&effects), rendered(&equivalent));
}
