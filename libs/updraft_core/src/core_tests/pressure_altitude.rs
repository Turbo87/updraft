use super::super::*;
use super::support::*;
use crate::ownship::Selected;
use claims::assert_some;
use std::assert_matches;
use updraft_units::{Length, PressureAltitude};

const PGRMZ_NO_FIX: &[u8] = b"$PGRMZ,1000,m,1\r\n";
const PGRMZ_SECOND: &[u8] = b"$PGRMZ,2000,m,3\r\n";
const PGRMZ_WITHOUT_ALTITUDE: &[u8] = b"$PGRMZ,,m,3\r\n";
const PLXVF_WITH_PRESSURE_ALTITUDE: &[u8] = b"$PLXVF,,1.00,0.87,-0.12,-0.25,90.2,244.3,\r\n";

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
        .apply(Bytes::new(device_id, PGRMZ_NO_FIX), at(0))
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

    core.apply(Bytes::new(device_id, PGRMZ_NO_FIX), at(0));
    core.apply(Bytes::new(device_id, PGRMZ_WITHOUT_ALTITUDE), at(2_500));
    core.apply(Tick, at(3_000));

    assert_matches!(core.pressure_altitude, DomainState::LastKnown(_));
}

#[test]
fn identical_pressure_altitude_refreshes_without_repeating_current_output() {
    let (mut core, device_id) = core_with_external_device();

    let effects = core
        .apply(Bytes::new(device_id, PGRMZ_NO_FIX), at(0))
        .effects;
    assert_matches!(effects.as_slice(), [Effect::Emit(Topic::Instruments(_))]);

    // The altitude repeats, but the pair of them is new: it is the
    // first interval the air estimate can differentiate, so it reports a
    // vertical speed of zero where it had none.
    let effects = core
        .apply(Bytes::new(device_id, PGRMZ_NO_FIX), at(2_500))
        .effects;
    assert_matches!(effects.as_slice(), [Effect::Emit(Topic::Instruments(_))]);

    let effects = core.apply(Tick, at(3_000)).effects;
    assert!(effects.is_empty());
    assert_matches!(core.pressure_altitude, DomainState::Current(_));
    let effects = core.apply(Tick, at(5_500)).effects;
    assert_matches!(effects.as_slice(), [Effect::Emit(Topic::Instruments(_))]);
    assert_matches!(core.pressure_altitude, DomainState::LastKnown(_));
}

#[test]
fn gps_and_pressure_altitude_select_independent_sources() {
    let (mut core, first, second) = core_with_two_external_devices();
    core.apply(Bytes::new(first, RMC), at(0));
    core.apply(Bytes::new(second, PGRMZ_SECOND), at(1));

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
    core.apply(Bytes::new(first, PGRMZ_NO_FIX), at(0));
    core.apply(Bytes::new(second, PGRMZ_SECOND), at(1_000));

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
    core.apply(Bytes::new(first, PGRMZ_NO_FIX), at(0));
    core.apply(Bytes::new(second, PGRMZ_SECOND), at(1));

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
    core.apply(Bytes::new(first, PGRMZ_NO_FIX), at(0));
    core.apply(Bytes::new(second, PGRMZ_SECOND), at(1));

    core.apply(SetExternalDeviceEnabled::disabled(first), at(2));

    let selected = current_pressure_altitude(&core);
    assert_eq!(selected.source, SourceId::External(second));

    core.apply(SetExternalDeviceEnabled::disabled(second), at(3));
    assert_matches!(core.pressure_altitude, DomainState::Unavailable);

    core.apply(SetExternalDeviceEnabled::enabled(first), at(4));
    assert_matches!(core.pressure_altitude, DomainState::Unavailable);
}
