use claims::{assert_none, assert_some};
use updraft_geo::LatLon;
use updraft_units::{Angle, Length, MslAltitude, PressureAltitude, Speed};

use super::{FusionInputs, SensorFusion};
use crate::ExternalDeviceId;
use crate::climb::Velocity;
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

fn fusion_with_current_netto() -> (SensorFusion, Selected<Speed>, Selected<PressureAltitude>) {
    let polar = updraft_polar::POLAR_STORE
        .iter()
        .find(|entry| entry.name == "JS-3-18m")
        .expect("the built-in store has this glider type")
        .glide_polar();
    let mut fusion = SensorFusion::default();
    fusion.set_polar(polar);

    for second in 0..60 {
        let millis = second * 1_000;
        let air_speed = selected(millis, Speed::from_kilometers_per_hour(108.));
        let pressure_altitude =
            selected(millis, PressureAltitude::new(Length::from_meters(1_000.)));
        fusion.update(FusionInputs {
            gps: DomainState::Unavailable,
            true_airspeed: DomainState::Current(air_speed),
            pressure_altitude: DomainState::Current(pressure_altitude),
        });
    }

    let air_speed = selected(59_000, Speed::from_kilometers_per_hour(108.));
    let pressure_altitude = selected(59_000, PressureAltitude::new(Length::from_meters(1_000.)));
    (fusion, air_speed, pressure_altitude)
}

