use super::super::*;
use super::support::*;
use crate::connection::ConnectionState;
use crate::{FixTime, UtcInstant, UtcTime};
use approx::assert_abs_diff_eq;
use claims::{assert_none, assert_some, assert_some_eq};
use std::assert_matches;
use updraft_units::{Length, MslAltitude};

fn selected_fix_time(sentence: &[u8]) -> FixTime {
    let (mut core, device_id) = core_with_external_device();
    core.apply(Bytes::new(device_id, sentence), at(0));
    let DomainState::Current(selected) = core.gps else {
        panic!("GPS should be current");
    };
    assert_some!(selected.value.fix_time)
}

fn utc_time(milliseconds_since_midnight: u32) -> UtcTime {
    assert_some!(UtcTime::from_milliseconds_since_midnight(
        milliseconds_since_midnight
    ))
}

#[test]
fn fix_emits_instruments_immediately() {
    let (mut core, device_id) = core_with_external_device();

    let effects = core.apply(Bytes::new(device_id, RMC), at(100)).effects;

    assert_matches!(effects.as_slice(), [Effect::Emit(Topic::Instruments(_))]);
    let [Effect::Emit(Topic::Instruments(instruments))] = effects.as_slice() else {
        unreachable!()
    };
    let gps = assert_some!(instruments.gps);
    let position = gps.position;
    assert_abs_diff_eq!(position.latitude_degrees, 50.823, epsilon = 1e-3);
    assert_abs_diff_eq!(position.longitude_degrees, 6.186, epsilon = 1e-3);
    assert_some_eq!(gps.track_degrees, 270.0);
}

#[test]
fn nmea_sentences_select_canonical_fix_times() {
    assert_eq!(
        selected_fix_time(RMC),
        FixTime::UtcInstant(UtcInstant::from_unix_milliseconds(1_767_268_800_000))
    );
    assert_eq!(
        selected_fix_time(UNDATED_RMC),
        FixTime::UtcTimeOfDay(utc_time(43_201_250))
    );
    assert_eq!(
        selected_fix_time(GGA),
        FixTime::UtcTimeOfDay(utc_time(43_200_000))
    );
}

#[test]
fn full_fix_time_precedes_then_falls_back_to_time_of_day() {
    let (mut core, device_id) = core_with_external_device();
    let full = FixTime::UtcInstant(UtcInstant::from_unix_milliseconds(1_767_268_800_000));
    let time_only = FixTime::UtcTimeOfDay(utc_time(43_201_500));

    core.apply(Bytes::new(device_id, RMC), at(0));
    core.apply(Bytes::new(device_id, GGA_LATER_TIME), at(1_000));
    core.apply(Bytes::new(device_id, GGA_WITHOUT_TIME), at(2_500));

    let DomainState::Current(selected) = core.gps else {
        panic!("GPS should be current");
    };
    assert_some_eq!(selected.value.fix_time, full);

    let effects = core.apply(Tick, at(3_000)).effects;
    assert_matches!(effects.as_slice(), [Effect::Emit(Topic::Instruments(_))]);

    let DomainState::Current(selected) = core.gps else {
        panic!("GPS should remain current");
    };
    assert_some_eq!(selected.value.fix_time, time_only);

    let effects = core.apply(Tick, at(4_000)).effects;

    let DomainState::Current(selected) = core.gps else {
        panic!("GPS should remain current");
    };
    assert!(selected.value.fix_time.is_none());
    assert_matches!(effects.as_slice(), [Effect::Emit(Topic::Instruments(_))]);
}

#[test]
fn nmea_leap_second_reuses_235959_for_fix_time() {
    assert_eq!(
        selected_fix_time(LEAP_SECOND_RMC),
        FixTime::UtcInstant(UtcInstant::from_unix_milliseconds(1_767_311_999_123))
    );
    assert_eq!(
        selected_fix_time(LEAP_SECOND_GGA),
        FixTime::UtcTimeOfDay(utc_time(86_399_123))
    );
}

#[test]
fn first_external_gps_source_has_priority() {
    let (mut core, first, second) = core_with_two_external_devices();

    core.apply(Bytes::new(first, RMC), at(0));
    core.apply(Bytes::new(second, RMC_SECOND_DEVICE), at(1));

    let position = gps_instruments(&core).position;
    assert_abs_diff_eq!(position.latitude_degrees, 50.823, epsilon = 1e-3);
    assert_abs_diff_eq!(position.longitude_degrees, 6.186, epsilon = 1e-3);
}

