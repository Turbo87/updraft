use std::time::Duration;

use tokio::time::timeout;
use updraft_core::{
    App, Change, FlightChange, FlightInput, Input, MonotonicTime, ObservationSource,
    OwnshipPosition, PositionFix, PositionObservation, Snapshot,
};
use updraft_geo::LatLon;
use updraft_runtime::{RuntimeHandle, StateMessage, StateStream};

fn position_input(offset_secs: u64, latitude: f64, longitude: f64) -> (Input, OwnshipPosition) {
    let location = LatLon::from_degrees(latitude, longitude);
    let observation = PositionObservation::new(
        ObservationSource::Simulation,
        MonotonicTime::from_duration(Duration::from_secs(offset_secs)),
        location,
        None,
    )
    .unwrap();
    let position = OwnshipPosition {
        location,
        track: None,
    };
    (
        Input::Flight(FlightInput::PositionObserved(observation)),
        position,
    )
}

fn snapshot_with(position: OwnshipPosition) -> Snapshot {
    Snapshot {
        position: Some(PositionFix::Current(position)),
        ..Snapshot::default()
    }
}

async fn recv(stream: &mut StateStream) -> Option<StateMessage> {
    timeout(Duration::from_secs(60), stream.recv())
        .await
        .expect("state stream stalled")
}

/// Collects change batches until `predicate` matches one change, panicking
/// after a bounded number of messages.
async fn recv_until(stream: &mut StateStream, predicate: impl Fn(&Change) -> bool) -> Change {
    for _ in 0..32 {
        match recv(stream).await.expect("state stream ended") {
            StateMessage::Changes(changes) => {
                if let Some(change) = changes.into_iter().find(&predicate) {
                    return change;
                }
            }
            StateMessage::Snapshot(_) => {}
        }
    }
    panic!("expected change did not arrive");
}

#[tokio::test]
async fn subscriber_receives_the_current_snapshot_first() {
    let runtime = updraft_runtime::spawn(App::default());

    let mut stream = runtime.subscribe().await.unwrap();

    assert_eq!(
        recv(&mut stream).await,
        Some(StateMessage::Snapshot(Snapshot::default()))
    );
}

#[tokio::test]
async fn changes_follow_the_snapshot_in_submission_order() {
    let runtime = updraft_runtime::spawn(App::default());
    let mut stream = runtime.subscribe().await.unwrap();
    recv(&mut stream).await.unwrap();

    let (first_input, first) = position_input(1, 50.823, 6.186);
    let (second_input, second) = position_input(2, 50.8231, 6.1861);
    runtime.submit(first_input).await.unwrap();
    runtime.submit(second_input).await.unwrap();

    assert_eq!(
        recv(&mut stream).await,
        Some(StateMessage::Changes(vec![Change::Flight(
            FlightChange::PositionChanged(first)
        )]))
    );
    assert_eq!(
        recv(&mut stream).await,
        Some(StateMessage::Changes(vec![Change::Flight(
            FlightChange::PositionChanged(second)
        )]))
    );
}

#[tokio::test]
async fn late_subscriber_snapshot_contains_earlier_submissions() {
    let runtime = updraft_runtime::spawn(App::default());

    let (input, position) = position_input(1, 50.823, 6.186);
    runtime.submit(input).await.unwrap();

    let mut stream = runtime.subscribe().await.unwrap();
    assert_eq!(
        recv(&mut stream).await,
        Some(StateMessage::Snapshot(snapshot_with(position)))
    );
}

#[tokio::test]
async fn subscriber_that_falls_behind_is_dropped_without_blocking_inputs() {
    let runtime = updraft_runtime::spawn(App::default());
    let mut stalled = runtime.subscribe().await.unwrap();

    // Never read from `stalled`: its buffer (snapshot + changes) fills up
    // and the runtime must drop it instead of waiting for it.
    for offset in 0..128 {
        let (input, _) = position_input(offset, 50., 6.);
        timeout(Duration::from_secs(5), runtime.submit(input))
            .await
            .expect("submit blocked on a stalled subscriber")
            .unwrap();
    }

    // The stalled stream ends after the buffered prefix.
    while recv(&mut stalled).await.is_some() {}

    // Resubscribing recovers with a fresh snapshot.
    let (input, position) = position_input(200, 50.0001, 6.0001);
    runtime.submit(input).await.unwrap();
    let mut fresh = runtime.subscribe().await.unwrap();
    let Some(StateMessage::Snapshot(snapshot)) = recv(&mut fresh).await else {
        panic!("expected a snapshot");
    };
    assert_eq!(snapshot.position, Some(PositionFix::Current(position)));
}

#[tokio::test]
async fn handles_are_cloneable_across_tasks() {
    let runtime = updraft_runtime::spawn(App::default());
    let clone: RuntimeHandle = runtime.clone();

    let (input, position) = position_input(1, 50.823, 6.186);
    tokio::spawn(async move { clone.submit(input).await })
        .await
        .unwrap()
        .unwrap();

    let mut stream = runtime.subscribe().await.unwrap();
    let Some(StateMessage::Snapshot(snapshot)) = recv(&mut stream).await else {
        panic!("expected a snapshot");
    };
    assert_eq!(snapshot.position, Some(PositionFix::Current(position)));
}

/// The compute seam end to end: observations spawn jobs on the worker
/// thread, and its result re-enters as an input and reaches the stream.
#[tokio::test]
async fn track_distance_arrives_from_the_worker_thread() {
    let runtime = updraft_runtime::spawn(App::default());
    let mut stream = runtime.subscribe().await.unwrap();

    let (first, _) = position_input(1, 50.823, 6.186);
    let (second, _) = position_input(2, 50.824, 6.187);
    runtime.submit(first).await.unwrap();
    runtime.submit(second).await.unwrap();

    let change = recv_until(&mut stream, |change| {
        matches!(
            change,
            Change::Flight(FlightChange::TrackDistanceChanged(_))
        )
    })
    .await;

    // Same build, same platform, same geodesic code path: exact equality.
    let expected =
        LatLon::from_degrees(50.823, 6.186).distance(LatLon::from_degrees(50.824, 6.187));
    assert_eq!(
        change,
        Change::Flight(FlightChange::TrackDistanceChanged(expected))
    );
}

/// The clock driver end to end, on tokio's paused clock: the runtime
/// arms its sleep from `next_deadline`, time auto-advances, and the
/// resulting clock input marks the position stale.
#[tokio::test(start_paused = true)]
async fn clock_driver_fires_the_staleness_deadline() {
    let runtime = updraft_runtime::spawn(App::default());
    let mut stream = runtime.subscribe().await.unwrap();

    let location = LatLon::from_degrees(50.823, 6.186);
    let observation =
        PositionObservation::new(ObservationSource::Simulation, runtime.now(), location, None)
            .unwrap();
    runtime
        .submit(Input::Flight(FlightInput::PositionObserved(observation)))
        .await
        .unwrap();

    let change = recv_until(&mut stream, |change| {
        matches!(change, Change::Flight(FlightChange::PositionStale))
    })
    .await;
    assert_eq!(change, Change::Flight(FlightChange::PositionStale));
}
