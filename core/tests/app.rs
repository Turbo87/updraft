use std::time::Duration;

use updraft_core::{
    App, Change, FlightChange, FlightInput, Input, MonotonicTime, ObservationSource,
    OwnshipPosition, PositionObservation,
};
use updraft_geo::LatLon;
use updraft_units::Angle;

#[test]
fn new_app_has_no_position() {
    assert_eq!(App::default().snapshot().position, None);
}

#[test]
fn position_observation_updates_snapshot_and_emits_change() {
    let location = LatLon::from_degrees(50.823, 6.186);
    let track = Some(Angle::from_degrees(45.));
    let position = PositionObservation::new(
        ObservationSource::Simulation,
        MonotonicTime::from_duration(Duration::from_secs(1)),
        location,
        track,
    )
    .unwrap();
    let expected = OwnshipPosition::new(location, track);
    let mut app = App::default();

    let update = app.handle(Input::Flight(FlightInput::PositionObserved(position)));

    assert_eq!(app.snapshot().position, Some(expected));
    assert_eq!(
        update.changes,
        vec![Change::Flight(FlightChange::PositionChanged(expected))]
    );
    assert!(update.effects.is_empty());
}

#[test]
fn latest_position_observation_replaces_the_current_position() {
    let mut app = App::default();
    let first = PositionObservation::new(
        ObservationSource::Simulation,
        MonotonicTime::from_duration(Duration::from_secs(1)),
        LatLon::from_degrees(50.823, 6.186),
        Some(Angle::from_degrees(45.)),
    )
    .unwrap();
    let second_location = LatLon::from_degrees(50.9, 6.3);
    let second_track = Some(Angle::from_degrees(90.));
    let second = PositionObservation::new(
        ObservationSource::Simulation,
        MonotonicTime::from_duration(Duration::from_secs(2)),
        second_location,
        second_track,
    )
    .unwrap();

    app.handle(Input::Flight(FlightInput::PositionObserved(first)));
    let update = app.handle(Input::Flight(FlightInput::PositionObserved(second)));

    let expected = OwnshipPosition::new(second_location, second_track);
    assert_eq!(app.snapshot().position, Some(expected));
    assert_eq!(
        update.changes,
        vec![Change::Flight(FlightChange::PositionChanged(expected))]
    );
}

#[test]
fn position_observation_rejects_latitude_outside_the_globe() {
    let result = PositionObservation::new(
        ObservationSource::Simulation,
        MonotonicTime::from_duration(Duration::ZERO),
        LatLon::from_degrees(91., 6.186),
        None,
    );

    assert_eq!(result, Err(updraft_core::InvalidPosition));
}

#[test]
fn position_observation_rejects_longitude_outside_the_globe() {
    let result = PositionObservation::new(
        ObservationSource::Simulation,
        MonotonicTime::from_duration(Duration::ZERO),
        LatLon::from_degrees(50.823, 181.),
        None,
    );

    assert_eq!(result, Err(updraft_core::InvalidPosition));
}

#[test]
fn position_observation_rejects_non_finite_track() {
    let result = PositionObservation::new(
        ObservationSource::Simulation,
        MonotonicTime::from_duration(Duration::ZERO),
        LatLon::from_degrees(50.823, 6.186),
        Some(Angle::from_degrees(f64::NAN)),
    );

    assert_eq!(result, Err(updraft_core::InvalidPosition));
}