#[test]
fn gps_becomes_last_known_at_the_exact_freshness_boundary() {
    let (mut core, device_id) = core_with_external_device();
    core.apply(Bytes::new(device_id, RMC), at(0));

    let effects = core.apply(Tick, at(2_999)).effects;
    assert!(effects.is_empty());
    assert_matches!(core.gps, DomainState::Current(_));

    let effects = core.apply(Tick, at(3_000)).effects;
    assert_matches!(effects.as_slice(), [Effect::Emit(Topic::Instruments(_))]);
    let DomainState::LastKnown(selected) = core.gps else {
        panic!("GPS should be last known");
    };
    assert!(selected.value.fix_time.is_some());
    assert!(gps_instruments(&core).stale);
}

#[test]
fn valid_gga_position_makes_a_gps_source_eligible() {
    let (mut core, device_id) = core_with_external_device();

    let effects = core.apply(Bytes::new(device_id, GGA), at(0)).effects;

    assert_matches!(effects.as_slice(), [Effect::Emit(Topic::Instruments(_))]);
    let gps = gps_instruments(&core);
    let position = gps.position;
    assert_abs_diff_eq!(position.latitude_degrees, 50.823, epsilon = 1e-3);
    assert_abs_diff_eq!(position.longitude_degrees, 6.186, epsilon = 1e-3);
    assert_some_eq!(gps.altitude_meters, 200.0);
}

#[test]
fn a_gga_dilution_reaches_the_selected_gps_domain() {
    let (mut core, device_id) = core_with_external_device();

    core.apply(Bytes::new(device_id, GGA), at(0));

    // A receiver reports the dilution rather than an accuracy, and a
    // consumer that weighs a fix needs it.
    let DomainState::Current(selected) = core.gps else {
        unreachable!()
    };
    assert_some_eq!(selected.value.horizontal_dilution, 0.9);
}

#[test]
fn invalid_gga_does_not_replace_or_refresh_valid_data() {
    let (mut core, device_id) = core_with_external_device();
    core.apply(Bytes::new(device_id, GGA), at(0));

    let effects = core
        .apply(Bytes::new(device_id, INVALID_GGA), at(2_500))
        .effects;
    assert!(effects.is_empty());
    let position = gps_instruments(&core).position;
    assert_abs_diff_eq!(position.latitude_degrees, 50.823, epsilon = 1e-3);

    core.apply(Tick, at(3_000));
    assert_matches!(core.gps, DomainState::LastKnown(_));
}

#[test]
fn not_valid_rmc_mode_does_not_replace_or_refresh_valid_data() {
    let (mut core, device_id) = core_with_external_device();
    core.apply(Bytes::new(device_id, RMC), at(0));

    let effects = core
        .apply(Bytes::new(device_id, INVALID_MODE_RMC), at(2_500))
        .effects;
    assert!(effects.is_empty());
    let position = gps_instruments(&core).position;
    assert_abs_diff_eq!(position.latitude_degrees, 50.823, epsilon = 1e-3);

    core.apply(Tick, at(3_000));
    assert_matches!(core.gps, DomainState::LastKnown(_));
}

#[test]
fn optional_gps_fields_expire_without_values_from_another_source() {
    let (mut core, first, second) = core_with_two_external_devices();
    core.apply(Bytes::new(first, RMC), at(0));
    core.apply(Bytes::new(first, GGA), at(0));
    core.apply(Bytes::new(second, RMC_SECOND_DEVICE), at(1_000));
    core.apply(Bytes::new(second, GGA_SECOND_DEVICE), at(1_000));
    core.apply(Bytes::new(first, POSITION_ONLY_RMC), at(2_500));

    let effects = core.apply(Tick, at(3_000)).effects;

    assert_matches!(effects.as_slice(), [Effect::Emit(Topic::Instruments(_))]);
    let gps = gps_instruments(&core);
    assert!(gps.altitude_meters.is_none());
    assert!(gps.track_degrees.is_none());
    assert!(gps.ground_speed_meters_per_second.is_none());
    let position = gps.position;
    assert_abs_diff_eq!(position.latitude_degrees, 50.823, epsilon = 1e-3);
}

