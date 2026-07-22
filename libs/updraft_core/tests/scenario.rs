//! Whole-flight scenario tests: a plain loop over `App::handle()` with no
//! async runtime, sleeps, or wall clock.

use claims::{assert_matches, assert_none, assert_some, assert_some_eq};
use std::time::Duration;
use updraft_core::device::DeviceId;
use updraft_core::flight::{
    Availability, FlightChange, FlightComputeJob, FlightComputeKind, FlightComputeResult,
    FlightConfig, FlightInput, FlightSnapshot, GetTraceStats, GnssData, GnssUpdate, Observation,
    SourceId, Sourced,
};
use updraft_core::{
    App, Change, ComputeFailure, ComputeJob, ComputeKind, ComputeResult, Effect, Input, Update,
};
use updraft_geo::LatLon;
use updraft_units::{Angle, Length, MslAltitude, PressureAltitude, Speed};

#[test]
fn app_selects_gnss_by_external_device_order() {
    let mut app = App::new();
    let preferred = DeviceId::new(1);
    let fallback = DeviceId::new(2);
    app.handle(external_device_order_input(vec![preferred, fallback]));

    let internal = fix(2., 50., 6.);
    let update = app.handle(gnss_input(SourceId::Internal, gnss_observation(internal)));
    assert_eq!(
        update.changes,
        vec![Change::Flight(FlightChange::Gnss(internal.current_data()))]
    );

    let fallback_fix = fix(3., 51., 7.);
    let update = app.handle(gnss_input(
        SourceId::External(fallback),
        gnss_observation(fallback_fix),
    ));
    assert_eq!(
        update.changes,
        vec![Change::Flight(FlightChange::Gnss(
            fallback_fix.current_data()
        ))]
    );

    let preferred_fix = fix(4., 52., 8.);
    let update = app.handle(gnss_input(
        SourceId::External(preferred),
        gnss_observation(preferred_fix),
    ));
    assert_eq!(
        update.changes,
        vec![Change::Flight(FlightChange::Gnss(
            preferred_fix.current_data()
        ))]
    );

    let newer_fallback_fix = fix(4.5, 53., 9.);
    let update = app.handle(gnss_input(
        SourceId::External(fallback),
        gnss_observation(newer_fallback_fix),
    ));
    assert!(update.changes.is_empty());
    assert_eq!(app.snapshot().flight.gnss, preferred_fix.current_data());

    let update = app.handle(external_device_order_input(vec![fallback, preferred]));
    assert_eq!(
        update.changes,
        vec![Change::Flight(FlightChange::Gnss(
            newer_fallback_fix.current_data()
        ))]
    );
    assert_eq!(
        app.snapshot().flight.gnss,
        newer_fallback_fix.current_data()
    );

    let update = app.handle(external_device_order_input(Vec::new()));
    assert_eq!(
        update.changes,
        vec![Change::Flight(FlightChange::Gnss(internal.current_data()))]
    );
    assert_eq!(app.snapshot().flight.gnss, internal.current_data());
}

#[test]
fn changing_external_order_preserves_simulator_selection() {
    let mut app = App::new();
    let gnss = fix(1., 50., 6.);
    app.handle(gnss_input(SourceId::Simulator, gnss_observation(gnss)));
    let pressure_altitude = PressureAltitude::new(Length::from_meters(900.));
    app.handle(pressure_altitude_input(
        SourceId::Simulator,
        1.,
        pressure_altitude,
    ));

    let update = app.handle(external_device_order_input(vec![DeviceId::new(1)]));

    assert!(update.changes.is_empty());
    assert_eq!(app.snapshot().flight.gnss, gnss.current_data());
    assert_eq!(
        app.snapshot().flight.pressure_altitude,
        current(pressure_altitude)
    );
}

#[test]
fn changing_external_order_preserves_the_selected_stale_source() {
    let mut app = App::new();
    let preferred = DeviceId::new(1);
    let fallback = DeviceId::new(2);
    app.handle(external_device_order_input(vec![preferred, fallback]));
    app.handle(gnss_input(
        SourceId::External(preferred),
        gnss_observation(fix(0., 50., 6.)),
    ));
    let fallback_fix = fix(1., 51., 7.);
    app.handle(gnss_input(
        SourceId::External(fallback),
        gnss_observation(fallback_fix),
    ));
    app.handle(Input::Clock { clock_time: at(3.) });
    app.handle(Input::Clock { clock_time: at(4.) });

    let update = app.handle(external_device_order_input(vec![
        DeviceId::new(3),
        preferred,
        fallback,
    ]));

    assert!(update.changes.is_empty());
    assert_eq!(app.snapshot().flight.gnss, fallback_fix.last_known_data());
}

