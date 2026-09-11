use super::super::*;
use super::support::*;
use approx::assert_abs_diff_eq;
use claims::{assert_ok, assert_some};
use updraft_geo::LatLon;
use updraft_units::{Angle, Length};

const FIX: &[u8] = b"$GPRMC,120000,A,5000.000,N,00800.000,E,100,90,050826,,,A\r\n";
const NEXT_FIX: &[u8] = b"$GPRMC,120001,A,5000.000,N,00800.100,E,100,0,050826,,,A\r\n";
const CYCLE: &[u8] = b"$PFLAU,1,1,2,1,0,,0,,,\r\n$PGRMZ,1000,f,3\r\n";
const TARGET: &[u8] = b"$PFLAA,0,0,0,0,1,ABC123,90,0,40,0,1,0,0\r\n";

fn position(core: &Core) -> LatLon {
    let target = traffic_snapshot(core).remove(0);
    LatLon::from_degrees(
        target.position.latitude_degrees,
        target.position.longitude_degrees,
    )
}

fn assert_position(core: &Core, expected: LatLon) {
    let error = position(core).distance(expected).as_meters();
    assert_abs_diff_eq!(error, 0.0, epsilon = 1e-6);
}

#[test]
fn flarm_reference_uses_the_cycle_fix_on_both_sides_of_the_next_gps() {
    let (mut core, device) = core_with_external_device();
    let origin = LatLon::from_degrees(50.0, 8.0);
    let expected = origin.destination(
        Angle::from_degrees(90.0),
        Length::from_meters(100.0 * 1852.0 / 3600.0 * 2.0),
    );
    core.apply(Bytes::new(device, [FIX, CYCLE, TARGET].concat()), at(0));
    assert_position(&core, expected);
    core.apply(Bytes::new(device, [NEXT_FIX, TARGET].concat()), at(1_000));
    assert_position(&core, expected);
    core.apply(Bytes::new(device, [CYCLE, TARGET].concat()), at(1_000));
    let expected = LatLon::from_degrees(50.0, 8.0 + 0.1 / 60.0).destination(
        Angle::from_degrees(0.0),
        Length::from_meters(100.0 * 1852.0 / 3600.0 * 2.0),
    );
    assert_position(&core, expected);
}

#[test]
fn flarm_reference_setting_changes_the_next_report() {
    let (mut core, device) = core_with_external_device();
    core.apply(Bytes::new(device, [FIX, CYCLE, TARGET].concat()), at(0));
    let corrected = position(&core);
    core.apply(SetFlarmPositionCorrection { enabled: false }, at(1));
    core.apply(Bytes::new(device, TARGET), at(2));
    assert_position(&core, LatLon::from_degrees(50.0, 8.0));
    core.apply(SetFlarmPositionCorrection { enabled: true }, at(3));
    core.apply(Bytes::new(device, TARGET), at(4));
    assert_eq!(position(&core), corrected);
}

#[test]
fn flarm_reference_falls_back_without_a_usable_cycle_fix() {
    let origin = LatLon::from_degrees(50.0, 8.0);
    for input in [
        [FIX, TARGET].concat(),
        [FIX, CYCLE, b"$PGRMZ,1000,f,3\r\n", TARGET].concat(),
        [
            b"$GPRMC,120000,A,5000.000,N,00800.000,E,100,,050826,,,A\r\n",
            CYCLE,
            TARGET,
        ]
        .concat(),
        [FIX, CYCLE, b"$PFLAA,0,0,0,0,1,ABC123,90,0,40,0,1,0,1\r\n"].concat(),
    ] {
        let (mut core, device) = core_with_external_device();
        core.apply(Bytes::new(device, input), at(0));
        assert_position(&core, origin);
    }
    let (mut core, device) = core_with_external_device();
    core.apply(Bytes::new(device, [FIX, CYCLE].concat()), at(0));
    core.apply(Bytes::new(device, TARGET), at(3_000));
    assert_position(&core, origin);
}

#[test]
fn flarm_reference_does_not_cross_devices_or_reconnects() {
    use crate::connection::ConnectionState;
    let (mut core, first, second) = core_with_two_external_devices();
    core.apply(Bytes::new(first, [FIX, CYCLE].concat()), at(0));
    core.apply(
        Bytes::new(second, [NEXT_FIX, CYCLE, TARGET].concat()),
        at(1_000),
    );
    core.apply(Bytes::new(first, TARGET), at(1_001));
    let expected = LatLon::from_degrees(50.0, 8.0).destination(
        Angle::from_degrees(90.0),
        Length::from_meters(100.0 * 1852.0 / 3600.0 * 2.0),
    );
    assert_position(&core, expected);
    core.apply(
        ConnectionChanged {
            device_id: first,
            state: ConnectionState::Disconnected,
        },
        at(1_002),
    );
    core.apply(Bytes::new(first, [FIX, TARGET].concat()), at(1_003));
    assert_position(&core, LatLon::from_degrees(50.0, 8.0));
}

