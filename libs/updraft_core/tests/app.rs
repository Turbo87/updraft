use std::time::Duration;

use updraft_core::{
    App, Change, ComputeJob, Effect, FlightChange, FlightInput, Input, JobOutcome, JobResult,
    MonotonicTime, ObservationSource, OwnshipPosition, PositionFix, PositionObservation, Snapshot,
    Update,
};
use updraft_geo::LatLon;
use updraft_units::{Angle, Length};

fn at(secs: u64) -> MonotonicTime {
    MonotonicTime::from_duration(Duration::from_secs(secs))
}

fn observation(secs: u64, latitude: f64, longitude: f64, track: Option<f64>) -> Input {
    let observation = PositionObservation::new(
        ObservationSource::Simulation,
        at(secs),
        LatLon::from_degrees(latitude, longitude),
        track.map(Angle::from_degrees),
    )
    .unwrap();
    Input::Flight(FlightInput::PositionObserved(observation))
}

fn position(latitude: f64, longitude: f64, track: Option<f64>) -> OwnshipPosition {
    OwnshipPosition {
        location: LatLon::from_degrees(latitude, longitude),
        track: track.map(Angle::from_degrees),
    }
}

/// Extracts the compute job from an update expected to spawn exactly one.
fn spawned_job(update: &Update) -> &ComputeJob {
    match update.effects.as_slice() {
        [Effect::Compute(job)] => job,
        effects => panic!("expected exactly one compute effect, got {effects:?}"),
    }
}

#[test]
fn new_app_is_empty() {
    let app = App::default();
    assert_eq!(app.snapshot(), Snapshot::default());
}

#[test]
fn position_observation_updates_snapshot_and_emits_change() {
    let mut app = App::default();

    let update = app.handle(observation(1, 50.823, 6.186, Some(45.)));

    let expected = position(50.823, 6.186, Some(45.));
    assert_eq!(
        update.changes,
        [Change::Flight(FlightChange::PositionChanged(expected))]
    );
    assert_eq!(
        app.snapshot().position,
        Some(PositionFix::Current(expected))
    );
}

#[test]
fn latest_position_observation_wins() {
    let mut app = App::default();

    app.handle(observation(1, 50.823, 6.186, Some(45.)));
    app.handle(observation(2, 50.824, 6.187, None));

    assert_eq!(
        app.snapshot().position,
        Some(PositionFix::Current(position(50.824, 6.187, None)))
    );
}

#[test]
fn observations_reject_out_of_range_and_non_finite_values() {
    let attempt = |latitude: f64, longitude: f64, track: Option<f64>| {
        PositionObservation::new(
            ObservationSource::Simulation,
            MonotonicTime::default(),
            LatLon::from_degrees(latitude, longitude),
            track.map(Angle::from_degrees),
        )
    };

    assert!(attempt(90.1, 0., None).is_err());
    assert!(attempt(-90.1, 0., None).is_err());
    assert!(attempt(0., 180.1, None).is_err());
    assert!(attempt(0., -180.1, None).is_err());
    assert!(attempt(f64::NAN, 0., None).is_err());
    assert!(attempt(0., f64::INFINITY, None).is_err());
    assert!(attempt(0., 0., Some(f64::NAN)).is_err());

    assert!(attempt(90., 180., Some(0.)).is_ok());
    assert!(attempt(-90., -180., None).is_ok());
}

#[test]
fn inputs_round_trip_through_serde() {
    let input = observation(1, 50.823, 6.186, Some(45.));

    let json = serde_json::to_string(&input).unwrap();
    let deserialized: Input = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized, input);
}

#[test]
fn changes_serialize_to_the_documented_wire_shape() {
    let change = Change::Flight(FlightChange::PositionChanged(position(
        50.823,
        6.186,
        Some(45.),
    )));

    assert_eq!(
        serde_json::to_value(change).unwrap(),
        serde_json::json!({
            "flight": {
                "position_changed": {
                    "location": { "latitude": 50.823, "longitude": 6.186 },
                    "track": 45.0,
                }
            }
        })
    );

    assert_eq!(
        serde_json::to_value(Change::Flight(FlightChange::PositionStale)).unwrap(),
        serde_json::json!({ "flight": "position_stale" })
    );
}

#[test]
fn position_goes_stale_when_the_clock_passes_the_deadline() {
    let mut app = App::default();

    let update = app.handle(observation(1, 50.823, 6.186, None));
    assert_eq!(update.next_deadline, Some(at(11)));

    // Not due yet: nothing fires, the deadline stays armed.
    let update = app.handle(Input::Clock(at(5)));
    assert!(update.changes.is_empty());
    assert_eq!(update.next_deadline, Some(at(11)));

    let update = app.handle(Input::Clock(at(11)));
    assert_eq!(
        update.changes,
        [Change::Flight(FlightChange::PositionStale)]
    );
    assert_eq!(update.next_deadline, None);
    assert_eq!(
        app.snapshot().position,
        Some(PositionFix::Stale(position(50.823, 6.186, None)))
    );

    // A fresh observation revives the position.
    let update = app.handle(observation(20, 50.824, 6.187, None));
    assert_eq!(
        update.changes,
        [Change::Flight(FlightChange::PositionChanged(position(
            50.824, 6.187, None
        )))]
    );
    assert_eq!(
        app.snapshot().position,
        Some(PositionFix::Current(position(50.824, 6.187, None)))
    );
}