#[test]
fn removing_the_only_live_source_makes_its_signals_unavailable() {
    let mut app = App::new();
    let device = DeviceId::new(1);
    let source = SourceId::External(device);
    app.handle(external_device_order_input(vec![device]));
    let gnss = fix(1., 50., 6.);
    app.handle(gnss_input(source, gnss_observation(gnss)));
    let pressure_altitude = PressureAltitude::new(Length::from_meters(900.));
    app.handle(pressure_altitude_input(source, 1., pressure_altitude));

    let update = app.handle(external_device_order_input(Vec::new()));

    assert_eq!(
        update.changes,
        vec![
            Change::Flight(FlightChange::Gnss(GnssData::default())),
            Change::Flight(FlightChange::PressureAltitude(Availability::Unavailable)),
        ]
    );
    assert_eq!(app.snapshot().flight.gnss, GnssData::default());
    assert_eq!(
        app.snapshot().flight.pressure_altitude,
        Availability::Unavailable
    );
}

#[test]
fn readding_a_removed_source_does_not_restore_old_observations() {
    let mut app = App::new();
    let device = DeviceId::new(1);
    let source = SourceId::External(device);
    app.handle(external_device_order_input(vec![device]));
    app.handle(gnss_input(source, gnss_observation(fix(1., 50., 6.))));
    app.handle(pressure_altitude_input(
        source,
        1.,
        PressureAltitude::new(Length::from_meters(900.)),
    ));
    app.handle(external_device_order_input(Vec::new()));

    let update = app.handle(external_device_order_input(vec![device]));

    assert!(update.changes.is_empty());
    assert_eq!(app.snapshot().flight.gnss, GnssData::default());
    assert_eq!(
        app.snapshot().flight.pressure_altitude,
        Availability::Unavailable
    );
}

#[test]
fn app_selects_gnss_and_pressure_altitude_independently() {
    let mut app = App::new();
    let preferred = DeviceId::new(1);
    let fallback = DeviceId::new(2);
    app.handle(external_device_order_input(vec![preferred, fallback]));
    let gnss = fix(1., 50., 6.);
    app.handle(gnss_input(
        SourceId::External(fallback),
        gnss_observation(gnss),
    ));
    let pressure_altitude = PressureAltitude::new(Length::from_meters(900.));
    app.handle(pressure_altitude_input(
        SourceId::External(preferred),
        1.,
        pressure_altitude,
    ));

    let snapshot = app.snapshot().flight;
    assert_eq!(snapshot.gnss, gnss.current_data());
    assert_eq!(snapshot.pressure_altitude, current(pressure_altitude));
}

#[test]
fn app_selects_pressure_altitude_by_external_device_order() {
    let mut app = App::new();
    let preferred = DeviceId::new(7);
    let fallback = DeviceId::new(8);
    app.handle(external_device_order_input(vec![preferred, fallback]));

    let internal = PressureAltitude::new(Length::from_meters(975.));
    let update = app.handle(pressure_altitude_input(SourceId::Internal, 1., internal));
    assert_eq!(
        update.changes,
        vec![Change::Flight(FlightChange::PressureAltitude(current(
            internal
        )))]
    );

    let fallback_altitude = PressureAltitude::new(Length::from_meters(950.));
    let update = app.handle(pressure_altitude_input(
        SourceId::External(fallback),
        2.,
        fallback_altitude,
    ));
    assert_eq!(
        update.changes,
        vec![Change::Flight(FlightChange::PressureAltitude(current(
            fallback_altitude
        )))]
    );

    let preferred_altitude = PressureAltitude::new(Length::from_meters(900.));
    let update = app.handle(pressure_altitude_input(
        SourceId::External(preferred),
        3.,
        preferred_altitude,
    ));
    assert_eq!(
        update.changes,
        vec![Change::Flight(FlightChange::PressureAltitude(current(
            preferred_altitude
        )))]
    );

    let stale = PressureAltitude::new(Length::from_meters(850.));
    let update = app.handle(pressure_altitude_input(
        SourceId::External(preferred),
        2.5,
        stale,
    ));
    assert!(update.changes.is_empty());
    assert_eq!(
        app.snapshot().flight.pressure_altitude,
        current(preferred_altitude)
    );

    let newer_fallback = PressureAltitude::new(Length::from_meters(1000.));
    let update = app.handle(pressure_altitude_input(
        SourceId::External(fallback),
        4.,
        newer_fallback,
    ));
    assert!(update.changes.is_empty());
    assert_eq!(
        app.snapshot().flight.pressure_altitude,
        current(preferred_altitude)
    );

    let update = app.handle(external_device_order_input(vec![fallback, preferred]));
    assert_eq!(
        update.changes,
        vec![Change::Flight(FlightChange::PressureAltitude(current(
            newer_fallback
        )))]
    );
    assert_eq!(
        app.snapshot().flight.pressure_altitude,
        current(newer_fallback)
    );
}

