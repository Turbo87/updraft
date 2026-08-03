use super::support::{core_with_external_device, describe};
use updraft_core::{Bytes, Core, Effect, Fix, InternalGps, SettingsSnapshot, Start, Timestamp};
use updraft_geo::LatLon;
use updraft_units::{Angle, Speed};

const FIXTURE: &str = include_str!("../../../../testdata/nmea/basic.nmea");
/// A valid refresh followed by a `V`-status fix with plausible values.
const REFRESH_THEN_INVALID: &str = include_str!("../../../../testdata/nmea/ignored.nmea");

/// Replays `sentences` through a fresh core and returns the whole effect
/// stream, rendered.
fn replay(sentences: &str) -> Vec<String> {
    let (mut core, device_id) = core_with_external_device();

    let mut log: Vec<String> = core
        .apply(Start, Timestamp::from_millis(0))
        .effects
        .iter()
        .map(describe)
        .collect();

    for (index, line) in sentences.lines().enumerate() {
        let at = Timestamp::from_millis(1_000 + index as u64 * 1_000);
        let sentence = format!("{line}\r\n");
        let input = Bytes::new(device_id, sentence.into_bytes());
        log.extend(core.apply(input, at).effects.iter().map(describe));
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

#[test]
fn invalid_sentence_after_a_valid_refresh_produces_no_effects() {
    let (valid_refresh, _) = REFRESH_THEN_INVALID
        .split_once('\n')
        .expect("the fixture should contain two sentences");
    let without_invalid = format!("{FIXTURE}{valid_refresh}\n");
    let combined = format!("{FIXTURE}{REFRESH_THEN_INVALID}");

    assert_eq!(
        replay(&combined),
        replay(&without_invalid),
        "the invalid sentence changed the effect stream"
    );
}

/// A GNSS fix and the equivalent NMEA sentence must leave the core in the
/// same state, or the two position sources disagree about what the aircraft
/// is doing.
#[test]
fn gnss_fix_and_equivalent_sentence_agree() {
    let (mut from_sentence, device_id) = core_with_external_device();
    let sentence = b"$GPRMC,120000.00,A,5049.38,N,00611.16,E,45.0,270.0,010126,,,A\r\n".as_slice();
    let input = Bytes::new(device_id, sentence);
    let effects = from_sentence
        .apply(input, Timestamp::from_millis(0))
        .effects;

    let mut from_fix = Core::new(SettingsSnapshot::default());
    let fix = Fix {
        position: LatLon::from_degrees(50.823, 6.186),
        // RMC carries no altitude, so neither may this fix.
        altitude_ellipsoid: None,
        track: Some(Angle::from_degrees(270.0)),
        ground_speed: Some(Speed::from_meters_per_second(45.0 * 1852.0 / 3600.0)),
    };
    let input = InternalGps::new(fix);
    let equivalent = from_fix.apply(input, Timestamp::from_millis(0)).effects;

    let rendered = |effects: &[Effect]| effects.iter().map(describe).collect::<Vec<_>>();
    assert_eq!(rendered(&effects), rendered(&equivalent));
}