#[test]
fn optional_fields_without_position_wait_for_their_own_source_position() {
    let (mut core, first, second) = core_with_two_external_devices();
    core.apply(Bytes::new(second, RMC_SECOND_DEVICE), at(0));

    let mut optional_fields = OPTIONAL_ONLY_RMC.to_vec();
    optional_fields.extend_from_slice(ALTITUDE_ONLY_GGA);
    let effects = core
        .apply(Bytes::new(first, optional_fields), at(1))
        .effects;
    assert!(effects.is_empty());
    let position = gps_instruments(&core).position;
    assert_abs_diff_eq!(position.latitude_degrees, 51.0, epsilon = 1e-3);

    let effects = core
        .apply(Bytes::new(first, POSITION_ONLY_RMC), at(2))
        .effects;
    assert_matches!(effects.as_slice(), [Effect::Emit(Topic::Instruments(_))]);
    let gps = gps_instruments(&core);
    let position = gps.position;
    assert_abs_diff_eq!(position.latitude_degrees, 50.823, epsilon = 1e-3);
    assert_some_eq!(gps.altitude_meters, 250.0);
    assert_some_eq!(gps.track_degrees, 90.0);
    assert_some_eq!(gps.ground_speed_meters_per_second, 25.722222222222225);
}

#[test]
fn gps_falls_back_then_keeps_the_last_selected_stale_source() {
    let (mut core, first, second) = core_with_two_external_devices();
    core.apply(Bytes::new(first, RMC), at(0));
    core.apply(Bytes::new(second, RMC_SECOND_DEVICE), at(1_000));

    let effects = core.apply(Tick, at(3_000)).effects;
    assert_matches!(effects.as_slice(), [Effect::Emit(Topic::Instruments(_))]);
    let DomainState::Current(selected) = core.gps else {
        panic!("the fresh fallback should be current");
    };
    assert_eq!(selected.source, SourceId::External(second));

    let effects = core.apply(Tick, at(4_000)).effects;
    assert_matches!(effects.as_slice(), [Effect::Emit(Topic::Instruments(_))]);
    let DomainState::LastKnown(selected) = core.gps else {
        panic!("the last selected source should become last known");
    };
    assert_eq!(selected.source, SourceId::External(second));
    assert_eq!(selected.ingested_at, at(1_000));

    let effects = core.apply(Tick, at(5_000)).effects;
    assert!(effects.is_empty());
    let DomainState::LastKnown(selected) = core.gps else {
        panic!("the last-known source should remain unchanged");
    };
    assert_eq!(selected.source, SourceId::External(second));
    assert_some_eq!(gps_instruments(&core).track_degrees, 180.0);
}

#[test]
fn split_gps_sentence_uses_its_first_byte_input_for_freshness() {
    let (mut core, device_id) = core_with_external_device();

    let effects = core.apply(Bytes::new(device_id, &RMC[..24]), at(0)).effects;
    assert!(effects.is_empty());

    let effects = core
        .apply(Bytes::new(device_id, &RMC[24..]), at(3_000))
        .effects;
    assert!(effects.is_empty());
    assert_matches!(core.gps, DomainState::Unavailable);
}

#[test]
fn disconnected_gps_source_remains_selected_until_it_is_stale() {
    let (mut core, first, second) = core_with_two_external_devices();
    core.apply(Bytes::new(first, RMC), at(0));
    core.apply(Bytes::new(second, RMC_SECOND_DEVICE), at(1_000));

    let input = ConnectionChanged::new(first, ConnectionState::Disconnected);
    let effects = core.apply(input, at(2_500)).effects;
    assert!(effects.is_empty());
    let DomainState::Current(selected) = core.gps else {
        panic!("the disconnected source should remain current during its grace period");
    };
    assert_eq!(selected.source, SourceId::External(first));

    core.apply(Tick, at(3_000));
    let DomainState::Current(selected) = core.gps else {
        panic!("the fresh fallback should become current");
    };
    assert_eq!(selected.source, SourceId::External(second));
}

#[test]
fn equal_internal_fallback_changes_source_without_an_instruments_effect() {
    let (mut core, device_id) = core_with_external_device();
    core.apply(Bytes::new(device_id, RMC), at(0));
    let external = core
        .external_devices
        .get(device_id)
        .expect("the configured external device")
        .gps;
    let equivalent_fix = Fix {
        position: assert_some!(external.position).value,
        altitude_ellipsoid: None,
        track: Some(assert_some!(external.track).value),
        ground_speed: Some(assert_some!(external.ground_speed).value),
        fix_time: external.fix_time.full.map(|time| time.value),
    };

    let effects = core.apply(InternalGps::new(equivalent_fix), at(1)).effects;
    assert!(effects.is_empty());
    let DomainState::Current(selected) = core.gps else {
        panic!("the external source should remain current");
    };
    assert_eq!(selected.source, SourceId::External(device_id));

    let effects = core.apply(Tick, at(3_000)).effects;
    assert!(effects.is_empty());
    let DomainState::Current(selected) = core.gps else {
        panic!("the internal fallback should become current");
    };
    assert_eq!(selected.source, SourceId::InternalGps);
}