#[test]
fn fresh_observations_rearm_the_staleness_deadline() {
    let mut app = App::default();

    app.handle(observation(1, 50.823, 6.186, None));
    let update = app.handle(observation(6, 50.823, 6.186, None));
    assert_eq!(update.next_deadline, Some(at(16)));

    // The original deadline has passed, but re-arming moved it.
    let update = app.handle(Input::Clock(at(11)));
    assert!(update.changes.is_empty());
    assert_eq!(update.next_deadline, Some(at(16)));
}

#[test]
fn track_distance_jobs_run_one_at_a_time_and_batch_pending_points() {
    let mut app = App::default();

    // The first observation spawns a job with its point.
    let update = app.handle(observation(1, 50.823, 6.186, None));
    let first_job = spawned_job(&update).clone();
    let ComputeJob::TrackDistance { epoch, points } = &first_job;
    assert_eq!(points, &[LatLon::from_degrees(50.823, 6.186)]);
    let epoch = *epoch;

    // While it runs, further points only accumulate.
    let update = app.handle(observation(2, 50.824, 6.187, None));
    assert!(update.effects.is_empty());
    let update = app.handle(observation(3, 50.825, 6.188, None));
    assert!(update.effects.is_empty());

    // Its completion applies the result and spawns the next job with the
    // accumulated batch.
    let update = app.handle(Input::Job(JobOutcome::Completed {
        epoch,
        result: JobResult::TrackDistance(Length::from_meters(0.)),
    }));
    assert!(
        update.changes.is_empty(),
        "unchanged total is not re-published"
    );
    let ComputeJob::TrackDistance { points, .. } = spawned_job(&update);
    assert_eq!(
        points,
        &[
            LatLon::from_degrees(50.824, 6.187),
            LatLon::from_degrees(50.825, 6.188),
        ]
    );

    let update = app.handle(Input::Job(JobOutcome::Completed {
        epoch,
        result: JobResult::TrackDistance(Length::from_meters(260.)),
    }));
    assert_eq!(
        update.changes,
        [Change::Flight(FlightChange::TrackDistanceChanged(
            Length::from_meters(260.)
        ))]
    );
    assert!(update.effects.is_empty());
    assert_eq!(app.snapshot().track_distance, Length::from_meters(260.));
}

#[test]
fn a_discontinuity_bumps_the_epoch_and_drops_the_stale_result() {
    let mut app = App::default();

    let update = app.handle(observation(1, 50.823, 6.186, None));
    let first_epoch = spawned_job(&update).epoch();

    // A teleport (simulator drag, replay seek) while the job is running.
    let update = app.handle(observation(2, 51.9, 8.5, None));
    assert!(update.effects.is_empty(), "job still in flight");

    // The in-flight result is semantically wrong now: dropped, and the
    // next job carries the new epoch and only the post-jump point.
    let update = app.handle(Input::Job(JobOutcome::Completed {
        epoch: first_epoch,
        result: JobResult::TrackDistance(Length::from_kilometers(500.)),
    }));
    assert!(update.changes.is_empty());
    let ComputeJob::TrackDistance { epoch, points } = spawned_job(&update);
    assert_ne!(*epoch, first_epoch);
    assert_eq!(points, &[LatLon::from_degrees(51.9, 8.5)]);
    assert_eq!(app.snapshot().track_distance, Length::default());
}

#[test]
fn a_failed_job_resets_the_accumulation_and_recovers() {
    let mut app = App::default();

    let update = app.handle(observation(1, 50.823, 6.186, None));
    let first_epoch = spawned_job(&update).epoch();
    app.handle(Input::Job(JobOutcome::Completed {
        epoch: first_epoch,
        result: JobResult::TrackDistance(Length::from_meters(100.)),
    }));

    let update = app.handle(observation(2, 50.824, 6.187, None));
    let failed_job = spawned_job(&update).clone();
    let update = app.handle(Input::Job(JobOutcome::Failed {
        kind: failed_job.kind(),
        epoch: failed_job.epoch(),
    }));
    assert_eq!(
        update.changes,
        [Change::Flight(FlightChange::TrackDistanceChanged(
            Length::default()
        ))]
    );
    assert!(
        update.effects.is_empty(),
        "no immediate retry after a failure"
    );

    // The next observation starts a fresh accumulation under a new epoch.
    let update = app.handle(observation(3, 50.825, 6.188, None));
    assert_ne!(spawned_job(&update).epoch(), first_epoch);
}

#[test]
fn snapshot_serializes_to_the_documented_wire_shape() {
    let mut app = App::default();
    app.handle(observation(1, 50.823, 6.186, None));

    assert_eq!(
        serde_json::to_value(app.snapshot()).unwrap(),
        serde_json::json!({
            "position": {
                "current": {
                    "location": { "latitude": 50.823, "longitude": 6.186 },
                    "track": null,
                }
            },
            "track_distance": 0.0,
        })
    );
}