#[test]
fn flarm_reference_resets_on_backward_gps_time() {
    let (mut core, device) = core_with_external_device();
    core.apply(
        Bytes::new(device, [FIX, CYCLE, NEXT_FIX, CYCLE].concat()),
        at(0),
    );
    core.apply(Bytes::new(device, [FIX, TARGET].concat()), at(1));
    assert_position(&core, LatLon::from_degrees(50.0, 8.0));
}

#[test]
fn flarm_reference_crosses_midnight_and_handles_missing_markers() {
    let (mut core, device) = core_with_external_device();
    let before = assert_ok!(std::str::from_utf8(FIX)).replace("120000", "235959");
    let after = assert_ok!(std::str::from_utf8(NEXT_FIX)).replace("120001", "000000");
    core.apply(
        Bytes::new(device, [before.as_bytes(), CYCLE, TARGET].concat()),
        at(0),
    );
    let previous = position(&core);
    core.apply(
        Bytes::new(device, [after.as_bytes(), TARGET].concat()),
        at(1_000),
    );
    assert_eq!(position(&core), previous);
    // With both markers missing, the next GPS bounds the cycle to its predecessor.
    let later = after.replace("000000,A", "000001,A");
    core.apply(
        Bytes::new(device, [later.as_bytes(), TARGET].concat()),
        at(2_000),
    );
    let expected = LatLon::from_degrees(50.0, 8.0 + 0.1 / 60.0).destination(
        Angle::from_degrees(0.0),
        Length::from_meters(100.0 * 1852.0 / 3600.0 * 2.0),
    );
    assert_position(&core, expected);
}

#[test]
fn flarm_reference_discards_invalid_gps_before_traffic() {
    for invalid in [
        b"$GPRMC,,A,5000,N,00800,E,100,90,050826,,,A\r\n".as_slice(),
        b"$GPGGA,120000,5000,N,00800,E,0,08,1,100,M,0,M,,\r\n",
    ] {
        let (mut core, device) = core_with_external_device();
        core.apply(
            Bytes::new(device, [FIX, CYCLE, invalid, TARGET].concat()),
            at(0),
        );
        assert_position(&core, LatLon::from_degrees(50., 8.));
    }
}

#[test]
fn flarm_reference_uses_a_stationary_cycle_fix_without_track() {
    let (mut core, device) = core_with_external_device();
    let stationary = b"$GPRMC,120000,A,5000,N,00800,E,0,,050826,,,A\r\n";
    let gga = b"$GPGGA,120000,5000,N,00800,E,1,08,1,100,M,0,M,,\r\n";
    let input = [stationary.as_slice(), gga, CYCLE, NEXT_FIX, TARGET].concat();
    core.apply(Bytes::new(device, input), at(0));
    assert_position(&core, LatLon::from_degrees(50., 8.));
}

#[test]
fn flarm_reference_keeps_the_active_fix_until_the_cycle_changes() {
    let (mut core, device) = core_with_external_device();
    core.apply(Bytes::new(device, [FIX, CYCLE, TARGET].concat()), at(0));
    let active = position(&core);
    let incomplete = assert_ok!(std::str::from_utf8(NEXT_FIX)).replace(",100,0,", ",100,,");
    core.apply(
        Bytes::new(device, [incomplete.as_bytes(), TARGET].concat()),
        at(1_000),
    );
    assert_eq!(position(&core), active);
    core.apply(Bytes::new(device, [CYCLE, TARGET].concat()), at(1_001));
    assert_position(&core, LatLon::from_degrees(50., 8. + 0.1 / 60.));
    core.apply(Bytes::new(device, [NEXT_FIX, TARGET].concat()), at(1_002));
    let expected = LatLon::from_degrees(50., 8. + 0.1 / 60.).destination(
        Angle::from_degrees(0.),
        Length::from_meters(100. * 1852. / 3600. * 2.),
    );
    assert_position(&core, expected);
    core.apply(
        Bytes::new(device, [incomplete.as_bytes(), TARGET].concat()),
        at(1_003),
    );
    assert_position(&core, LatLon::from_degrees(50., 8. + 0.1 / 60.));
}

fn altitude(core: &Core) -> f64 {
    assert_some!(traffic_snapshot(core).remove(0).altitude_msl_meters)
}

fn gga(time: &str, height: f64) -> Vec<u8> {
    format!("$GPGGA,{time},5000,N,00800,E,1,08,1,{height},M,0,M,,\r\n").into_bytes()
}

