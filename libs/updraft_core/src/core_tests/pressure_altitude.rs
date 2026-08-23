use super::super::*;
use super::support::*;
use crate::connection::ConnectionSpec;
use crate::ownship::Selected;
use approx::assert_abs_diff_eq;
use claims::{assert_none, assert_ok, assert_some};
use std::assert_matches;
use updraft_nmea::PgrmzFixDimension::{NoFix, ThreeDimensional};
use updraft_nmea::{Pgrmz, PgrmzFixDimension};
use updraft_units::{Length, PressureAltitude};

const PLXVF_WITH_PRESSURE_ALTITUDE: &[u8] = b"$PLXVF,,1.00,0.87,-0.12,-0.25,90.2,244.3,\r\n";

fn pgrmz(meters: Option<f64>, fix_dimension: PgrmzFixDimension) -> Vec<u8> {
    let altitude = meters.map(Length::from_meters);
    assert_ok!(Vec::try_from(&Pgrmz {
        altitude,
        fix_dimension,
    }))
}

fn current_pressure_altitude(core: &Core) -> Selected<PressureAltitude> {
    let DomainState::Current(selected) = core.pressure_altitude else {
        panic!("pressure altitude should be current");
    };
    selected
}

#[test]
fn pgrmz_selects_pressure_altitude_without_using_fix_dimension() {
    let (mut core, device_id) = core_with_external_device();

    let effects = core
        .apply(Bytes::new(device_id, pgrmz(Some(1_000.), NoFix)), at(0))
        .effects;

    let selected = current_pressure_altitude(&core);
    assert_eq!(selected.source, SourceId::External(device_id));
    assert_eq!(
        selected.value,
        PressureAltitude::new(Length::from_meters(1_000.0))
    );
    assert_eq!(selected.ingested_at, at(0));
    assert_matches!(effects.as_slice(), [Effect::Emit(Topic::Instruments(_))]);
    let published = assert_some!(instruments(&core).pressure_altitude);
    assert_eq!(published.meters, 1_000.0);
    assert!(!published.stale);
}

#[test]
fn unsupported_or_incomplete_pressure_altitude_does_not_update_the_candidate() {
    let (mut core, device_id) = core_with_external_device();
    core.apply(Bytes::new(device_id, PLXVF_WITH_PRESSURE_ALTITUDE), at(0));
    assert_matches!(core.pressure_altitude, DomainState::Unavailable);

    core.apply(Bytes::new(device_id, pgrmz(Some(1_000.), NoFix)), at(0));
    core.apply(
        Bytes::new(device_id, pgrmz(None, ThreeDimensional)),
        at(2_500),
    );
    core.apply(Tick, at(3_000));

    assert_matches!(core.pressure_altitude, DomainState::LastKnown(_));
}

#[test]
fn identical_pressure_altitude_initializes_the_fused_rate() {
    let (mut core, device_id) = core_with_external_device();

    let effects = core
        .apply(Bytes::new(device_id, pgrmz(Some(1_000.), NoFix)), at(0))
        .effects;
    assert_matches!(effects.as_slice(), [Effect::Emit(Topic::Instruments(_))]);
    assert_none!(instruments(&core).derived);

    let effects = core
        .apply(Bytes::new(device_id, pgrmz(Some(1_000.), NoFix)), at(2_500))
        .effects;
    assert_matches!(effects.as_slice(), [Effect::Emit(Topic::Instruments(_))]);
    let derived = assert_some!(instruments(&core).derived);
    let raw_vertical_speed = assert_some!(derived.raw_vertical_speed);
    assert_eq!(raw_vertical_speed.meters_per_second, 0.0);
    assert!(!raw_vertical_speed.stale);
    let vertical_speed = assert_some!(derived.vertical_speed);
    assert_eq!(vertical_speed.meters_per_second, 0.0);
    assert!(!vertical_speed.stale);

    let effects = core.apply(Tick, at(3_000)).effects;
    assert!(effects.is_empty());
    assert_matches!(core.pressure_altitude, DomainState::Current(_));
    let effects = core.apply(Tick, at(5_500)).effects;
    assert_matches!(effects.as_slice(), [Effect::Emit(Topic::Instruments(_))]);
    assert_matches!(core.pressure_altitude, DomainState::LastKnown(_));
}

