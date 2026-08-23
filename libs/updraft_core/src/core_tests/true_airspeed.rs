use super::super::*;
use super::support::*;
use crate::ownship::Selected;
use approx::assert_abs_diff_eq;
use claims::{assert_ok, assert_some};
use std::assert_matches;
use updraft_nmea::{Lxwp0, Pgrmz, PgrmzFixDimension};
use updraft_units::{Length, Speed};

const LXWP0_FIRST: &[u8] = b"$LXWP0,Y,180,1000,1,1,1,1,1,1,239,174,10\r\n";
const LXWP0_SECOND: &[u8] = b"$LXWP0,Y,360,2000,2,2,2,2,2,2,180,90,20\r\n";
const LXWP0_WITHOUT_TAS: &[u8] = b"$LXWP0,Y,,3000,3,3,3,3,3,3,90,45,30\r\n";

fn pressure_altitude(altitude: Length) -> Vec<u8> {
    assert_ok!(Vec::try_from(&Pgrmz {
        altitude: Some(altitude),
        fix_dimension: PgrmzFixDimension::NoFix,
    }))
}

fn pressure_altitude_with_airspeed(altitude: Length, true_airspeed: Speed) -> Vec<u8> {
    let mut bytes = pressure_altitude(altitude);
    bytes.extend(assert_ok!(Vec::try_from(&Lxwp0 {
        logger_running: None,
        true_airspeed: Some(true_airspeed),
        pressure_altitude: None,
        vario_samples: Vec::new(),
        heading: None,
        wind_direction: None,
        wind_speed: None,
    })));
    bytes
}

fn current_true_airspeed(core: &Core) -> Selected<Speed> {
    let DomainState::Current(selected) = core.true_airspeed else {
        panic!("true airspeed should be current");
    };
    selected
}

#[test]
fn lxwp0_selects_true_airspeed_without_using_other_fields() {
    let (mut core, device_id) = core_with_external_device();

    core.apply(Bytes::new(device_id, LXWP0_FIRST), at(0));

    let selected = current_true_airspeed(&core);
    assert_eq!(selected.source, SourceId::External(device_id));
    assert_eq!(selected.value, Speed::from_kilometers_per_hour(180.0));
    assert_eq!(selected.ingested_at, at(0));
    assert_matches!(core.pressure_altitude, DomainState::Unavailable);
    let published = assert_some!(instruments(&core).true_airspeed);
    assert_eq!(published.meters_per_second, 50.0);
    assert!(!published.stale);
}

#[test]
fn absent_true_airspeed_does_not_update_or_refresh_the_candidate() {
    let (mut core, device_id) = core_with_external_device();
    core.apply(Bytes::new(device_id, LXWP0_WITHOUT_TAS), at(0));
    assert_matches!(core.true_airspeed, DomainState::Unavailable);

    core.apply(Bytes::new(device_id, LXWP0_FIRST), at(0));
    core.apply(Bytes::new(device_id, LXWP0_WITHOUT_TAS), at(2_500));
    core.apply(Tick, at(3_000));

    assert_matches!(core.true_airspeed, DomainState::LastKnown(_));
}

#[test]
fn identical_true_airspeed_refreshes_the_candidate() {
    let (mut core, device_id) = core_with_external_device();
    core.apply(Bytes::new(device_id, LXWP0_FIRST), at(0));
    core.apply(Bytes::new(device_id, LXWP0_FIRST), at(2_500));

    core.apply(Tick, at(3_000));
    assert_matches!(core.true_airspeed, DomainState::Current(_));
    core.apply(Tick, at(5_500));
    assert_matches!(core.true_airspeed, DomainState::LastKnown(_));
    assert!(assert_some!(instruments(&core).true_airspeed).stale);
}

#[test]
fn changed_true_airspeed_at_same_timestamp_keeps_the_first_value() {
    let (mut core, device_id) = core_with_external_device();
    core.apply(Bytes::new(device_id, LXWP0_FIRST), at(0));

    core.apply(Bytes::new(device_id, LXWP0_SECOND), at(0));

    let selected = current_true_airspeed(&core);
    assert_eq!(selected.value, Speed::from_kilometers_per_hour(180.));
}

