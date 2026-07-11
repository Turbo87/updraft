//! Whole-flight scenario tests: a plain loop over `App::handle()` with no
//! async runtime, sleeps, or wall clock.

use updraft_core::{
    App, Change, Command, ComputeFailure, ComputeJob, ComputeKind, ComputeResult, Effect, Input,
    MonotonicTime, Observation, PositionFix, Query, QueryResult, Update,
};
use updraft_geo::LatLon;
use updraft_units::Length;

fn at(seconds: f64) -> MonotonicTime {
    MonotonicTime::from_micros((seconds * 1e6) as u64)
}

fn fix(seconds: f64, latitude: f64, longitude: f64) -> PositionFix {
    PositionFix {
        observed_at: at(seconds),
        position: LatLon::from_degrees(latitude, longitude),
        altitude: Some(Length::from_meters(1000.)),
        track: None,
        ground_speed: None,
    }
}

fn position_input(seconds: f64, latitude: f64, longitude: f64) -> Input {
    Input::Observation(Observation::Position(fix(seconds, latitude, longitude)))
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
    assert_eq!(update.changes.len(), 1);
    assert!(matches!(update.changes[0], Change::Position(_)));
    let job = compute_job(&update)
        .expect("first fix starts a job")
        .clone();
    let ComputeJob::TraceStats { epoch, ref fixes } = job;
    assert_eq!(fixes.len(), 1);
    // The job is running, nothing further is requested yet.
    assert_eq!(update.next_deadline, None);

    // A second fix while the job runs only marks the slot pending.
    let update = app.handle(position_input(0.2, 50.01, 6.));
    assert!(matches!(update.changes.as_slice(), [Change::Position(_)]));
    assert_eq!(update.effects, vec![]);
    assert_eq!(update.next_deadline, None);

    // The worker result applies and schedules the next start five
    // seconds after the previous one.
    let result = job.clone().run();
    let ComputeResult::TraceStats { stats, .. } = result;
    assert_eq!(stats.fix_count, 1);
    let update = app.handle(Input::ComputeResult(result));
    assert_eq!(
        update.changes,
        vec![Change::TraceStats(Some(stats))],
        "current-epoch result becomes a change"
    );
    assert_eq!(update.next_deadline, Some(at(5.)));
    assert_eq!(
        app.query(Query::TraceStats),
        QueryResult::TraceStats(Some(stats))
    );

    // The clock reaching the deadline starts the next job over both fixes.
    let update = app.handle(Input::Clock { now: at(5.) });
    let job = compute_job(&update)
        .expect("timer starts the next job")
        .clone();
    let ComputeJob::TraceStats {
        epoch: second_epoch,
        ref fixes,
    } = job;
    assert_eq!(epoch, second_epoch, "no invalidation happened");
    assert_eq!(fixes.len(), 2);

    // Clearing the trace invalidates the in-flight job and clears the
    // published statistics.
    let update = app.handle(Input::Command(Command::ClearTrace));
    assert_eq!(update.changes, vec![Change::TraceStats(None)]);
    assert_eq!(update.next_deadline, None);

    // The stale result is rejected: no change, state stays cleared.
    let update = app.handle(Input::ComputeResult(job.run()));
    assert_eq!(update.changes, vec![]);
    assert_eq!(app.query(Query::TraceStats), QueryResult::TraceStats(None));

    // A fresh fix starts over under the new epoch, throttled to five
    // seconds after the previous start.
    let update = app.handle(position_input(5.5, 51., 6.));
    assert_eq!(update.next_deadline, Some(at(10.)));
    let update = app.handle(Input::Clock { now: at(10.) });
    let job = compute_job(&update).expect("job starts under the new epoch");
    let ComputeJob::TraceStats {
        epoch: new_epoch,
        fixes,
    } = job;
    assert_ne!(epoch, *new_epoch);
    assert_eq!(fixes.len(), 1);
}

#[test]
fn stats_interval_is_configurable() {
    let mut app = App::with_config(updraft_core::AppConfig {
        trace_stats_interval: std::time::Duration::from_millis(100),
    });

    let update = app.handle(position_input(0., 50., 6.));
    let job = compute_job(&update)
        .expect("first fix starts a job")
        .clone();
    app.handle(position_input(0.02, 50.01, 6.));

    // The result schedules the next start at the configured interval
    // instead of the five-second default.
    let update = app.handle(Input::ComputeResult(job.run()));
    assert_eq!(update.next_deadline, Some(at(0.1)));
}

#[test]
fn compute_failure_frees_the_slot() {
    let mut app = App::new();

    let update = app.handle(position_input(0., 50., 6.));
    let job = compute_job(&update)
        .expect("first fix starts a job")
        .clone();

    // More work arrives while the job runs, then the job fails.
    app.handle(position_input(0.5, 50.01, 6.));
    let update = app.handle(Input::ComputeFailed(ComputeFailure {
        kind: ComputeKind::TraceStats,
        epoch: job.epoch(),
        message: "worker panicked".into(),
    }));

    // No change is published, but the pending request reschedules.
    assert_eq!(update.changes, vec![]);
    assert_eq!(update.next_deadline, Some(at(5.)));
    let update = app.handle(Input::Clock { now: at(5.) });
    assert!(compute_job(&update).is_some(), "the slot accepts a new job");
}