#[test]
fn flight_signals_expire_and_recover_at_exact_deadlines() {
    let mut app = App::new();
    let preferred = DeviceId::new(1);
    let fallback = DeviceId::new(2);
    let preferred_source = SourceId::External(preferred);
    let fallback_source = SourceId::External(fallback);
    app.handle(external_device_order_input(vec![preferred, fallback]));

    let fallback_fix = fix(1., 51., 7.);
    app.handle(gnss_input(fallback_source, gnss_observation(fallback_fix)));
    let preferred_fix = fix(0., 50., 6.);
    app.handle(gnss_input(
        preferred_source,
        gnss_observation(preferred_fix),
    ));
    let pressure_altitude = PressureAltitude::new(Length::from_meters(900.));
    app.handle(pressure_altitude_input(
        preferred_source,
        1.,
        pressure_altitude,
    ));

    let update = app.handle(Input::Clock {
        clock_time: at(2.999),
    });
    assert!(update.changes.is_empty());
    assert_some_eq!(update.next_deadline, at(3.));
    assert_eq!(app.snapshot().flight.gnss, preferred_fix.current_data());
    assert_eq!(
        app.snapshot().flight.pressure_altitude,
        current(pressure_altitude)
    );

    let update = app.handle(Input::Clock { clock_time: at(3.) });
    assert_eq!(
        update.changes,
        vec![Change::Flight(FlightChange::Gnss(
            fallback_fix.current_data()
        ))]
    );
    assert_some_eq!(update.next_deadline, at(4.));
    assert_eq!(app.snapshot().flight.gnss, fallback_fix.current_data());
    assert_eq!(
        app.snapshot().flight.pressure_altitude,
        current(pressure_altitude)
    );

    let update = app.handle(Input::Clock { clock_time: at(4.) });
    assert_eq!(
        update.changes,
        vec![
            Change::Flight(FlightChange::Gnss(fallback_fix.last_known_data())),
            Change::Flight(FlightChange::PressureAltitude(last_known(
                pressure_altitude
            ))),
        ]
    );
    assert_none!(update.next_deadline);

    let recovered_fix = fix(4., 52., 8.);
    let update = app.handle(gnss_input(
        preferred_source,
        gnss_observation(recovered_fix),
    ));
    assert_eq!(
        update.changes,
        vec![Change::Flight(FlightChange::Gnss(
            recovered_fix.current_data()
        ))]
    );
    assert_eq!(app.snapshot().flight.gnss, recovered_fix.current_data());

    let recovered_pressure_altitude = PressureAltitude::new(Length::from_meters(875.));
    let update = app.handle(pressure_altitude_input(
        preferred_source,
        4.,
        recovered_pressure_altitude,
    ));
    assert_eq!(
        update.changes,
        vec![Change::Flight(FlightChange::PressureAltitude(current(
            recovered_pressure_altitude
        )))]
    );
}

#[test]
fn already_stale_observations_are_published_as_last_known() {
    let mut app = App::new();
    app.handle(Input::Clock {
        clock_time: at(10.),
    });
    let gnss = fix(1., 50., 6.);

    let update = app.handle(gnss_input(SourceId::Internal, gnss_observation(gnss)));

    assert_eq!(
        update.changes,
        vec![Change::Flight(FlightChange::Gnss(gnss.last_known_data()))]
    );
    assert_eq!(app.snapshot().flight.gnss, gnss.last_known_data());

    let pressure_altitude = PressureAltitude::new(Length::from_meters(900.));
    let update = app.handle(pressure_altitude_input(
        SourceId::Internal,
        1.,
        pressure_altitude,
    ));

    assert_eq!(
        update.changes,
        vec![Change::Flight(FlightChange::PressureAltitude(last_known(
            pressure_altitude
        )))]
    );
    assert_eq!(
        app.snapshot().flight.pressure_altitude,
        last_known(pressure_altitude)
    );
}