#[test]
fn internal_gps_input_expires_pressure_altitude_at_the_exact_freshness_boundary() {
    let (mut core, device_id) = core_with_external_device();
    core.apply(Bytes::new(device_id, pgrmz(Some(1_000.), NoFix)), at(0));

    let effects = core
        .apply(InternalGps::new(fix(50.823, 6.186)), at(3_000))
        .effects;

    assert_matches!(core.pressure_altitude, DomainState::LastKnown(_));
    assert_matches!(effects.as_slice(), [Effect::Emit(Topic::Instruments(_))]);
}

#[test]
fn pressure_altitude_climb_updates_fused_instruments() {
    let (mut core, device_id) = core_with_external_device();

    let mut last_effects = Vec::new();
    for second in 0..60u64 {
        let meters = 1_000.0 + 2.0 * second as f64;
        last_effects = core
            .apply(
                Bytes::new(device_id, pgrmz(Some(meters), NoFix)),
                at(second * 1_000),
            )
            .effects;
    }

    let [Effect::Emit(Topic::Instruments(instruments))] = last_effects.as_slice() else {
        panic!("the climb should emit derived instruments");
    };
    let derived = assert_some!(instruments.derived.as_ref());
    let raw_vertical_speed = assert_some!(derived.raw_vertical_speed);
    assert_abs_diff_eq!(raw_vertical_speed.meters_per_second, 2.0, epsilon = 0.05);
    assert!(!raw_vertical_speed.stale);
    let vertical_speed = assert_some!(derived.vertical_speed);
    assert_abs_diff_eq!(vertical_speed.meters_per_second, 2.0, epsilon = 0.05);
    assert!(!vertical_speed.stale);
}

#[test]
fn pressure_source_change_keeps_the_previous_vertical_speed_stale() {
    let (mut core, first, second) = core_with_two_external_devices();
    core.apply(Bytes::new(first, pgrmz(Some(1_000.), NoFix)), at(0));
    core.apply(Bytes::new(first, pgrmz(Some(1_000.), NoFix)), at(1_000));
    core.apply(Bytes::new(second, pgrmz(Some(2_000.), NoFix)), at(2_000));

    let current = assert_some!(assert_some!(instruments(&core).derived).raw_vertical_speed);
    assert_eq!(current.meters_per_second, 0.0);
    assert!(!current.stale);

    core.apply(Tick, at(4_000));

    let selected = current_pressure_altitude(&core);
    assert_eq!(selected.source, SourceId::External(second));
    let derived = assert_some!(instruments(&core).derived);
    let raw_vertical_speed = assert_some!(derived.raw_vertical_speed);
    assert_eq!(raw_vertical_speed.meters_per_second, 0.0);
    assert!(raw_vertical_speed.stale);
    let vertical_speed = assert_some!(derived.vertical_speed);
    assert_eq!(vertical_speed.meters_per_second, 0.0);
    assert!(vertical_speed.stale);
}

#[test]
fn stale_pressure_altitude_keeps_the_previous_vertical_speed_stale() {
    let (mut core, device_id) = core_with_external_device();
    core.apply(Bytes::new(device_id, pgrmz(Some(1_000.), NoFix)), at(0));
    core.apply(Bytes::new(device_id, pgrmz(Some(1_000.), NoFix)), at(1_000));

    core.apply(Tick, at(4_000));

    let stale = assert_some!(assert_some!(instruments(&core).derived).raw_vertical_speed);
    assert_eq!(stale.meters_per_second, 0.0);
    assert!(stale.stale);
}