#[test]
fn flarm_altitude_uses_the_cycle_height_and_changes_only_when_enabled() {
    let (mut core, device) = core_with_external_device();
    let before = gga("115959", 100.);
    let current = gga("120000", 103.);
    let next = gga("120001", 108.);
    let input = [before.as_slice(), FIX, &current, CYCLE, TARGET].concat();
    core.apply(Bytes::new(device, input), at(0));
    assert_eq!(altitude(&core), 109.);
    core.apply(
        Bytes::new(device, [NEXT_FIX, &next, TARGET].concat()),
        at(1_000),
    );
    assert_eq!(altitude(&core), 109.);
    core.apply(SetFlarmPositionCorrection { enabled: false }, at(1_001));
    core.apply(Bytes::new(device, TARGET), at(1_002));
    assert_eq!(altitude(&core), 108.);
    core.apply(SetFlarmPositionCorrection { enabled: true }, at(1_003));
    core.apply(Bytes::new(device, TARGET), at(1_004));
    assert_eq!(altitude(&core), 109.);
    core.apply(Bytes::new(device, [CYCLE, TARGET].concat()), at(1_005));
    assert_eq!(altitude(&core), 118.);
}

#[test]
fn flarm_altitude_falls_back_without_a_usable_history() {
    let before = gga("115959", 100.);
    let current = gga("120000", 103.);
    for input in [
        [FIX, &current, CYCLE, TARGET].concat(),
        [before.as_slice(), FIX, &current, TARGET].concat(),
        [gga("115955", 100.).as_slice(), FIX, &current, CYCLE, TARGET].concat(),
        [
            before.as_slice(),
            FIX,
            &current,
            CYCLE,
            b"$PFLAA,0,0,0,0,1,ABC123,90,0,40,0,1,0,1\r\n",
        ]
        .concat(),
    ] {
        let (mut core, device) = core_with_external_device();
        core.apply(Bytes::new(device, input), at(0));
        assert_eq!(altitude(&core), 103.);
    }
    let (mut core, device) = core_with_external_device();
    core.apply(
        Bytes::new(device, [before.as_slice(), FIX, &current, CYCLE].concat()),
        at(0),
    );
    core.apply(Bytes::new(device, TARGET), at(3_000));
    assert_eq!(altitude(&core), 103.);
}

#[test]
fn flarm_altitude_handles_sink_duplicates_and_midnight() {
    let (mut core, device) = core_with_external_device();
    let before = gga("235958", 110.);
    let current = gga("000000", 104.);
    let fix = assert_ok!(std::str::from_utf8(FIX)).replace("120000", "000000");
    let input = [
        before.as_slice(),
        fix.as_bytes(),
        &current,
        CYCLE,
        &current,
        TARGET,
    ]
    .concat();
    core.apply(Bytes::new(device, input), at(0));
    assert_eq!(altitude(&core), 98.);
}

#[test]
fn flarm_altitude_clears_history_on_invalid_fixes_and_does_not_cross_devices() {
    let before = gga("115959", 100.);
    let current = gga("120000", 103.);
    for invalid in [
        "$GPGGA,120000,5000,N,00800,E,1,08,1,,M,0,M,,\r\n",
        "$GPGGA,,5000,N,00800,E,1,08,1,103,M,0,M,,\r\n",
        "$GPGGA,120000,5000,N,00800,E,0,08,1,103,M,0,M,,\r\n",
        "$GPGGA,115958,5000,N,00800,E,1,08,1,103,M,0,M,,\r\n",
    ] {
        let (mut core, device) = core_with_external_device();
        let input = [
            before.as_slice(),
            FIX,
            &current,
            CYCLE,
            invalid.as_bytes(),
            &current,
            CYCLE,
            TARGET,
        ]
        .concat();
        core.apply(Bytes::new(device, input), at(0));
        assert_eq!(altitude(&core), 103.);
    }
    let (mut core, first, second) = core_with_two_external_devices();
    core.apply(
        Bytes::new(first, [before.as_slice(), FIX, &current, CYCLE].concat()),
        at(0),
    );
    core.apply(
        Bytes::new(second, [FIX, &current, CYCLE, TARGET].concat()),
        at(1),
    );
    assert_eq!(altitude(&core), 103.);
    core.apply(Bytes::new(first, TARGET), at(2));
    assert_eq!(altitude(&core), 109.);
    core.apply(
        ConnectionChanged {
            device_id: first,
            state: crate::connection::ConnectionState::Disconnected,
        },
        at(3),
    );
    core.apply(
        Bytes::new(first, [FIX, &current, CYCLE, TARGET].concat()),
        at(4),
    );
    assert_eq!(altitude(&core), 103.);
}