#[test]
fn a_stale_selected_source_can_update_its_last_known_values() {
    let mut app = App::new();
    let initial_gnss = fix(1., 50., 6.);
    app.handle(gnss_input(
        SourceId::Internal,
        gnss_observation(initial_gnss),
    ));
    let initial_pressure_altitude = PressureAltitude::new(Length::from_meters(900.));
    app.handle(pressure_altitude_input(
        SourceId::Internal,
        1.,
        initial_pressure_altitude,
    ));
    app.handle(Input::Clock {
        clock_time: at(10.),
    });

    let newer_gnss = fix(5., 51., 7.);
    let update = app.handle(gnss_input(SourceId::Internal, gnss_observation(newer_gnss)));
    assert_eq!(
        update.changes,
        vec![Change::Flight(FlightChange::Gnss(
            newer_gnss.last_known_data()
        ))]
    );

    let newer_pressure_altitude = PressureAltitude::new(Length::from_meters(950.));
    let update = app.handle(pressure_altitude_input(
        SourceId::Internal,
        5.,
        newer_pressure_altitude,
    ));
    assert_eq!(
        update.changes,
        vec![Change::Flight(FlightChange::PressureAltitude(last_known(
            newer_pressure_altitude
        )))]
    );
}

#[test]
fn a_stale_preferred_source_does_not_displace_the_selected_source() {
    let mut app = App::new();
    let preferred = DeviceId::new(1);
    let fallback = DeviceId::new(2);
    let preferred_source = SourceId::External(preferred);
    let fallback_source = SourceId::External(fallback);
    app.handle(external_device_order_input(vec![preferred, fallback]));
    app.handle(gnss_input(
        preferred_source,
        gnss_observation(fix(1., 50., 6.)),
    ));
    let fallback_fix = fix(2., 51., 7.);
    app.handle(gnss_input(fallback_source, gnss_observation(fallback_fix)));
    app.handle(Input::Clock { clock_time: at(4.) });
    app.handle(Input::Clock {
        clock_time: at(10.),
    });

    let update = app.handle(gnss_input(
        preferred_source,
        gnss_observation(fix(6., 52., 8.)),
    ));

    assert!(update.changes.is_empty());
    assert_eq!(app.snapshot().flight.gnss, fallback_fix.last_known_data());
}

#[test]
fn a_fresh_replacement_at_expiry_does_not_publish_a_stale_transition() {
    let mut app = App::new();
    let external = DeviceId::new(1);
    app.handle(external_device_order_input(vec![external]));
    app.handle(gnss_input(
        SourceId::Internal,
        gnss_observation(fix(1., 50., 6.)),
    ));
    let external_fix = fix(4., 51., 7.);

    let update = app.handle(gnss_input(
        SourceId::External(external),
        gnss_observation(external_fix),
    ));

    assert_eq!(
        update.changes,
        vec![Change::Flight(FlightChange::Gnss(
            external_fix.current_data()
        ))]
    );
    assert_some_eq!(update.next_deadline, at(7.));
}