#[test]
fn one_byte_input_publishes_only_its_final_gps_snapshot() {
    let (mut core, device_id) = core_with_external_device();
    let mut input = RMC.to_vec();
    input.extend_from_slice(RMC_SECOND_DEVICE);

    let effects = core.apply(Bytes::new(device_id, input), at(0)).effects;

    let [Effect::Emit(Topic::Instruments(instruments))] = effects.as_slice() else {
        panic!("one byte input should emit one final instruments snapshot");
    };
    let position = assert_some!(instruments.gps).position;
    assert_abs_diff_eq!(position.latitude_degrees, 51.0, epsilon = 1e-3);
    assert_abs_diff_eq!(position.longitude_degrees, 7.0, epsilon = 1e-3);
}

#[test]
fn repeated_identical_sentences_emit_only_once() {
    let (mut core, device_id) = core_with_external_device();
    let mut emissions = 0;

    for millis in [0, 2_500] {
        let input = Bytes::new(device_id, RMC);
        emissions += core.apply(input, at(millis)).effects.len();
    }

    assert_eq!(emissions, 1, "only the first sentence changed any value");
    let device = core
        .external_devices
        .get(device_id)
        .expect("the configured external device");
    assert_eq!(assert_some!(device.gps.position).ingested_at, at(2_500));

    core.apply(Tick, at(3_000));
    assert_matches!(core.gps, DomainState::Current(_));
    core.apply(Tick, at(5_500));
    assert_matches!(core.gps, DomainState::LastKnown(_));
}

#[test]
fn external_devices_keep_their_timed_gps_candidates() {
    let (mut core, first_device_id, second_device_id) = core_with_two_external_devices();

    core.apply(Bytes::new(first_device_id, RMC), at(0));
    core.apply(Bytes::new(first_device_id, GGA), at(1));
    core.apply(Bytes::new(second_device_id, RMC_SECOND_DEVICE), at(2));
    core.apply(Bytes::new(second_device_id, GGA_SECOND_DEVICE), at(3));

    let first = core
        .external_devices
        .iter()
        .next()
        .expect("the first configured external device");
    let first_position = assert_some!(first.gps.position);
    assert_abs_diff_eq!(
        first_position.value.latitude().as_degrees(),
        50.823,
        epsilon = 1e-3
    );
    assert_abs_diff_eq!(
        first_position.value.longitude().as_degrees(),
        6.186,
        epsilon = 1e-3
    );
    assert_eq!(first_position.ingested_at, at(1));
    assert_eq!(assert_some!(first.gps.track).ingested_at, at(0));
    assert_eq!(assert_some!(first.gps.ground_speed).ingested_at, at(0));
    let first_altitude = assert_some!(first.gps.altitude);
    assert_eq!(
        first_altitude.value,
        MslAltitude::new(Length::from_meters(200.0))
    );
    assert_eq!(first_altitude.ingested_at, at(1));

    let second = core
        .external_devices
        .iter()
        .nth(1)
        .expect("the second configured external device");
    let second_position = assert_some!(second.gps.position);
    assert_abs_diff_eq!(
        second_position.value.latitude().as_degrees(),
        51.0,
        epsilon = 1e-3
    );
    assert_abs_diff_eq!(
        second_position.value.longitude().as_degrees(),
        7.0,
        epsilon = 1e-3
    );
    assert_eq!(second_position.ingested_at, at(3));
    assert_eq!(assert_some!(second.gps.track).ingested_at, at(2));
    assert_eq!(assert_some!(second.gps.ground_speed).ingested_at, at(2));
    let second_altitude = assert_some!(second.gps.altitude);
    assert_eq!(
        second_altitude.value,
        MslAltitude::new(Length::from_meters(300.0))
    );
    assert_eq!(second_altitude.ingested_at, at(3));

    let topics = core.topics();
    let [Topic::Instruments(instruments), ..] = topics.as_slice() else {
        unreachable!()
    };
    let gps = assert_some!(instruments.gps);
    let position = gps.position;
    assert_abs_diff_eq!(position.latitude_degrees, 50.823, epsilon = 1e-3);
    assert_abs_diff_eq!(position.longitude_degrees, 6.186, epsilon = 1e-3);
    assert_some_eq!(gps.altitude_meters, 200.0);
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

    let candidate = assert_some!(core.internal_gps.position);
    assert_eq!(candidate.ingested_at, at(100));
    assert_eq!(
        assert_some!(core.internal_gps.altitude).ingested_at,
        at(100)
    );
    assert_eq!(assert_some!(core.internal_gps.track).ingested_at, at(100));
    assert_eq!(
        assert_some!(core.internal_gps.ground_speed).ingested_at,
        at(100)
    );
    assert_matches!(effects.as_slice(), [Effect::Emit(Topic::Instruments(_))]);
    let [Effect::Emit(Topic::Instruments(instruments))] = effects.as_slice() else {
        unreachable!()
    };
    let gps = assert_some!(instruments.gps);
    let position = gps.position;
    assert_abs_diff_eq!(position.latitude_degrees, 50.823, epsilon = 1e-9);
    assert_some_eq!(gps.track_degrees, 90.0);
}

