use super::super::*;
use super::support::*;
use crate::connection::ConnectionSpec;
use crate::settings::SettingsSnapshot;
use crate::topic::LatLon as TopicLatLon;
use approx::assert_abs_diff_eq;
use claims::{assert_some, assert_some_eq};
use std::assert_matches;
use updraft_units::{Length, MslAltitude};

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

    for millis in 100..105 {
        let input = Bytes::new(device_id, RMC);
        emissions += core.apply(input, at(millis)).effects.len();
    }

    assert_eq!(emissions, 1, "only the first sentence changed any value");
    let device = core
        .external_devices
        .get(device_id)
        .expect("the configured external device");
    assert_eq!(assert_some!(device.gps.position).ingested_at, at(104));
}

#[test]
fn external_devices_keep_their_timed_gps_candidates() {
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
    assert_eq!(first_position.ingested_at, at(0));
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
    assert_eq!(second_position.ingested_at, at(2));
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
        assert_some!(instruments.altitude_msl_meters),
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

    assert_eq!(emissions, 1, "only the first fix changed any value");
    assert_eq!(
        assert_some!(core.internal_gps.position).ingested_at,
        at(104)
    );
}