#[test]
fn app_retains_gnss_components_per_source() {
    let mut app = App::new();
    let device_a = DeviceId::new(1);
    let device_b = DeviceId::new(2);
    let external_a = SourceId::External(device_a);
    let external_b = SourceId::External(device_b);
    app.handle(external_device_order_input(vec![device_a, device_b]));
    let a_initial = Observation::new(
        at(2.),
        GnssUpdate {
            position: LatLon::from_degrees(50., 6.),
            altitude: Some(MslAltitude::new(Length::from_meters(1000.))),
            track: Some(Angle::from_degrees(10.)),
            ground_speed: Some(Speed::from_meters_per_second(20.)),
        },
    );
    let first_update = app.handle(gnss_input(external_a, a_initial));
    let first_job = assert_some!(compute_job(&first_update)).clone();

    let b_initial = Observation::new(
        at(1.),
        GnssUpdate {
            position: LatLon::from_degrees(51., 7.),
            altitude: Some(MslAltitude::new(Length::from_meters(2000.))),
            track: Some(Angle::from_degrees(20.)),
            ground_speed: Some(Speed::from_meters_per_second(30.)),
        },
    );
    let update = app.handle(gnss_input(external_b, b_initial));
    assert!(update.changes.is_empty());
    let update = app.handle(Input::ComputeResult(first_job.run()));
    assert_some_eq!(
        update.next_deadline,
        at(5.),
        "the unselected fix did not change the selected source's freshness deadline"
    );

    let a_partial = Observation::new(
        at(3.),
        GnssUpdate {
            position: LatLon::from_degrees(50.1, 6.1),
            altitude: None,
            track: None,
            ground_speed: None,
        },
    );
    let expected_a = GnssData {
        position: current(a_partial.value.position),
        altitude: current(assert_some!(a_initial.value.altitude)),
        track: current(assert_some!(a_initial.value.track)),
        ground_speed: current(assert_some!(a_initial.value.ground_speed)),
    };
    let update = app.handle(gnss_input(external_a, a_partial));
    assert_eq!(
        update.changes,
        vec![Change::Flight(FlightChange::Gnss(expected_a))]
    );
    assert_eq!(app.snapshot().flight.gnss, expected_a);

    let stale_a = Observation::new(
        at(2.5),
        GnssUpdate {
            position: LatLon::from_degrees(49., 5.),
            altitude: Some(MslAltitude::new(Length::from_meters(1500.))),
            track: None,
            ground_speed: None,
        },
    );
    let update = app.handle(gnss_input(external_a, stale_a));
    assert!(update.changes.is_empty());
    assert_eq!(app.snapshot().flight.gnss, expected_a);

    let expected_b_initial = GnssData {
        position: current(b_initial.value.position),
        altitude: current(assert_some!(b_initial.value.altitude)),
        track: current(assert_some!(b_initial.value.track)),
        ground_speed: current(assert_some!(b_initial.value.ground_speed)),
    };
    let update = app.handle(external_device_order_input(vec![device_b, device_a]));
    assert_eq!(
        update.changes,
        vec![Change::Flight(FlightChange::Gnss(expected_b_initial))]
    );
    assert!(update.effects.is_empty());

    let b_partial = Observation::new(
        at(4.),
        GnssUpdate {
            position: LatLon::from_degrees(51.1, 7.1),
            altitude: None,
            track: None,
            ground_speed: None,
        },
    );
    let expected_b = GnssData {
        position: current(b_partial.value.position),
        altitude: last_known(assert_some!(b_initial.value.altitude)),
        track: last_known(assert_some!(b_initial.value.track)),
        ground_speed: last_known(assert_some!(b_initial.value.ground_speed)),
    };
    let update = app.handle(gnss_input(external_b, b_partial));
    assert_eq!(
        update.changes,
        vec![Change::Flight(FlightChange::Gnss(expected_b))]
    );
    assert_eq!(app.snapshot().flight.gnss, expected_b);

    let update = app.handle(Input::Clock { clock_time: at(7.) });
    let job = assert_some!(compute_job(&update));
    let ComputeJob::Flight(FlightComputeJob::TraceStats { fixes, .. }) = job;
    assert_eq!(
        fixes.iter().map(|fix| fix.position).collect::<Vec<_>>(),
        vec![
            a_initial.value.position,
            a_partial.value.position,
            b_partial.value.position
        ]
    );
}

#[test]
fn omitted_gnss_components_expire_independently() {
    let mut app = App::new();
    let altitude = MslAltitude::new(Length::from_meters(1000.));
    let track = Angle::from_degrees(45.);
    let ground_speed = Speed::from_meters_per_second(30.);
    app.handle(gnss_input(
        SourceId::Internal,
        Observation::new(
            at(0.),
            GnssUpdate {
                position: LatLon::from_degrees(50., 6.),
                altitude: Some(altitude),
                track: Some(track),
                ground_speed: Some(ground_speed),
            },
        ),
    ));
    let pressure_altitude = PressureAltitude::new(Length::from_meters(900.));
    app.handle(pressure_altitude_input(
        SourceId::Internal,
        1.,
        pressure_altitude,
    ));
    let position = LatLon::from_degrees(50.1, 6.1);

    let update = app.handle(gnss_input(
        SourceId::Internal,
        Observation::new(
            at(2.),
            GnssUpdate {
                position,
                altitude: None,
                track: None,
                ground_speed: None,
            },
        ),
    ));

    let current_gnss = GnssData {
        position: current(position),
        altitude: current(altitude),
        track: current(track),
        ground_speed: current(ground_speed),
    };
    assert_eq!(
        update.changes,
        vec![Change::Flight(FlightChange::Gnss(current_gnss))]
    );
    assert_some_eq!(update.next_deadline, at(3.));

    let update = app.handle(Input::Clock {
        clock_time: at(2.999),
    });
    assert!(update.changes.is_empty());

    let stale_companions = GnssData {
        position: current(position),
        altitude: last_known(altitude),
        track: last_known(track),
        ground_speed: last_known(ground_speed),
    };
    let update = app.handle(Input::Clock { clock_time: at(3.) });
    assert_eq!(
        update.changes,
        vec![Change::Flight(FlightChange::Gnss(stale_companions))]
    );
    assert_some_eq!(update.next_deadline, at(4.));
    assert_eq!(app.snapshot().flight.gnss, stale_companions);

    let update = app.handle(Input::Clock { clock_time: at(4.) });
    assert_eq!(
        update.changes,
        vec![Change::Flight(FlightChange::PressureAltitude(last_known(
            pressure_altitude
        )))]
    );
    assert_some_eq!(update.next_deadline, at(5.));
}

