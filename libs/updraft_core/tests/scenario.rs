//! Whole-flight scenario tests: a plain loop over `App::handle()` with no
//! async runtime, sleeps, or wall clock.

use claims::{assert_matches, assert_none, assert_some, assert_some_eq};
use std::time::Duration;
use updraft_core::device::DeviceId;
use updraft_core::flight::{
    FlightChange, FlightComputeJob, FlightComputeKind, FlightComputeResult, FlightConfig,
    FlightInput, FlightSnapshot, GetTraceStats, GnssUpdate, Observation, PositionFix, Sourced,
};
use updraft_core::{
    App, Change, ComputeFailure, ComputeJob, ComputeKind, ComputeResult, Effect, Input, Update,
};
use updraft_geo::LatLon;
use updraft_units::{Length, MslAltitude, PressureAltitude};

#[test]
fn app_publishes_latest_pressure_altitude() {
    let mut app = App::new();
    let first = Sourced::external(
        DeviceId::new(7),
        Observation::new(at(1.), PressureAltitude::new(Length::from_meters(950.))),
    );
    app.handle(Input::Flight(FlightInput::PressureAltitude(first)));
    let second = Sourced::internal(Observation::new(
        at(2.),
        PressureAltitude::new(Length::from_meters(975.)),
    ));
    let altitude = second.value.value;

    let update = app.handle(Input::Flight(FlightInput::PressureAltitude(second)));

    assert_eq!(
        update.changes,
        vec![Change::Flight(FlightChange::PressureAltitude(altitude))]
    );
    assert_some_eq!(app.snapshot().flight.pressure_altitude, altitude);
}

#[test]
fn app_routes_flight_protocol_through_the_flight_domain() {
    let mut app = App::new();
    let fix = fix(0., 50., 6.);

    let update = app.handle(Input::Flight(FlightInput::Gnss(Sourced::simulator(
        gnss_observation(fix),
    ))));

    assert_eq!(
        update.changes,
        vec![Change::Flight(FlightChange::Position(fix))]
    );
    assert_eq!(
        app.snapshot().flight,
        FlightSnapshot {
            position: Some(fix),
            pressure_altitude: None,
            trace_stats: None,
        }
    );
}

fn at(seconds: f64) -> Duration {
    Duration::from_secs_f64(seconds)
}

fn fix(seconds: f64, latitude: f64, longitude: f64) -> PositionFix {
    PositionFix {
        observed_at: at(seconds),
        position: LatLon::from_degrees(latitude, longitude),
        altitude: Some(MslAltitude::new(Length::from_meters(1000.))),
        track: None,
        ground_speed: None,
    }
}

fn gnss_observation(fix: PositionFix) -> Observation<GnssUpdate> {
    Observation::new(
        fix.observed_at,
        GnssUpdate {
            position: fix.position,
            altitude: fix.altitude,
            track: fix.track,
            ground_speed: fix.ground_speed,
        },
    )
}

fn position_input(seconds: f64, latitude: f64, longitude: f64) -> Input {
    Input::Flight(FlightInput::Gnss(Sourced::simulator(gnss_observation(
        fix(seconds, latitude, longitude),
    ))))
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
        [Change::Flight(FlightChange::Position(_))]
    );
    let job = assert_some!(compute_job(&update), "first fix starts a job").clone();
    let ComputeJob::Flight(FlightComputeJob::TraceStats {
        revision,
        ref fixes,
    }) = job;
    assert_eq!(fixes.len(), 1);
    // The job is running, nothing further is requested yet.
    assert_none!(update.next_deadline);

    // A second fix while the job runs only marks the slot pending.
    let update = app.handle(position_input(0.2, 50.01, 6.));
    assert_matches!(
        update.changes.as_slice(),
        [Change::Flight(FlightChange::Position(_))]
    );
    assert_eq!(update.effects, vec![]);
    assert_none!(update.next_deadline);

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
    assert_some_eq!(update.next_deadline, at(5.));
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
    assert_some_eq!(update.next_deadline, at(10.));
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
    assert_some_eq!(update.next_deadline, at(5.));
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
    assert_none!(update.next_deadline);
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
    assert_some_eq!(update.next_deadline, at(5.), "throttle timer is armed");

    // Clearing the trace before that timer fires must cancel it, not leave a
    // stale deadline that would wake the runtime for nothing.
    let update = app.handle(clear_trace_input());
    assert_eq!(
        update.changes,
        vec![Change::Flight(FlightChange::TraceStats(None))]
    );
    assert_none!(update.next_deadline);
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
    assert_some_eq!(update.next_deadline, at(5.));

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
    assert_some_eq!(snapshot.flight.position, fix(0., 50., 6.));
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
