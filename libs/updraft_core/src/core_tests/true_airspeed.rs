use super::super::*;
use super::support::*;
use crate::ownship::Selected;
use std::assert_matches;
use updraft_units::Speed;

const LXWP0_FIRST: &[u8] = b"$LXWP0,Y,180,1000,1,1,1,1,1,1,239,174,10\r\n";
const LXWP0_SECOND: &[u8] = b"$LXWP0,Y,360,2000,2,2,2,2,2,2,180,90,20\r\n";
const LXWP0_WITHOUT_TAS: &[u8] = b"$LXWP0,Y,,3000,3,3,3,3,3,3,90,45,30\r\n";

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