#[test]
fn out_of_order_pressure_altitude_keeps_the_previous_vertical_speed_stale() {
    let (mut core, device_id) = core_with_external_device();
    core.apply(Bytes::new(device_id, pgrmz(Some(1_000.), NoFix)), at(0));
    core.apply(Bytes::new(device_id, pgrmz(Some(1_000.), NoFix)), at(1_000));
    core.apply(Tick, at(4_000));

    core.apply(Bytes::new(device_id, pgrmz(Some(2_000.), NoFix)), at(500));

    let stale = assert_some!(assert_some!(instruments(&core).derived).raw_vertical_speed);
    assert_eq!(stale.meters_per_second, 0.0);
    assert!(stale.stale);
}

#[test]
fn pressure_altitude_gap_restarts_its_vertical_speed_series_without_a_tick() {
    let (mut core, device_id) = core_with_external_device();
    core.apply(Bytes::new(device_id, pgrmz(Some(1_000.), NoFix)), at(0));
    core.apply(Bytes::new(device_id, pgrmz(Some(1_000.), NoFix)), at(1_000));

    core.apply(
        Bytes::new(device_id, pgrmz(Some(2_000.), NoFix)),
        at(31_001),
    );

    let stale = assert_some!(assert_some!(instruments(&core).derived).raw_vertical_speed);
    assert_eq!(stale.meters_per_second, 0.0);
    assert!(stale.stale);

    core.apply(
        Bytes::new(device_id, pgrmz(Some(2_000.), NoFix)),
        at(32_001),
    );
    let current = assert_some!(assert_some!(instruments(&core).derived).raw_vertical_speed);
    assert_eq!(current.meters_per_second, 0.0);
    assert!(!current.stale);
}

#[test]
fn reset_pressure_source_restarts_its_vertical_speed_series() {
    let (mut core, device_id) = core_with_external_device();
    core.apply(Bytes::new(device_id, pgrmz(Some(1_000.), NoFix)), at(0));
    core.apply(Bytes::new(device_id, pgrmz(Some(1_000.), NoFix)), at(1_000));

    core.apply(SetExternalDeviceEnabled::disabled(device_id), at(2_000));
    core.apply(SetExternalDeviceEnabled::enabled(device_id), at(3_000));
    core.apply(Bytes::new(device_id, pgrmz(Some(2_000.), NoFix)), at(4_000));

    let stale = assert_some!(assert_some!(instruments(&core).derived).raw_vertical_speed);
    assert_eq!(stale.meters_per_second, 0.0);
    assert!(stale.stale);

    core.apply(Bytes::new(device_id, pgrmz(Some(2_000.), NoFix)), at(5_000));
    let current = assert_some!(assert_some!(instruments(&core).derived).raw_vertical_speed);
    assert_eq!(current.meters_per_second, 0.0);
    assert!(!current.stale);
}

#[test]
fn editing_pressure_source_restarts_its_vertical_speed_series() {
    let (mut core, device_id) = core_with_external_device();
    core.apply(Bytes::new(device_id, pgrmz(Some(1_000.), NoFix)), at(0));
    core.apply(Bytes::new(device_id, pgrmz(Some(1_000.), NoFix)), at(1_000));

    let spec = ConnectionSpec::tcp("192.0.2.1", 10110);
    core.apply(EditExternalDevice::new(device_id, spec), at(2_000));
    core.apply(Bytes::new(device_id, pgrmz(Some(2_000.), NoFix)), at(3_000));

    let stale = assert_some!(assert_some!(instruments(&core).derived).raw_vertical_speed);
    assert_eq!(stale.meters_per_second, 0.0);
    assert!(stale.stale);

    core.apply(Bytes::new(device_id, pgrmz(Some(2_000.), NoFix)), at(4_000));
    let current = assert_some!(assert_some!(instruments(&core).derived).raw_vertical_speed);
    assert_eq!(current.meters_per_second, 0.0);
    assert!(!current.stale);
}

