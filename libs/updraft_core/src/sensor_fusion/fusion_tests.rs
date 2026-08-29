use claims::assert_some;
use updraft_geo::LatLon;
use updraft_units::{Angle, Length, MslAltitude, PressureAltitude, Speed};

use super::{FusionInputs, SensorFusion};
use crate::ExternalDeviceId;
use crate::ownship::{DomainState, GpsSnapshot, Selected, SourceId, Timed};
use crate::time::Timestamp;

fn selected<T>(millis: u64, value: T) -> Selected<T> {
    Selected {
        source: SourceId::InternalGps,
        ingested_at: Timestamp::from_millis(millis),
        value,
    }
}

fn selected_from<T>(source: SourceId, millis: u64, value: T) -> Selected<T> {
    Selected {
        source,
        ingested_at: Timestamp::from_millis(millis),
        value,
    }
}

fn gps(second: u64) -> GpsSnapshot {
    let at = Timestamp::from_millis(second * 1_000);
    let altitude = MslAltitude::new(Length::from_meters(1_000.));
    GpsSnapshot {
        position: LatLon::from_degrees(50.8, 6.2),
        altitude_msl: Some(Timed::new(altitude, at)),
        track: Some(Timed::new(Angle::from_degrees(6. * second as f64), at)),
        ground_speed: Some(Timed::new(Speed::from_kilometers_per_hour(120.), at)),
        fix_time: None,
    }
}

fn circling_gps(second: u64) -> GpsSnapshot {
    let mut gps = gps(second);
    gps.track = Some(Timed::new(
        Angle::from_degrees(18. * second as f64),
        Timestamp::from_millis(second * 1_000),
    ));
    gps
}

fn fusion_with_converged_wind() -> SensorFusion {
    let mut fusion = SensorFusion::default();
    for second in 0..60 {
        let millis = second * 1_000;
        let air_speed = selected(millis, Speed::from_kilometers_per_hour(100.));
        fusion.update(FusionInputs {
            gps: DomainState::Current(selected(millis, gps(second))),
            true_airspeed: DomainState::Current(air_speed),
            pressure_altitude: DomainState::Unavailable,
        });
    }

    let wind = assert_some!(assert_some!(fusion.instruments()).wind);
    assert!(!wind.stale);
    fusion
}

#[test]
fn stale_airspeed_does_not_stale_wind() {
    let mut fusion = SensorFusion::default();
    for second in 0..60 {
        let millis = second * 1_000;
        let pressure_altitude =
            selected(millis, PressureAltitude::new(Length::from_meters(1_000.)));
        let air_speed = selected(millis, Speed::from_kilometers_per_hour(100.));
        let gps = selected(millis, gps(second));
        fusion.update(FusionInputs {
            gps: DomainState::Current(gps),
            true_airspeed: DomainState::Current(air_speed),
            pressure_altitude: DomainState::Current(pressure_altitude),
        });
    }

    let current = assert_some!(assert_some!(fusion.instruments()).wind);
    assert!(!current.stale);

    let pressure_altitude = selected(60_000, PressureAltitude::new(Length::from_meters(1_000.)));
    let air_speed = selected(59_000, Speed::from_kilometers_per_hour(100.));
    fusion.update(FusionInputs {
        gps: DomainState::Current(selected(60_000, gps(60))),
        true_airspeed: DomainState::LastKnown(air_speed),
        pressure_altitude: DomainState::Current(pressure_altitude),
    });

    let retained = assert_some!(assert_some!(fusion.instruments()).wind);
    assert_eq!(retained.direction_degrees, current.direction_degrees);
    assert_eq!(
        retained.speed_meters_per_second,
        current.speed_meters_per_second
    );
    assert!(!retained.stale);
}

#[test]
fn airspeed_source_change_after_outage_resets_wind() {
    let mut fusion = fusion_with_converged_wind();
    fusion.update(FusionInputs {
        gps: DomainState::Current(selected(59_000, gps(59))),
        true_airspeed: DomainState::Unavailable,
        pressure_altitude: DomainState::Unavailable,
    });

    let source = SourceId::External(ExternalDeviceId(1));
    let air_speed = selected_from(source, 60_000, Speed::from_kilometers_per_hour(100.));
    fusion.update(FusionInputs {
        gps: DomainState::Current(selected(60_000, gps(60))),
        true_airspeed: DomainState::Current(air_speed),
        pressure_altitude: DomainState::Unavailable,
    });

    let wind = assert_some!(assert_some!(fusion.instruments()).wind);
    assert!(wind.stale);
}

#[test]
fn mismatched_ground_velocity_timestamps_stale_the_wind() {
    let mut fusion = fusion_with_converged_wind();
    let mut gps = gps(60);
    gps.track = Some(Timed::new(
        Angle::from_degrees(360.),
        Timestamp::from_millis(59_000),
    ));

    let air_speed = selected(60_000, Speed::from_kilometers_per_hour(100.));
    fusion.update(FusionInputs {
        gps: DomainState::Current(selected(60_000, gps)),
        true_airspeed: DomainState::Current(air_speed),
        pressure_altitude: DomainState::Unavailable,
    });

    let wind = assert_some!(assert_some!(fusion.instruments()).wind);
    assert!(wind.stale);
}