#[test]
fn stale_gnss_altitude_is_not_added_to_the_trace() {
    let mut app = App::with_config(updraft_core::AppConfig {
        flight: FlightConfig {
            trace_stats_interval: Duration::ZERO,
        },
    });
    let altitude = MslAltitude::new(Length::from_meters(1000.));
    let first = app.handle(gnss_input(
        SourceId::Internal,
        Observation::new(
            at(0.),
            GnssUpdate {
                position: LatLon::from_degrees(50., 6.),
                altitude: Some(altitude),
                track: None,
                ground_speed: None,
            },
        ),
    ));
    let first_job = assert_some!(compute_job(&first)).clone();
    app.handle(Input::ComputeResult(first_job.run()));

    let update = app.handle(gnss_input(
        SourceId::Internal,
        Observation::new(
            at(4.),
            GnssUpdate {
                position: LatLon::from_degrees(50.1, 6.1),
                altitude: None,
                track: None,
                ground_speed: None,
            },
        ),
    ));

    let ComputeJob::Flight(FlightComputeJob::TraceStats { fixes, .. }) =
        assert_some!(compute_job(&update));
    assert_some_eq!(fixes[0].altitude, altitude);
    assert_none!(fixes[1].altitude);
}

#[test]
fn app_routes_flight_protocol_through_the_flight_domain() {
    let mut app = App::new();
    let gnss = fix(0., 50., 6.);

    let update = app.handle(Input::Flight(FlightInput::Gnss(Sourced::simulator(
        gnss_observation(gnss),
    ))));

    assert_eq!(
        update.changes,
        vec![Change::Flight(FlightChange::Gnss(gnss.current_data()))]
    );
    assert_eq!(
        app.snapshot().flight,
        FlightSnapshot {
            gnss: gnss.current_data(),
            pressure_altitude: Availability::Unavailable,
            trace_stats: None,
        }
    );
}

fn at(seconds: f64) -> Duration {
    Duration::from_secs_f64(seconds)
}

fn current<T>(value: T) -> Availability<T> {
    Availability::Current(value)
}

fn last_known<T>(value: T) -> Availability<T> {
    Availability::LastKnown(value)
}

#[derive(Clone, Copy)]
struct GnssFixture {
    observation: Observation<GnssUpdate>,
}

impl GnssFixture {
    fn current_data(self) -> GnssData {
        GnssData {
            position: Availability::Current(self.observation.value.position),
            altitude: self
                .observation
                .value
                .altitude
                .map_or(Availability::Unavailable, Availability::Current),
            track: self
                .observation
                .value
                .track
                .map_or(Availability::Unavailable, Availability::Current),
            ground_speed: self
                .observation
                .value
                .ground_speed
                .map_or(Availability::Unavailable, Availability::Current),
        }
    }

    fn last_known_data(self) -> GnssData {
        GnssData {
            position: Availability::LastKnown(self.observation.value.position),
            altitude: self
                .observation
                .value
                .altitude
                .map_or(Availability::Unavailable, Availability::LastKnown),
            track: self
                .observation
                .value
                .track
                .map_or(Availability::Unavailable, Availability::LastKnown),
            ground_speed: self
                .observation
                .value
                .ground_speed
                .map_or(Availability::Unavailable, Availability::LastKnown),
        }
    }
}

fn fix(seconds: f64, latitude: f64, longitude: f64) -> GnssFixture {
    GnssFixture {
        observation: Observation::new(
            at(seconds),
            GnssUpdate {
                position: LatLon::from_degrees(latitude, longitude),
                altitude: Some(MslAltitude::new(Length::from_meters(1000.))),
                track: None,
                ground_speed: None,
            },
        ),
    }
}

fn gnss_observation(gnss: GnssFixture) -> Observation<GnssUpdate> {
    gnss.observation
}

fn gnss_input(source: SourceId, observation: Observation<GnssUpdate>) -> Input {
    Input::Flight(FlightInput::Gnss(Sourced::new(source, observation)))
}

fn external_device_order_input(order: Vec<DeviceId>) -> Input {
    Input::Flight(FlightInput::SetExternalDeviceOrder(order))
}

fn pressure_altitude_input(source: SourceId, seconds: f64, altitude: PressureAltitude) -> Input {
    Input::Flight(FlightInput::PressureAltitude(Sourced::new(
        source,
        Observation::new(at(seconds), altitude),
    )))
}

fn position_input(seconds: f64, latitude: f64, longitude: f64) -> Input {
    gnss_input(
        SourceId::Simulator,
        gnss_observation(fix(seconds, latitude, longitude)),
    )
}