#[test]
fn internal_gps_selects_full_fix_time() {
    let mut core = Core::new(config());
    let fix_time = UtcInstant::from_unix_milliseconds(1_767_268_800_000);
    let mut reported = fix(50.823, 6.186);
    reported.fix_time = Some(fix_time);

    core.apply(InternalGps::new(reported), at(100));

    let DomainState::Current(selected) = core.gps else {
        panic!("GPS should be current");
    };
    assert_some_eq!(selected.value.fix_time, FixTime::UtcInstant(fix_time));
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
        Topic::Airspace(_),
        Topic::Traffic(_),
    ] = topics.as_slice()
    else {
        unreachable!()
    };
    // The geoid sits 46.54 m above the ellipsoid at this position, so the
    // 247 m the fix carries lands here. Pinned to the centimetre: a change
    // in what the pilot reads as altitude is a change worth seeing.
    assert_abs_diff_eq!(
        assert_some!(assert_some!(instruments.gps).altitude_meters),
        200.46,
        epsilon = 0.01
    );
}

#[test]
fn repeated_identical_fixes_emit_only_once() {
    let mut core = Core::new(config());
    let mut emissions = 0;

    for millis in 100..105 {
        let input = InternalGps::new(fix(50.823, 6.186));
        emissions += core.apply(input, at(millis)).effects.len();
    }

    // Two, not one: the first fix carries the position, and the second
    // gives the air estimate two altitudes to differentiate, which is
    // the first vertical speed. The rest change nothing.
    assert_eq!(emissions, 2, "only new values were emitted");
    assert_eq!(
        assert_some!(core.internal_gps.position).ingested_at,
        at(104)
    );
}

/// A barometric altitude, as a Garmin sentence carries it: whole feet,
/// with the checksum left off, which the decoder accepts.
fn pgrmz(meters: f64) -> Vec<u8> {
    format!("$PGRMZ,{:.0},F,2\r\n", meters / 0.3048).into_bytes()
}

#[test]
fn a_climb_reaches_the_instruments_topic() {
    let (mut core, device_id) = core_with_external_device();

    // A minute of climbing at 2 m/s, with a fix each second so the
    // estimate has a ground velocity to work from.
    let mut vertical_speed = None;
    for second in 0..60u64 {
        let time = at(second * 1_000);
        core.apply(
            Bytes::new(device_id, pgrmz(1000. + 2. * second as f64)),
            time,
        );
        core.apply(Bytes::new(device_id, RMC), time);
        if let Some(Topic::Instruments(instruments)) = core
            .topics()
            .into_iter()
            .find(|topic| matches!(topic, Topic::Instruments(_)))
        {
            vertical_speed = instruments
                .air
                .and_then(|air| air.vertical_speed_meters_per_second);
        }
    }

    let climb = assert_some!(vertical_speed);
    assert_abs_diff_eq!(climb, 2.0, epsilon = 0.05);
}

#[test]
fn a_pressure_altitude_alone_reports_no_vertical_speed() {
    let (mut core, device_id) = core_with_external_device();

    // One altitude has nothing to be differentiated against.
    core.apply(Bytes::new(device_id, pgrmz(1000.)), at(0));

    let Some(Topic::Instruments(instruments)) = core
        .topics()
        .into_iter()
        .find(|topic| matches!(topic, Topic::Instruments(_)))
    else {
        unreachable!()
    };
    assert_none!(
        instruments
            .air
            .and_then(|air| air.vertical_speed_meters_per_second)
    );
}
