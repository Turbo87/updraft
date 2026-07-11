//! Whole-flow scenario tests: a plain loop over `App::handle`, with no
//! async runtime, threads, sleeps, or wall clock. Time and worker results
//! are fed in as ordinary inputs.

use updraft_core::{
    App, Change, ComputeFailure, ComputeJob, ComputeKind, ComputeResult, Effect, Input,
    MonotonicTime, Query, QueryResult, Reduction, Sample, Snapshot,
};

fn observe(app: &mut App, value: u32, at_millis: u64) -> updraft_core::Update {
    app.handle(Input::Observe(Sample {
        value,
        observed_at: MonotonicTime::from_millis(at_millis),
    }))
}

fn tick(app: &mut App, at_millis: u64) -> updraft_core::Update {
    app.handle(Input::Tick(MonotonicTime::from_millis(at_millis)))
}

fn completed(app: &mut App, reduction: Reduction) -> updraft_core::Update {
    app.handle(Input::ComputeCompleted(ComputeResult::Demo { reduction }))
}

#[test]
fn samples_are_batched_then_reduced_by_the_worker() {
    let mut app = App::with_interval_millis(1000);

    // The first observation is cheap and synchronous, and arms the cadence
    // timer one interval out.
    let update = observe(&mut app, 10, 0);
    assert_eq!(update.changes, vec![Change::Samples(1)]);
    assert!(update.effects.is_empty());
    assert_eq!(update.next_deadline, Some(MonotonicTime::from_millis(1000)));

    // More samples accumulate before the timer fires.
    assert_eq!(observe(&mut app, 20, 100).changes, vec![Change::Samples(2)]);
    assert_eq!(observe(&mut app, 30, 200).changes, vec![Change::Samples(3)]);

    // A tick before the deadline does nothing.
    let update = tick(&mut app, 500);
    assert!(update.effects.is_empty());
    assert_eq!(update.next_deadline, Some(MonotonicTime::from_millis(1000)));

    // At the deadline the whole batch is handed to the worker, and the
    // timer disarms because there is no more pending work.
    let update = tick(&mut app, 1000);
    assert_eq!(
        update.effects,
        vec![Effect::Compute(ComputeJob::Demo {
            batch: vec![10, 20, 30],
        })]
    );
    assert!(update.changes.is_empty());
    assert_eq!(update.next_deadline, None);

    // The worker result returns as an input and becomes a change.
    let reduction = Reduction { count: 3, sum: 60 };
    let update = completed(&mut app, reduction);
    assert_eq!(update.changes, vec![Change::Computed(reduction)]);
    assert!(update.effects.is_empty());
    assert_eq!(update.next_deadline, None);

    assert_eq!(
        app.query(Query::LatestReduction),
        QueryResult::LatestReduction(Some(reduction))
    );
    assert_eq!(app.query(Query::SampleCount), QueryResult::SampleCount(3));
    assert_eq!(
        app.snapshot(),
        Snapshot {
            sample_count: 3,
            latest: Some(reduction),
        }
    );
}

#[test]
fn a_sample_arriving_mid_job_triggers_a_rerun_on_completion() {
    let mut app = App::with_interval_millis(1000);

    observe(&mut app, 1, 0);
    let update = tick(&mut app, 1000);
    assert_eq!(
        update.effects,
        vec![Effect::Compute(ComputeJob::Demo { batch: vec![1] })]
    );

    // A sample arrives while the worker is busy; the timer re-arms to retry.
    let update = observe(&mut app, 2, 1100);
    assert_eq!(update.changes, vec![Change::Samples(2)]);
    assert_eq!(update.next_deadline, Some(MonotonicTime::from_millis(2100)));

    // When the first job returns, the core immediately launches a rerun
    // over the full accumulation without waiting for the next tick.
    let update = completed(&mut app, Reduction { count: 1, sum: 1 });
    assert_eq!(
        update.changes,
        vec![Change::Computed(Reduction { count: 1, sum: 1 })]
    );
    assert_eq!(
        update.effects,
        vec![Effect::Compute(ComputeJob::Demo { batch: vec![1, 2] })]
    );
}

#[test]
fn a_worker_failure_frees_the_slot_and_never_stalls() {
    let mut app = App::with_interval_millis(1000);

    observe(&mut app, 5, 0);
    let update = tick(&mut app, 1000);
    assert_eq!(update.effects.len(), 1);

    // The worker fails: no change, no rerun (no new work), and no result.
    let update = app.handle(Input::ComputeFailed(ComputeFailure {
        kind: ComputeKind::Demo,
        message: "boom".to_owned(),
    }));
    assert!(update.changes.is_empty());
    assert!(update.effects.is_empty());
    assert_eq!(
        app.query(Query::LatestReduction),
        QueryResult::LatestReduction(None)
    );

    // A later sample recovers: the next job re-reduces the full accumulation.
    observe(&mut app, 6, 1100);
    let update = tick(&mut app, 2100);
    assert_eq!(
        update.effects,
        vec![Effect::Compute(ComputeJob::Demo { batch: vec![5, 6] })]
    );
}

#[test]
fn no_timer_is_armed_before_the_first_sample() {
    let mut app = App::new();
    assert_eq!(app.snapshot(), Snapshot::default());

    // A tick with nothing observed yields no work and no pending deadline.
    let update = tick(&mut app, 1000);
    assert!(update.changes.is_empty());
    assert!(update.effects.is_empty());
    assert_eq!(update.next_deadline, None);
}