#[test]
fn rejected_airspeed_measurement_keeps_motion_estimates_stale() {
    let mut fusion = fusion_with_converged_wind();
    let mut rejected_gps = gps(60);
    let at = Timestamp::from_millis(60_000);
    rejected_gps.track = Some(Timed::new(Angle::from_degrees(355.), at));
    let rejected_air_speed = selected(60_000, Speed::ZERO);

    fusion.update(FusionInputs {
        gps: DomainState::Current(selected(60_000, rejected_gps)),
        true_airspeed: DomainState::Current(rejected_air_speed),
        pressure_altitude: DomainState::Unavailable,
    });

    for second in 61..=62 {
        fusion.update(FusionInputs {
            gps: DomainState::Current(selected(second * 1_000, gps(second))),
            true_airspeed: DomainState::Current(rejected_air_speed),
            pressure_altitude: DomainState::Unavailable,
        });
    }

    let instruments = assert_some!(fusion.instruments());
    let wind = assert_some!(instruments.wind);
    assert!(wind.stale);
}

#[test]
fn inferred_airspeed_produces_vario_through_sensor_fusion() {
    let mut fusion = SensorFusion::default();
    for second in 0..=70 {
        let millis = second * 1_000;
        let altitude = PressureAltitude::new(Length::from_meters(1_000. + second as f64));
        fusion.update(FusionInputs {
            gps: DomainState::Current(selected(millis, circling_gps(second))),
            true_airspeed: DomainState::Unavailable,
            pressure_altitude: DomainState::Current(selected(millis, altitude)),
        });
    }

    let vario = assert_some!(assert_some!(fusion.instruments()).vario);
    assert!(!vario.stale);
}

#[test]
fn gnss_fix_uses_its_inferred_airspeed_for_vario() {
    let mut fusion = SensorFusion::default();
    for second in 0..=20 {
        let millis = second * 1_000;
        fusion.update(FusionInputs {
            gps: DomainState::Current(selected(millis, circling_gps(second))),
            true_airspeed: DomainState::Unavailable,
            pressure_altitude: DomainState::Unavailable,
        });
    }

    let vario = assert_some!(assert_some!(fusion.instruments()).vario);
    assert!(!vario.stale);
}

#[test]
fn stale_gps_stales_inferred_vario() {
    let mut fusion = SensorFusion::default();
    for second in 0..=70 {
        let millis = second * 1_000;
        let altitude = PressureAltitude::new(Length::from_meters(1_000. + second as f64));
        fusion.update(FusionInputs {
            gps: DomainState::Current(selected(millis, circling_gps(second))),
            true_airspeed: DomainState::Unavailable,
            pressure_altitude: DomainState::Current(selected(millis, altitude)),
        });
    }

    let current = assert_some!(assert_some!(fusion.instruments()).vario);
    assert!(!current.stale);

    fusion.update(FusionInputs {
        gps: DomainState::LastKnown(selected(70_000, circling_gps(70))),
        true_airspeed: DomainState::Unavailable,
        pressure_altitude: DomainState::Current(selected(
            71_000,
            PressureAltitude::new(Length::from_meters(1_071.)),
        )),
    });

    let stale = assert_some!(assert_some!(fusion.instruments()).vario);
    assert!(stale.stale);
}

#[test]
fn unconverged_wind_stales_inferred_airspeed() {
    let mut fusion = fusion_with_converged_wind();
    for second in 60..=3_600 {
        let millis = second * 1_000;
        let mut gps = gps(second);
        gps.track = Some(Timed::new(Angle::ZERO, Timestamp::from_millis(millis)));
        fusion.update(FusionInputs {
            gps: DomainState::Current(selected(millis, gps)),
            true_airspeed: DomainState::Unavailable,
            pressure_altitude: DomainState::Unavailable,
        });
    }

    let instruments = assert_some!(fusion.instruments());
    assert!(assert_some!(instruments.wind).stale);
    assert!(assert_some!(instruments.airspeed).stale);
}

#[test]
fn stale_inputs_do_not_make_cached_airspeed_current() {
    let mut fusion = SensorFusion::default();
    let mut last_air_speed = None;
    let mut last_gps = None;
    let mut last_pressure_altitude = None;
    for second in 0..60 {
        let millis = second * 1_000;
        let air_speed = selected(millis, Speed::from_kilometers_per_hour(100.));
        let gps = selected(millis, gps(second));
        let pressure_altitude =
            selected(millis, PressureAltitude::new(Length::from_meters(1_000.)));
        fusion.update(FusionInputs {
            gps: DomainState::Current(gps),
            true_airspeed: DomainState::Current(air_speed),
            pressure_altitude: DomainState::Current(pressure_altitude),
        });
        last_air_speed = Some(air_speed);
        last_gps = Some(gps);
        last_pressure_altitude = Some(pressure_altitude);
    }

    let last_air_speed = assert_some!(last_air_speed);
    let last_gps = assert_some!(last_gps);
    let last_pressure_altitude = assert_some!(last_pressure_altitude);
    fusion.update(FusionInputs {
        gps: DomainState::LastKnown(last_gps),
        true_airspeed: DomainState::Current(last_air_speed),
        pressure_altitude: DomainState::Current(last_pressure_altitude),
    });
    let gps_stale = assert_some!(fusion.instruments());
    assert!(assert_some!(gps_stale.wind).stale);
    assert!(!assert_some!(gps_stale.airspeed).stale);

    fusion.update(FusionInputs {
        gps: DomainState::LastKnown(last_gps),
        true_airspeed: DomainState::LastKnown(last_air_speed),
        pressure_altitude: DomainState::Current(last_pressure_altitude),
    });
    assert!(assert_some!(assert_some!(fusion.instruments()).airspeed).stale);
}
