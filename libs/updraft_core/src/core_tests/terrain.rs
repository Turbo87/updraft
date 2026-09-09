use super::super::*;
use super::support::*;
use crate::{AltitudeInstrument, TerrainElevation};
use claims::{assert_none, assert_ok, assert_some, assert_some_eq};

#[test]
fn terrain_uses_fused_altitude_and_rejects_other_positions() {
    let (mut core, device) = core_with_external_device();
    let pressure = |meters| {
        assert_ok!(Vec::try_from(&updraft_nmea::Pgrmz {
            altitude: Some(updraft_units::Length::from_meters(meters)),
            fix_dimension: updraft_nmea::PgrmzFixDimension::NoFix,
        }))
    };
    core.apply(Bytes::new(device, pressure(1000.0)), at(0));
    core.apply(Bytes::new(device, GGA), at(1));
    let position = gps_instruments(&core).position;
    let altitude =
        assert_some!(assert_some!(instruments(&core).derived).altitude).altitude_msl_meters;
    core.apply(
        TerrainElevation {
            position,
            meters: Some(altitude + 10.0),
        },
        at(2),
    );
    assert_some_eq!(
        instruments(&core).altitude_agl,
        AltitudeInstrument {
            meters: -10.0,
            stale: false
        }
    );
    core.apply(Bytes::new(device, pressure(1100.0)), at(1000));
    let updated = instruments(&core);
    let fused = assert_some!(assert_some!(updated.derived).altitude).altitude_msl_meters;
    assert_ne!(fused, assert_some!(gps_instruments(&core).altitude_meters));
    assert_eq!(
        assert_some!(updated.altitude_agl).meters,
        fused - (altitude + 10.0)
    );
    core.apply(Bytes::new(device, GGA_SECOND_DEVICE), at(1001));
    assert_none!(instruments(&core).terrain_elevation);
    core.apply(
        TerrainElevation {
            position,
            meters: Some(1.0),
        },
        at(1002),
    );
    assert_none!(instruments(&core).altitude_agl);
}

#[test]
fn terrain_handles_staleness_and_missing_altitude() {
    let (mut core, device) = core_with_external_device();
    core.apply(Bytes::new(device, POSITION_ONLY_RMC), at(0));
    let position = gps_instruments(&core).position;
    core.apply(
        TerrainElevation {
            position,
            meters: Some(100.0),
        },
        at(1),
    );
    assert_some_eq!(
        instruments(&core).terrain_elevation,
        AltitudeInstrument {
            meters: 100.0,
            stale: false
        }
    );
    assert_none!(instruments(&core).altitude_agl);
    core.apply(Bytes::new(device, GGA), at(2));
    assert_some!(instruments(&core).altitude_agl);
    core.apply(Tick, at(20_000));
    assert!(assert_some!(instruments(&core).terrain_elevation).stale);
    assert!(assert_some!(instruments(&core).altitude_agl).stale);
    core.apply(
        TerrainElevation {
            position,
            meters: None,
        },
        at(20_001),
    );
    assert_none!(instruments(&core).terrain_elevation);
    assert_none!(instruments(&core).altitude_agl);
}