fn clear_trace_input() -> Input {
    Input::Flight(FlightInput::ClearTrace)
}

/// Extracts the single compute job from an update, if any.
fn compute_job(update: &Update) -> Option<&ComputeJob> {
    match update.effects.as_slice() {
        [] => None,
        [Effect::Compute(job)] => Some(job),
        effects => panic!("unexpected effects: {effects:?}"),
    }
}

#[test]
fn trace_stats_compute_lifecycle() {
    let mut app = App::new();

    // The first fix updates the position and immediately starts a
    // trace-statistics job (nothing ran before, so no throttling).
    let update = app.handle(position_input(0., 50., 6.));
    assert_matches!(
        update.changes.as_slice(),
        [Change::Flight(FlightChange::Gnss(_))]
    );
    let job = assert_some!(compute_job(&update), "first fix starts a job").clone();
    let ComputeJob::Flight(FlightComputeJob::TraceStats {
        revision,
        ref fixes,
    }) = job;
    assert_eq!(fixes.len(), 1);
    // The job is running, and the selected position expires after three seconds.
    assert_some_eq!(update.next_deadline, at(3.));

    // A second fix while the job runs only marks the slot pending.
    let update = app.handle(position_input(0.2, 50.01, 6.));
    assert_matches!(
        update.changes.as_slice(),
        [Change::Flight(FlightChange::Gnss(_))]
    );
    assert_eq!(update.effects, vec![]);
    assert_some_eq!(update.next_deadline, at(3.2));

    // The worker result applies and schedules the next start five
    // seconds after the previous one.
    let result = job.clone().run();
    let ComputeResult::Flight(FlightComputeResult::TraceStats { stats, .. }) = result;
    assert_eq!(stats.fix_count, 1);
    let update = app.handle(Input::ComputeResult(result));
    assert_eq!(
        update.changes,
        vec![Change::Flight(FlightChange::TraceStats(Some(stats)))],
        "current-revision result becomes a change"
    );
    assert_some_eq!(update.next_deadline, at(3.2));
    assert_some_eq!(app.query(GetTraceStats), stats);

    // The clock reaching the deadline starts the next job over both fixes.
    let update = app.handle(Input::Clock { clock_time: at(5.) });
    let job = assert_some!(compute_job(&update), "timer starts the next job").clone();
    let ComputeJob::Flight(FlightComputeJob::TraceStats {
        revision: second_revision,
        ref fixes,
    }) = job;
    assert_eq!(revision, second_revision, "no invalidation happened");
    assert_eq!(fixes.len(), 2);

    // Clearing the trace invalidates the in-flight job and clears the
    // published statistics.
    let update = app.handle(clear_trace_input());
    assert_eq!(
        update.changes,
        vec![Change::Flight(FlightChange::TraceStats(None))]
    );
    assert_none!(update.next_deadline);

    // The stale result is rejected: no change, state stays cleared.
    let update = app.handle(Input::ComputeResult(job.run()));
    assert_eq!(update.changes, vec![]);
    assert_none!(app.query(GetTraceStats));

    // A fresh fix starts over under the new revision, throttled to five
    // seconds after the previous start.
    let update = app.handle(position_input(5.5, 51., 6.));
    assert_some_eq!(update.next_deadline, at(8.5));
    let update = app.handle(Input::Clock {
        clock_time: at(10.),
    });
    let job = assert_some!(compute_job(&update), "job starts under the new revision");
    let ComputeJob::Flight(FlightComputeJob::TraceStats {
        revision: new_revision,
        fixes,
    }) = job;
    assert_ne!(revision, *new_revision);
    assert_eq!(fixes.len(), 1);
}

#[test]
fn stats_interval_is_configurable() {
    let mut app = App::with_config(updraft_core::AppConfig {
        flight: FlightConfig {
            trace_stats_interval: Duration::from_millis(100),
        },
    });

    let update = app.handle(position_input(0., 50., 6.));
    let job = assert_some!(compute_job(&update), "first fix starts a job").clone();
    app.handle(position_input(0.02, 50.01, 6.));

    // The result schedules the next start at the configured interval
    // instead of the five-second default.
    let update = app.handle(Input::ComputeResult(job.run()));
    assert_some_eq!(update.next_deadline, at(0.1));
}