#[test]
fn same_batch_airspeed_compensates_the_current_pressure_altitude() {
    let (mut core, device_id) = core_with_external_device();
    let first_speed = Speed::from_meters_per_second(50.);
    let first_altitude = Length::from_meters(1_000.);
    let bytes = pressure_altitude_with_airspeed(first_altitude, first_speed);
    core.apply(Bytes::new(device_id, bytes), at(0));

    let second_speed = Speed::from_meters_per_second(40.);
    let first_speed_squared = first_speed.as_meters_per_second().powi(2);
    let second_speed_squared = second_speed.as_meters_per_second().powi(2);
    let altitude_gain = (first_speed_squared - second_speed_squared) / (2. * 9.80665);
    let second_altitude = Length::from_meters(1_000. + altitude_gain);
    let bytes = pressure_altitude_with_airspeed(second_altitude, second_speed);
    core.apply(Bytes::new(device_id, bytes), at(1_000));

    let derived = assert_some!(instruments(&core).derived);
    let vario = assert_some!(derived.vario);
    assert_abs_diff_eq!(vario.meters_per_second, 0., epsilon = 0.01);
}

#[test]
fn pressure_input_at_the_airspeed_freshness_boundary_keeps_the_vario_stale() {
    let (mut core, device_id) = core_with_external_device();
    let altitude = Length::from_meters(1_000.);
    let air_speed = Speed::from_meters_per_second(50.);
    let bytes = pressure_altitude_with_airspeed(altitude, air_speed);
    core.apply(Bytes::new(device_id, bytes), at(0));
    let bytes = pressure_altitude(altitude);
    core.apply(Bytes::new(device_id, bytes), at(1_000));
    let derived = assert_some!(instruments(&core).derived);
    let current = assert_some!(derived.vario);
    assert!(!current.stale);

    let bytes = pressure_altitude(altitude);
    core.apply(Bytes::new(device_id, bytes), at(3_000));

    assert_matches!(core.true_airspeed, DomainState::LastKnown(_));
    let derived = assert_some!(instruments(&core).derived);
    let stale = assert_some!(derived.vario);
    assert!(stale.stale);
}

#[test]
fn gps_pressure_altitude_and_true_airspeed_select_independent_sources() {
    let (mut core, first, second) = core_with_two_external_devices();
    core.apply(Bytes::new(first, RMC), at(0));
    core.apply(Bytes::new(second, LXWP0_SECOND), at(1));

    let DomainState::Current(gps) = core.gps else {
        panic!("GPS should be current");
    };
    assert_eq!(gps.source, SourceId::External(first));
    assert_matches!(core.pressure_altitude, DomainState::Unavailable);
    assert_eq!(
        current_true_airspeed(&core).source,
        SourceId::External(second)
    );
}

#[test]
fn true_airspeed_input_expires_gps_at_the_exact_freshness_boundary() {
    let (mut core, device_id) = core_with_external_device();
    core.apply(Bytes::new(device_id, RMC), at(0));

    let effects = core
        .apply(Bytes::new(device_id, LXWP0_FIRST), at(3_000))
        .effects;

    assert_matches!(core.gps, DomainState::LastKnown(_));
    assert_matches!(core.true_airspeed, DomainState::Current(_));
    assert_matches!(effects.as_slice(), [Effect::Emit(Topic::Instruments(_))]);
}

#[test]
fn true_airspeed_follows_source_priority_fallback_and_reset() {
    let (mut core, first, second) = core_with_two_external_devices();
    core.apply(Bytes::new(first, LXWP0_FIRST), at(0));
    core.apply(Bytes::new(second, LXWP0_SECOND), at(1_000));

    assert_eq!(
        current_true_airspeed(&core).source,
        SourceId::External(first)
    );

    core.apply(Tick, at(3_000));
    assert_eq!(
        current_true_airspeed(&core).source,
        SourceId::External(second)
    );

    core.apply(ReorderExternalDevices::new(vec![first, second]), at(3_001));
    assert_eq!(
        current_true_airspeed(&core).source,
        SourceId::External(second)
    );

    core.apply(SetExternalDeviceEnabled::disabled(second), at(3_002));
    assert_matches!(core.true_airspeed, DomainState::Unavailable);

    core.apply(SetExternalDeviceEnabled::enabled(first), at(3_003));
    assert_matches!(core.true_airspeed, DomainState::Unavailable);
}
