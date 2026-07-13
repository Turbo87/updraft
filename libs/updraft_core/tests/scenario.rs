//! Whole-flight scenario tests run as a plain loop over `App::handle()`.

use std::time::Duration;

use updraft_core::flight::{
    Change as FlightChange, Command, ComputeJob as FlightComputeJob, GetPosition, MslAltitude,
    Observation, PositionFix,
};
use updraft_core::{App, Change, ComputeJob, ComputeResult, Effect, Input};
use updraft_geo::LatLon;
use updraft_units::Length;

fn fix(seconds: f64, latitude: f64, longitude: f64) -> PositionFix {
    PositionFix {
        observed_at: Duration::from_secs_f64(seconds),
        position: LatLon::from_degrees(latitude, longitude),
        altitude: Some(MslAltitude::new(Length::from_meters(1_000.))),
        track: None,
        ground_speed: None,
    }
}

fn position_input(seconds: f64, latitude: f64, longitude: f64) -> Input {
    Input::Flight(updraft_core::flight::Input::Observation(
        Observation::Position(fix(seconds, latitude, longitude)),
    ))
}

fn clear_trace_input() -> Input {
    Input::Flight(updraft_core::flight::Input::Command(Command::ClearTrace))
}

fn compute_job(effects: &[Effect]) -> Option<&ComputeJob> {
    match effects {
        [] => None,
        [Effect::Compute(job)] => Some(job),
        effects => panic!("unexpected effects: {effects:?}"),
    }
}

#[test]
fn app_routes_position_state_through_the_flight_domain() {
    let mut app = App::new();
    let position = fix(1., 50., 6.);

    let update = app.handle(position_input(1., 50., 6.));

    assert_eq!(
        update.changes,
        vec![Change::Flight(FlightChange::Position(position))]
    );
    assert_eq!(app.query(GetPosition), Some(position));
    assert_eq!(app.snapshot().flight.position, Some(position));
}

#[test]
fn trace_stats_compute_lifecycle() {
    let mut app = App::new();

    let first = app.handle(position_input(0., 50., 6.));
    let first_job = compute_job(&first.effects).unwrap().clone();
    let ComputeJob::Flight(FlightComputeJob::TraceStats { fixes, .. }) = &first_job;
    assert_eq!(fixes.len(), 1);

    let second = app.handle(position_input(0.2, 50.01, 6.));
    assert_eq!(second.effects, vec![]);

    let first_result = first_job.run();
    let update = app.handle(Input::ComputeResult(first_result));
    assert!(matches!(
        update.changes.as_slice(),
        [Change::Flight(FlightChange::TraceStats(Some(_)))]
    ));
    let second_job = compute_job(&update.effects).unwrap().clone();
    let ComputeJob::Flight(FlightComputeJob::TraceStats { fixes, .. }) = &second_job;
    assert_eq!(fixes.len(), 2);

    let update = app.handle(clear_trace_input());
    assert_eq!(
        update.changes,
        vec![Change::Flight(FlightChange::TraceStats(None))]
    );
    app.handle(position_input(0.5, 51., 6.));

    let update = app.handle(Input::ComputeResult(second_job.run()));
    assert_eq!(update.changes, vec![]);
    let fresh_job = compute_job(&update.effects).unwrap();
    let ComputeJob::Flight(FlightComputeJob::TraceStats { fixes, .. }) = fresh_job;
    assert_eq!(fixes.len(), 1);
}

#[test]
fn compute_jobs_are_pure() {
    let mut app = App::new();
    let update = app.handle(position_input(0., 50., 6.));
    let job = compute_job(&update.effects).unwrap().clone();

    assert_eq!(job.clone().run(), job.run());
}

#[test]
fn snapshot_reflects_completed_trace_statistics() {
    let mut app = App::new();
    let update = app.handle(position_input(0., 50., 6.));
    let result: ComputeResult = compute_job(&update.effects).unwrap().clone().run();
    app.handle(Input::ComputeResult(result));

    assert_eq!(app.snapshot().flight.trace_stats.unwrap().fix_count, 1);
}
