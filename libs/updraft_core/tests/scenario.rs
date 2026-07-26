use updraft_core::{
    ConnectionId, ConnectionSpec, Core, CoreConfig, Effect, Input, Timestamp, Topic,
};

const LINK: ConnectionId = ConnectionId(1);
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
        Effect::OpenConnection { connection, spec } => format!("open {connection:?} {spec:?}"),
        Effect::CloseConnection { connection } => format!("close {connection:?}"),
    }
}

/// Replays `sentences` through a fresh core and returns the whole effect
/// stream, rendered.
fn replay(sentences: &str) -> Vec<String> {
    let mut core = Core::new(CoreConfig {
        connections: vec![(LINK, ConnectionSpec::tcp("127.0.0.1", 4353))],
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
