use std::time::Duration;

use updraft_core::{
    App, Change, FlightChange, FlightInput, Input, MonotonicTime, ObservationSource,
    OwnshipPosition, PositionObservation, Snapshot,
};
use updraft_geo::LatLon;
use updraft_units::Angle;

fn observation(latitude: f64, longitude: f64, track: Option<f64>) -> PositionObservation {
    PositionObservation::new(
        ObservationSource::Simulation,
        MonotonicTime::from_duration(Duration::from_secs(1)),
        LatLon::from_degrees(latitude, longitude),
        track.map(Angle::from_degrees),
    )
    .unwrap()
}

fn position(latitude: f64, longitude: f64, track: Option<f64>) -> OwnshipPosition {
    OwnshipPosition {
        location: LatLon::from_degrees(latitude, longitude),
        track: track.map(Angle::from_degrees),
    }
}

#[test]
fn new_app_has_no_position() {
    let app = App::default();
    assert_eq!(app.snapshot(), Snapshot { position: None });
}

#[test]
fn position_observation_updates_snapshot_and_emits_change() {
    let mut app = App::default();

    let input = Input::Flight(FlightInput::PositionObserved(observation(
        50.823,
        6.186,
        Some(45.),
    )));
    let update = app.handle(input);

    let expected = position(50.823, 6.186, Some(45.));
    assert_eq!(
        update.changes,
        [Change::Flight(FlightChange::PositionChanged(expected))]
    );
    assert!(update.effects.is_empty());
    assert_eq!(
        app.snapshot(),
        Snapshot {
            position: Some(expected)
        }
    );
}

#[test]
fn latest_position_observation_wins() {
    let mut app = App::default();

    app.handle(Input::Flight(FlightInput::PositionObserved(observation(
        50.823,
        6.186,
        Some(45.),
    ))));
    app.handle(Input::Flight(FlightInput::PositionObserved(observation(
        50.824, 6.187, None,
    ))));

    assert_eq!(
        app.snapshot(),
        Snapshot {
            position: Some(position(50.824, 6.187, None))
        }
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
    let input = Input::Flight(FlightInput::PositionObserved(observation(
        50.823,
        6.186,
        Some(45.),
    )));

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
}
