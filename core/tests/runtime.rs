use std::time::Duration;

use updraft_core::{
    App, Change, CoreRuntime, CoreRuntimeHandle, FlightChange, FlightInput, Input, MonotonicTime,
    ObservationSource, OwnshipPosition, PositionObservation, Snapshot, StateMessage,
};
use updraft_geo::LatLon;
use updraft_units::Angle;

fn position(
    observed_after: Duration,
    latitude: f64,
    longitude: f64,
    track: f64,
) -> (PositionObservation, OwnshipPosition) {
    let location = LatLon::from_degrees(latitude, longitude);
    let track = Some(Angle::from_degrees(track));
    let observation = PositionObservation::new(
        ObservationSource::Simulation,
        MonotonicTime::from_duration(observed_after),
        location,
        track,
    )
    .unwrap();
    (observation, OwnshipPosition::new(location, track))
}

async fn submit_position_burst(runtime: &CoreRuntimeHandle) {
    for offset in 0..32 {
        let (position, _) = position(
            Duration::from_secs(offset),
            50.823 + offset as f64 / 1_000.,
            6.186,
            45.,
        );
        runtime
            .submit(Input::Flight(FlightInput::PositionObserved(position)))
            .await
            .unwrap();
    }
}

#[tokio::test]
async fn subscriber_receives_current_snapshot_first() {
    let runtime = CoreRuntime::spawn(App::default());

    let mut stream = runtime.subscribe().await.unwrap();

    assert_eq!(
        stream.recv().await,
        Some(StateMessage::Snapshot(Snapshot::default()))
    );
}

#[tokio::test]
async fn submitted_input_publishes_changes_after_the_snapshot() {
    let runtime = CoreRuntime::spawn(App::default());
    let mut stream = runtime.subscribe().await.unwrap();
    assert_eq!(
        stream.recv().await,
        Some(StateMessage::Snapshot(Snapshot::default()))
    );
    let (position, expected) = position(Duration::from_secs(1), 50.823, 6.186, 45.);

    runtime
        .submit(Input::Flight(FlightInput::PositionObserved(position)))
        .await
        .unwrap();

    assert_eq!(
        stream.recv().await,
        Some(StateMessage::Changes(vec![Change::Flight(
            FlightChange::PositionChanged(expected)
        )]))
    );
}

#[tokio::test]
async fn submitted_inputs_publish_changes_in_fifo_order() {
    let runtime = CoreRuntime::spawn(App::default());
    let mut stream = runtime.subscribe().await.unwrap();
    assert!(matches!(
        stream.recv().await,
        Some(StateMessage::Snapshot(_))
    ));
    let (first, first_expected) = position(Duration::from_secs(1), 50.823, 6.186, 45.);
    let (second, second_expected) = position(Duration::from_secs(2), 50.9, 6.3, 90.);

    runtime
        .submit(Input::Flight(FlightInput::PositionObserved(first)))
        .await
        .unwrap();
    runtime
        .submit(Input::Flight(FlightInput::PositionObserved(second)))
        .await
        .unwrap();

    assert_eq!(
        stream.recv().await,
        Some(StateMessage::Changes(vec![Change::Flight(
            FlightChange::PositionChanged(first_expected)
        )]))
    );
    assert_eq!(
        stream.recv().await,
        Some(StateMessage::Changes(vec![Change::Flight(
            FlightChange::PositionChanged(second_expected)
        )]))
    );
}

#[tokio::test]
async fn late_subscriber_receives_the_latest_position_in_its_snapshot() {
    let runtime = CoreRuntime::spawn(App::default());
    let (position, expected) = position(Duration::from_secs(1), 50.823, 6.186, 45.);
    runtime
        .submit(Input::Flight(FlightInput::PositionObserved(position)))
        .await
        .unwrap();

    let mut stream = runtime.subscribe().await.unwrap();

    assert_eq!(
        stream.recv().await,
        Some(StateMessage::Snapshot(Snapshot {
            position: Some(expected)
        }))
    );
}

#[tokio::test]
async fn full_subscriber_queue_does_not_block_input_processing() {
    let runtime = CoreRuntime::spawn(App::default());
    let slow_stream = runtime.subscribe().await.unwrap();

    let late_stream = tokio::time::timeout(Duration::from_secs(1), async {
        submit_position_burst(&runtime).await;
        runtime.subscribe().await
    })
    .await
    .expect("slow subscriber blocked input processing")
    .unwrap();

    assert!(slow_stream.is_closed());
    drop(late_stream);
}
