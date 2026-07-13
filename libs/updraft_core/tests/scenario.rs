//! Whole-flight scenario tests run as a plain loop over `App::handle()`.

use std::time::Duration;

use updraft_core::flight::{
    Change as FlightChange, GetPosition, MslAltitude, Observation, PositionFix,
};
use updraft_core::{App, Change, Input};
use updraft_geo::LatLon;
use updraft_units::Length;

#[test]
fn app_routes_position_state_through_the_flight_domain() {
    let mut app = App::new();
    let fix = PositionFix {
        observed_at: Duration::from_secs(1),
        position: LatLon::from_degrees(50., 6.),
        altitude: Some(MslAltitude::new(Length::from_meters(1_000.))),
        track: None,
        ground_speed: None,
    };

    let update = app.handle(Input::Flight(updraft_core::flight::Input::Observation(
        Observation::Position(fix),
    )));

    assert_eq!(
        update.changes,
        vec![Change::Flight(FlightChange::Position(fix))]
    );
    assert_eq!(app.query(GetPosition), Some(fix));
    assert_eq!(app.snapshot().flight.position, Some(fix));
}