fn gps(second: u64) -> GpsSnapshot {
    let at = Timestamp::from_millis(second * 1_000);
    let altitude = MslAltitude::new(Length::from_meters(1_000.));
    GpsSnapshot {
        position: LatLon::from_degrees(50.8, 6.2),
        altitude_msl: Some(Timed::new(altitude, at)),
        track: Some(Timed::new(Angle::from_degrees(6. * second as f64), at)),
        turn_rate: None,
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
    let bank = assert_some!(assert_some!(fusion.instruments()).bank);
    assert!(!bank.stale);
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
fn airspeed_source_change_after_outage_keeps_the_wind() {
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

    let instruments = assert_some!(fusion.instruments());
    // The new airspeed sample is a wind measurement, so the wind changes.
    assert!(!assert_some!(instruments.wind).stale);
    assert!(!assert_some!(instruments.heading).stale);
}

#[test]
fn gps_source_change_keeps_the_wind() {
    let mut fusion = fusion_with_converged_wind();
    let expected = assert_some!(assert_some!(fusion.instruments()).wind);
    let source = SourceId::External(ExternalDeviceId(1));
    fusion.update(FusionInputs {
        gps: DomainState::Current(selected_from(source, 60_000, gps(60))),
        true_airspeed: DomainState::Unavailable,
        pressure_altitude: DomainState::Unavailable,
    });

    let instruments = assert_some!(fusion.instruments());
    assert_eq!(assert_some!(instruments.wind), expected);
    assert!(!assert_some!(instruments.airspeed).stale);
}

#[test]
fn unavailable_gps_keeps_the_wind_for_the_next_fix() {
    let mut fusion = fusion_with_converged_wind();
    let expected = assert_some!(assert_some!(fusion.instruments()).wind);
    fusion.update(FusionInputs {
        gps: DomainState::Unavailable,
        true_airspeed: DomainState::Unavailable,
        pressure_altitude: DomainState::Unavailable,
    });
    assert!(assert_some!(assert_some!(fusion.instruments()).wind).stale);

    fusion.update(FusionInputs {
        gps: DomainState::Current(selected(61_000, gps(61))),
        true_airspeed: DomainState::Unavailable,
        pressure_altitude: DomainState::Unavailable,
    });

    let instruments = assert_some!(fusion.instruments());
    assert_eq!(assert_some!(instruments.wind), expected);
    assert!(!assert_some!(instruments.airspeed).stale);
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
    assert!(assert_some!(instruments.heading).stale);
    assert!(assert_some!(instruments.bank).stale);
}

#[test]
fn inferred_airspeed_produces_vario_through_sensor_fusion() {
    let mut fusion = SensorFusion::default();
    fly_circling_climb(&mut fusion, 0..=70);

    let vario = assert_some!(assert_some!(fusion.instruments()).vario);
    assert!(!vario.stale);
}

#[test]
fn inferred_airspeed_stays_current_between_gps_samples() {
    let mut fusion = SensorFusion::default();
    let polar = updraft_polar::POLAR_STORE
        .iter()
        .find(|entry| entry.name == "JS-3-18m")
        .expect("the built-in store has this glider type")
        .glide_polar();
    fusion.set_polar(polar);

    fly_circling_climb(&mut fusion, 0..=70);

    let current = assert_some!(fusion.instruments());
    assert!(!assert_some!(current.airspeed).stale);
    assert!(!assert_some!(current.vario).stale);
    assert!(!assert_some!(current.netto).stale);

    let mut inputs = FusionInputs {
        gps: DomainState::Current(selected(70_000, circling_gps(70))),
        true_airspeed: DomainState::Unavailable,
        pressure_altitude: DomainState::Current(selected(
            70_000,
            PressureAltitude::new(Length::from_meters(1_070.)),
        )),
    };
    for _ in 0..10 {
        fusion.update(inputs);
        assert_eq!(assert_some!(fusion.instruments()), current);
    }

    inputs.gps = DomainState::LastKnown(selected(70_000, circling_gps(70)));
    fusion.update(inputs);
    let stale = assert_some!(fusion.instruments());
    assert!(assert_some!(stale.airspeed).stale);
    assert_none!(stale.vario);
    assert_none!(stale.netto);
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

    fusion.update(FusionInputs {
        gps: DomainState::LastKnown(selected(20_000, circling_gps(20))),
        true_airspeed: DomainState::Unavailable,
        pressure_altitude: DomainState::Unavailable,
    });

    let instruments = assert_some!(fusion.instruments());
    let stale = assert_some!(instruments.vario);
    assert_eq!(stale.meters_per_second, vario.meters_per_second);
    assert!(stale.stale);
    assert!(assert_some!(instruments.vertical_speed).stale);
}

#[test]
fn stale_gps_removes_inferred_vario_while_pressure_altitude_is_current() {
    let mut fusion = SensorFusion::default();
    fly_circling_climb(&mut fusion, 0..=70);

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

    let instruments = assert_some!(fusion.instruments());
    assert_none!(instruments.vario);
    assert!(!assert_some!(instruments.vertical_speed).stale);
}

fn glide(fusion: &mut SensorFusion, seconds: std::ops::RangeInclusive<u64>) {
    for second in seconds {
        let millis = second * 1_000;
        let mut gps = gps(second);
        gps.track = Some(Timed::new(Angle::ZERO, Timestamp::from_millis(millis)));
        fusion.update(FusionInputs {
            gps: DomainState::Current(selected(millis, gps)),
            true_airspeed: DomainState::Unavailable,
            pressure_altitude: DomainState::Unavailable,
        });
    }
}

#[test]
fn converged_wind_keeps_inferred_airspeed_without_measurements() {
    let mut fusion = fusion_with_converged_wind();
    glide(&mut fusion, 60..=3 * 3_600);

    let instruments = assert_some!(fusion.instruments());
    assert!(!assert_some!(instruments.wind).stale);
    assert!(!assert_some!(instruments.airspeed).stale);
}

#[test]
fn inferred_airspeed_returns_after_a_gps_outage() {
    let mut fusion = fusion_with_converged_wind();
    glide(&mut fusion, 60..=70);
    fusion.update(FusionInputs {
        gps: DomainState::LastKnown(selected(70_000, gps(70))),
        true_airspeed: DomainState::Unavailable,
        pressure_altitude: DomainState::Unavailable,
    });
    let outage = assert_some!(fusion.instruments());
    assert!(assert_some!(outage.wind).stale);
    assert!(assert_some!(outage.airspeed).stale);

    glide(&mut fusion, 75..=76);

    let instruments = assert_some!(fusion.instruments());
    assert!(!assert_some!(instruments.wind).stale);
    assert!(!assert_some!(instruments.airspeed).stale);
}

#[test]
fn restored_wind_stays_reported_without_measurements() {
    let mut fusion = SensorFusion::default();
    fusion.restore_wind(Velocity {
        east: Speed::from_meters_per_second(-3.),
        north: Speed::from_meters_per_second(4.),
    });

    glide(&mut fusion, 0..=2 * 3_600);

    let instruments = assert_some!(fusion.instruments());
    assert!(!assert_some!(instruments.wind).stale);
    assert!(!assert_some!(instruments.airspeed).stale);
}

#[test]
fn stale_inputs_do_not_make_cached_motion_estimates_current() {
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
    assert!(assert_some!(gps_stale.heading).stale);
    assert!(assert_some!(gps_stale.bank).stale);
    assert!(!assert_some!(gps_stale.airspeed).stale);

    fusion.update(FusionInputs {
        gps: DomainState::LastKnown(last_gps),
        true_airspeed: DomainState::LastKnown(last_air_speed),
        pressure_altitude: DomainState::Current(last_pressure_altitude),
    });
    let all_stale = assert_some!(fusion.instruments());
    assert!(assert_some!(all_stale.heading).stale);
    assert!(assert_some!(all_stale.bank).stale);
    assert!(assert_some!(all_stale.airspeed).stale);
}

#[test]
fn stale_airspeed_removes_netto() {
    let (mut fusion, air_speed, pressure_altitude) = fusion_with_current_netto();

    let current = assert_some!(fusion.instruments());
    assert!(!assert_some!(current.netto).stale);
    assert!(!assert_some!(current.relative_vario).stale);

    fusion.update(FusionInputs {
        gps: DomainState::Unavailable,
        true_airspeed: DomainState::LastKnown(air_speed),
        pressure_altitude: DomainState::Current(pressure_altitude),
    });

    let instruments = assert_some!(fusion.instruments());
    assert_none!(instruments.netto);
    assert_none!(instruments.relative_vario);
    assert!(!assert_some!(instruments.estimated_netto).stale);
}

#[test]
fn returning_altitude_without_airspeed_removes_the_stale_vario_and_netto() {
    let (mut fusion, air_speed, pressure_altitude) = fusion_with_current_netto();
    fusion.update(FusionInputs {
        gps: DomainState::Unavailable,
        true_airspeed: DomainState::Current(air_speed),
        pressure_altitude: DomainState::LastKnown(pressure_altitude),
    });
    assert!(assert_some!(assert_some!(fusion.instruments()).vario).stale);

    let altitude = PressureAltitude::new(Length::from_meters(1_000.));
    fusion.update(FusionInputs {
        gps: DomainState::Unavailable,
        true_airspeed: DomainState::Unavailable,
        pressure_altitude: DomainState::Current(selected(63_000, altitude)),
    });

    let instruments = assert_some!(fusion.instruments());
    assert_none!(instruments.vario);
    assert_none!(instruments.netto);
    assert_none!(instruments.relative_vario);
    assert!(!assert_some!(instruments.vertical_speed).stale);
}

#[test]
fn stale_pressure_altitude_stales_netto_independently() {
    let (mut fusion, air_speed, pressure_altitude) = fusion_with_current_netto();

    let average = assert_some!(assert_some!(fusion.instruments()).average_vario);
    assert!(!average.stale);
    let current = assert_some!(assert_some!(fusion.instruments()).netto);
    assert!(!current.stale);
    let relative = assert_some!(assert_some!(fusion.instruments()).relative_vario);
    assert!(!relative.stale);

    fusion.update(FusionInputs {
        gps: DomainState::Unavailable,
        true_airspeed: DomainState::Current(air_speed),
        pressure_altitude: DomainState::LastKnown(pressure_altitude),
    });

    let retained = assert_some!(assert_some!(fusion.instruments()).average_vario);
    assert_eq!(retained.meters_per_second, average.meters_per_second);
    assert!(retained.stale);
    let retained = assert_some!(assert_some!(fusion.instruments()).netto);
    assert_eq!(retained.meters_per_second, current.meters_per_second);
    assert!(retained.stale);
    let retained = assert_some!(assert_some!(fusion.instruments()).relative_vario);
    assert_eq!(retained.meters_per_second, relative.meters_per_second);
    assert!(retained.stale);
}

#[test]
fn estimated_netto_follows_the_vertical_speed() {
    let mut fusion = SensorFusion::default();
    let altitude = |second: u64| {
        let altitude = PressureAltitude::new(Length::from_meters(1_000.));
        selected(second * 1_000, altitude)
    };
    for second in 0..60 {
        fusion.update(FusionInputs {
            gps: DomainState::Unavailable,
            true_airspeed: DomainState::Unavailable,
            pressure_altitude: DomainState::Current(altitude(second)),
        });
    }
    assert_none!(assert_some!(fusion.instruments()).estimated_netto);

    let polar = updraft_polar::POLAR_STORE
        .iter()
        .find(|entry| entry.name == "JS-3-18m")
        .expect("the built-in store has this glider type")
        .glide_polar();
    fusion.set_polar(polar);

    let current = assert_some!(fusion.instruments());
    assert_none!(current.netto);
    assert_none!(current.relative_vario);
    let netto = assert_some!(current.estimated_netto);
    assert!(!netto.stale);
    let relative = assert_some!(current.estimated_relative_vario);
    assert!(!relative.stale);

    fusion.update(FusionInputs {
        gps: DomainState::Unavailable,
        true_airspeed: DomainState::Unavailable,
        pressure_altitude: DomainState::LastKnown(altitude(59)),
    });

    let stale = assert_some!(fusion.instruments());
    assert!(assert_some!(stale.vertical_speed).stale);
    let retained = assert_some!(stale.estimated_netto);
    assert_eq!(retained.meters_per_second, netto.meters_per_second);
    assert!(retained.stale);
    let retained = assert_some!(stale.estimated_relative_vario);
    assert_eq!(retained.meters_per_second, relative.meters_per_second);
    assert!(retained.stale);
}

fn fly_circling_climb(fusion: &mut SensorFusion, seconds: std::ops::RangeInclusive<u64>) {
    for second in seconds {
        let millis = second * 1_000;
        let altitude = PressureAltitude::new(Length::from_meters(1_000. + second as f64));
        fusion.update(FusionInputs {
            gps: DomainState::Current(selected(millis, circling_gps(second))),
            true_airspeed: DomainState::Unavailable,
            pressure_altitude: DomainState::Current(selected(millis, altitude)),
        });
    }
}

#[test]
fn reset_removes_the_flight_data_but_keeps_the_polar() {
    let (mut fusion, _, _) = fusion_with_current_netto();
    fusion.reset();
    assert_none!(fusion.instruments());

    for second in 60..62 {
        let altitude = PressureAltitude::new(Length::from_meters(1_000.));
        fusion.update(FusionInputs {
            gps: DomainState::Unavailable,
            true_airspeed: DomainState::Unavailable,
            pressure_altitude: DomainState::Current(selected(second * 1_000, altitude)),
        });
    }

    let instruments = assert_some!(fusion.instruments());
    assert!(!assert_some!(instruments.estimated_netto).stale);
}