#[test]
fn compute_failure_frees_the_slot() {
    let mut app = App::new();

    let update = app.handle(position_input(0., 50., 6.));
    let job = assert_some!(compute_job(&update), "first fix starts a job").clone();

    // More work arrives while the job runs, then the job fails.
    app.handle(position_input(0.5, 50.01, 6.));
    let update = app.handle(Input::ComputeFailed(ComputeFailure {
        kind: ComputeKind::Flight(FlightComputeKind::TraceStats),
        revision: job.revision(),
        message: "worker panicked".into(),
    }));

    // No change is published, but the pending request reschedules.
    assert_eq!(update.changes, vec![]);
    assert_some_eq!(update.next_deadline, at(3.5));
    let update = app.handle(Input::Clock { clock_time: at(5.) });
    assert_some!(compute_job(&update), "the slot accepts a new job");
}

#[test]
fn fix_after_the_interval_starts_a_job_without_waiting() {
    let mut app = App::new();

    // The first fix starts and completes a job, leaving the slot idle with
    // its last start five seconds before the next fix.
    let update = app.handle(position_input(0., 50., 6.));
    let job = assert_some!(compute_job(&update)).clone();
    app.handle(Input::ComputeResult(job.run()));

    // A fix arriving after the throttle interval has already elapsed starts
    // the next job in the same handle() call, with no throttle wait.
    let update = app.handle(position_input(10., 50.1, 6.));
    assert_some!(compute_job(&update), "the job starts immediately");
    assert_some_eq!(update.next_deadline, at(13.));
}

#[test]
fn clearing_the_trace_cancels_a_pending_stats_timer() {
    let mut app = App::new();

    // Run one job to completion with a second fix pending, so the result
    // arms the next start as an unfired throttle timer.
    let update = app.handle(position_input(0., 50., 6.));
    let job = assert_some!(compute_job(&update)).clone();
    app.handle(position_input(0.2, 50.01, 6.));
    let update = app.handle(Input::ComputeResult(job.run()));
    assert_some_eq!(
        update.next_deadline,
        at(3.2),
        "freshness expires before the throttle timer"
    );

    // Clearing the trace before that timer fires must cancel it, not leave a
    // stale deadline that would wake the runtime for nothing.
    let update = app.handle(clear_trace_input());
    assert_eq!(
        update.changes,
        vec![Change::Flight(FlightChange::TraceStats(None))]
    );
    assert_some_eq!(update.next_deadline, at(3.2));
    let update = app.handle(Input::Clock {
        clock_time: at(3.2),
    });
    assert_none!(
        update.next_deadline,
        "the cancelled throttle timer does not remain after freshness expiry"
    );
}

#[test]
fn stale_result_frees_the_slot_for_new_revision_work() {
    let mut app = App::new();

    // Start a job, then clear the trace so the running job's revision is stale.
    let update = app.handle(position_input(0., 50., 6.));
    let job = assert_some!(compute_job(&update)).clone();
    app.handle(clear_trace_input());

    // New work arrives under the new revision while the stale job is still out.
    app.handle(position_input(0.5, 51., 6.));

    // The stale result publishes no change but still frees the slot, so the
    // pending new-revision request gets scheduled.
    let update = app.handle(Input::ComputeResult(job.run()));
    assert_eq!(update.changes, vec![]);
    assert_some_eq!(update.next_deadline, at(3.5));

    let update = app.handle(Input::Clock { clock_time: at(5.) });
    let job = assert_some!(compute_job(&update), "new-revision job starts");
    let ComputeJob::Flight(FlightComputeJob::TraceStats { fixes, .. }) = job;
    assert_eq!(fixes.len(), 1, "only the post-clear fix is included");
}

#[test]
fn snapshot_reflects_current_shared_state() {
    let mut app = App::new();
    assert_eq!(app.snapshot(), updraft_core::Snapshot::default());

    let update = app.handle(position_input(0., 50., 6.));
    let job = assert_some!(compute_job(&update)).clone();
    app.handle(Input::ComputeResult(job.run()));

    let snapshot = app.snapshot();
    assert_eq!(snapshot.flight.gnss, fix(0., 50., 6.).current_data());
    let stats = assert_some!(snapshot.flight.trace_stats, "stats are shared state");
    assert_eq!(stats.fix_count, 1);
}

#[test]
fn same_inputs_produce_same_updates() {
    let inputs = [
        position_input(0., 50., 6.),
        position_input(0.2, 50.01, 6.),
        Input::Clock { clock_time: at(1.) },
        clear_trace_input(),
        position_input(1.5, 50.02, 6.),
        Input::Clock {
            clock_time: at(2.5),
        },
    ];

    let run = || -> Vec<Update> {
        let mut app = App::new();
        inputs
            .iter()
            .cloned()
            .map(|input| app.handle(input))
            .collect()
    };
    assert_eq!(run(), run());
}
