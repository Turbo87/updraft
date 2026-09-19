use super::support::{UNDATED_RMC, at, core_with_external_device, fix, instruments};
use crate::{Bytes, Core, InternalGps, SettingsSnapshot, UtcInstant, UtcTick};
use approx::assert_abs_diff_eq;
use claims::{assert_none, assert_some};

const REFERENCE_UTC_MILLIS: i64 = 1_066_419_030_000;

#[test]
fn device_utc_calculates_solar_position_for_ownship() {
    let mut core = Core::new(SettingsSnapshot::default());
    assert_none!(instruments(&core).solar_position);
    core.apply(
        UtcTick::new(UtcInstant::from_unix_milliseconds(REFERENCE_UTC_MILLIS)),
        at(0),
    );
    core.apply(InternalGps::new(fix(39.742476, -105.1786)), at(0));

    let solar = assert_some!(instruments(&core).solar_position);
    assert_abs_diff_eq!(solar.azimuth_degrees, 194.34024, epsilon = 0.02);
    assert_abs_diff_eq!(solar.elevation_degrees, 39.88838, epsilon = 0.02);
    assert!(!solar.stale);
}

#[test]
fn complete_gps_utc_takes_precedence_and_advances_with_monotonic_time() {
    let mut core = Core::new(SettingsSnapshot::default());
    core.apply(UtcTick::new(UtcInstant::from_unix_milliseconds(0)), at(0));
    let mut reported = fix(39.742476, -105.1786);
    reported.fix_time = Some(UtcInstant::from_unix_milliseconds(REFERENCE_UTC_MILLIS));
    core.apply(InternalGps::new(reported), at(1_000));
    let first = assert_some!(instruments(&core).solar_position);

    core.apply(
        UtcTick::new(UtcInstant::from_unix_milliseconds(60_000)),
        at(61_000),
    );
    let advanced = assert_some!(instruments(&core).solar_position);
    assert_ne!(advanced.azimuth_degrees, first.azimuth_degrees);
    assert!(advanced.stale);
}

#[test]
fn time_only_gps_uses_device_utc() {
    let (mut core, device_id) = core_with_external_device();
    core.apply(
        UtcTick::new(UtcInstant::from_unix_milliseconds(REFERENCE_UTC_MILLIS)),
        at(0),
    );
    core.apply(Bytes::new(device_id, UNDATED_RMC), at(0));

    assert_some!(instruments(&core).solar_position);
}