#[test]
fn gps_and_pressure_altitude_select_independent_sources() {
    let (mut core, first, second) = core_with_two_external_devices();
    core.apply(Bytes::new(first, RMC), at(0));
    core.apply(
        Bytes::new(second, pgrmz(Some(2_000.), ThreeDimensional)),
        at(1),
    );

    let DomainState::Current(gps) = core.gps else {
        panic!("GPS should be current");
    };
    let pressure_altitude = current_pressure_altitude(&core);
    assert_eq!(gps.source, SourceId::External(first));
    assert_eq!(pressure_altitude.source, SourceId::External(second));
}

#[test]
fn pressure_altitude_falls_back_then_becomes_last_known() {
    let (mut core, first, second) = core_with_two_external_devices();
    core.apply(Bytes::new(first, pgrmz(Some(1_000.), NoFix)), at(0));
    core.apply(
        Bytes::new(second, pgrmz(Some(2_000.), ThreeDimensional)),
        at(1_000),
    );

    let selected = current_pressure_altitude(&core);
    assert_eq!(selected.source, SourceId::External(first));

    let effects = core.apply(Tick, at(3_000)).effects;
    assert_matches!(effects.as_slice(), [Effect::Emit(Topic::Instruments(_))]);

    let selected = current_pressure_altitude(&core);
    assert_eq!(selected.source, SourceId::External(second));
    let published = assert_some!(instruments(&core).pressure_altitude);
    assert_eq!(published.meters, 2_000.0);
    assert!(!published.stale);

    let effects = core.apply(Tick, at(4_000)).effects;
    assert_matches!(effects.as_slice(), [Effect::Emit(Topic::Instruments(_))]);

    let DomainState::LastKnown(selected) = core.pressure_altitude else {
        panic!("selected pressure altitude should become last known");
    };
    assert_eq!(selected.source, SourceId::External(second));
    assert_eq!(selected.ingested_at, at(1_000));
    assert!(assert_some!(instruments(&core).pressure_altitude).stale);

    core.apply(ReorderExternalDevices::new(vec![second, first]), at(4_001));
    let DomainState::LastKnown(selected) = core.pressure_altitude else {
        panic!("reorder should keep the selected last-known altitude");
    };
    assert_eq!(selected.source, SourceId::External(second));
}

#[test]
fn reorder_reselects_fresh_pressure_altitude_without_discarding_candidates() {
    let (mut core, first, second) = core_with_two_external_devices();
    core.apply(Bytes::new(first, pgrmz(Some(1_000.), NoFix)), at(0));
    core.apply(
        Bytes::new(second, pgrmz(Some(2_000.), ThreeDimensional)),
        at(1),
    );

    core.apply(ReorderExternalDevices::new(vec![second, first]), at(2));
    let selected = current_pressure_altitude(&core);
    assert_eq!(selected.source, SourceId::External(second));

    core.apply(ReorderExternalDevices::new(vec![first, second]), at(2));
    let selected = current_pressure_altitude(&core);
    assert_eq!(selected.source, SourceId::External(first));
}

#[test]
fn disabling_pressure_sources_reselects_and_discards_candidates() {
    let (mut core, first, second) = core_with_two_external_devices();
    core.apply(Bytes::new(first, pgrmz(Some(1_000.), NoFix)), at(0));
    core.apply(
        Bytes::new(second, pgrmz(Some(2_000.), ThreeDimensional)),
        at(1),
    );

    core.apply(SetExternalDeviceEnabled::disabled(first), at(2));

    let selected = current_pressure_altitude(&core);
    assert_eq!(selected.source, SourceId::External(second));

    core.apply(SetExternalDeviceEnabled::disabled(second), at(3));
    assert_matches!(core.pressure_altitude, DomainState::Unavailable);

    core.apply(SetExternalDeviceEnabled::enabled(first), at(4));
    assert_matches!(core.pressure_altitude, DomainState::Unavailable);
}