#[test]
fn fix_after_the_interval_starts_a_job_without_waiting() {
    let mut app = App::new();

    // The first fix starts and completes a job, leaving the slot idle with
    // its last start five seconds before the next fix.
    let update = app.handle(position_input(0., 50., 6.));
    let job = compute_job(&update).unwrap().clone();
    app.handle(Input::ComputeResult(job.run()));

    // A fix arriving after the throttle interval has already elapsed starts
    // the next job in the same handle() call, with no throttle wait.
    let update = app.handle(position_input(10., 50.1, 6.));
    assert!(compute_job(&update).is_some(), "the job starts immediately");
    assert_eq!(update.next_deadline, None);
}

#[test]
fn clearing_the_trace_cancels_a_pending_stats_timer() {
    let mut app = App::new();

    // Run one job to completion with a second fix pending, so the result
    // arms the next start as an unfired throttle timer.
    let update = app.handle(position_input(0., 50., 6.));
    let job = compute_job(&update).unwrap().clone();
    app.handle(position_input(0.2, 50.01, 6.));
    let update = app.handle(Input::ComputeResult(job.run()));
    assert_eq!(
        update.next_deadline,
        Some(at(5.)),
        "throttle timer is armed"
    );

    // Clearing the trace before that timer fires must cancel it, not leave a
    // stale deadline that would wake the runtime for nothing.
    let update = app.handle(Input::Command(Command::ClearTrace));
    assert_eq!(update.changes, vec![Change::TraceStats(None)]);
    assert_eq!(update.next_deadline, None);
}

#[test]
fn stale_result_frees_the_slot_for_new_epoch_work() {
    let mut app = App::new();

    // Start a job, then clear the trace so the running job's epoch is stale.
    let update = app.handle(position_input(0., 50., 6.));
    let job = compute_job(&update).unwrap().clone();
    app.handle(Input::Command(Command::ClearTrace));

    // New work arrives under the new epoch while the stale job is still out.
    app.handle(position_input(0.5, 51., 6.));

    // The stale result publishes no change but still frees the slot, so the
    // pending new-epoch request gets scheduled.
    let update = app.handle(Input::ComputeResult(job.run()));
    assert_eq!(update.changes, vec![]);
    assert_eq!(update.next_deadline, Some(at(5.)));

    let update = app.handle(Input::Clock { now: at(5.) });
    let job = compute_job(&update).expect("new-epoch job starts");
    let ComputeJob::TraceStats { fixes, .. } = job;
    assert_eq!(fixes.len(), 1, "only the post-clear fix is included");
}

#[test]
fn snapshot_reflects_current_shared_state() {
    let mut app = App::new();
    assert_eq!(app.snapshot(), updraft_core::Snapshot::default());

    let update = app.handle(position_input(0., 50., 6.));
    let job = compute_job(&update).unwrap().clone();
    app.handle(Input::ComputeResult(job.run()));

    let snapshot = app.snapshot();
    assert_eq!(snapshot.position, Some(fix(0., 50., 6.)));
    let stats = snapshot.trace_stats.expect("stats are shared state");
    assert_eq!(stats.fix_count, 1);
}

#[test]
fn same_inputs_produce_same_updates() {
    let inputs = [
        position_input(0., 50., 6.),
        position_input(0.2, 50.01, 6.),
        Input::Clock { now: at(1.) },
        Input::Command(Command::ClearTrace),
        position_input(1.5, 50.02, 6.),
        Input::Clock { now: at(2.5) },
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

#[test]
fn inputs_round_trip_through_json() {
    let inputs = [
        Input::Clock { now: at(1.) },
        Input::Command(Command::ClearTrace),
        Input::ComputeResult(
            ComputeJob::TraceStats {
                epoch: updraft_core::Epoch::default(),
                fixes: vec![fix(0., 50., 6.)],
            }
            .run(),
        ),
        Input::ComputeFailed(ComputeFailure {
            kind: ComputeKind::TraceStats,
            epoch: updraft_core::Epoch::default(),
            message: "boom".into(),
        }),
    ];
    for input in inputs {
        let json = serde_json::to_string(&input).unwrap();
        let back: Input = serde_json::from_str(&json).unwrap();
        assert_eq!(input, back, "{json}");
    }

    // Angles serialize as degrees but are stored as radians, so a fix
    // round-trips within a degree ulp rather than bit-exactly.
    let input = position_input(0., 50., 6.);
    let json = serde_json::to_string(&input).unwrap();
    let back: Input = serde_json::from_str(&json).unwrap();
    let (
        Input::Observation(Observation::Position(sent)),
        Input::Observation(Observation::Position(received)),
    ) = (&input, &back)
    else {
        panic!("unexpected variants: {input:?} / {back:?}");
    };
    assert!(
        sent.position.distance(received.position) < Length::from_meters(1e-6),
        "{json}"
    );
    assert_eq!(sent.observed_at, received.observed_at);
    assert_eq!(sent.altitude, received.altitude);
}

#[test]
fn clock_never_goes_backward() {
    let mut app = App::new();
    app.handle(position_input(1., 50., 6.));

    // A stale clock input must not resurrect an earlier deadline.
    let update = app.handle(Input::Clock {
        now: MonotonicTime::ORIGIN,
    });
    assert_eq!(update.changes, vec![]);
    assert_eq!(update.effects, vec![]);
}
