use claims::{assert_le, assert_ok, assert_some};
use time::{
    OffsetDateTime,
    macros::{date, datetime},
};
use updraft_geo::LatLon;
use updraft_sun::{SolarEvent, SolarEventsError, solar_events};

#[test]
fn events_match_aachen_reference_times() {
    // The timeanddate.com Aachen table reports local CEST times for this date.
    let events = assert_ok!(solar_events(
        LatLon::from_degrees(50.7753, 6.0839),
        date!(2026 - 06 - 21),
    ));
    assert_near(events.solar_noon(), datetime!(2026-06-21 11:37 UTC));
    assert_near(occurring(events.sunrise()), datetime!(2026-06-21 03:22 UTC));
    assert_near(occurring(events.sunset()), datetime!(2026-06-21 19:52 UTC));
    assert_near(
        occurring(events.civil_dawn()),
        datetime!(2026-06-21 02:36 UTC),
    );
    assert_near(
        occurring(events.civil_dusk()),
        datetime!(2026-06-21 20:38 UTC),
    );
}

#[test]
fn invalid_locations_and_unrepresentable_events_are_rejected() {
    assert_eq!(
        solar_events(LatLon::from_degrees(91.0, 0.0), date!(2026 - 06 - 21)),
        Err(SolarEventsError::InvalidLocation)
    );
    assert_eq!(
        solar_events(LatLon::from_degrees(0.0, -179.0), time::Date::MAX),
        Err(SolarEventsError::DateOutOfRange)
    );
}

#[test]
fn polar_results_distinguish_summer_and_winter() {
    let location = LatLon::from_degrees(80.0, 0.0);
    let summer = assert_ok!(solar_events(location, date!(2026 - 06 - 21)));
    assert_eq!(summer.sunrise(), SolarEvent::AlwaysAbove);
    assert_eq!(summer.sunset(), SolarEvent::AlwaysAbove);
    assert_eq!(summer.civil_dawn(), SolarEvent::AlwaysAbove);
    assert_eq!(summer.civil_dusk(), SolarEvent::AlwaysAbove);

    let winter = assert_ok!(solar_events(location, date!(2026 - 12 - 21)));
    assert_eq!(winter.sunrise(), SolarEvent::AlwaysBelow);
    assert_eq!(winter.sunset(), SolarEvent::AlwaysBelow);
    assert_eq!(winter.civil_dawn(), SolarEvent::AlwaysBelow);
    assert_eq!(winter.civil_dusk(), SolarEvent::AlwaysBelow);
}

#[test]
fn the_input_date_selects_the_transit_near_the_date_line() {
    let date = date!(2026 - 03 - 20);
    let east = assert_ok!(solar_events(LatLon::from_degrees(0.0, 179.0), date));
    let west = assert_ok!(solar_events(LatLon::from_degrees(0.0, -179.0), date));

    assert_eq!(east.solar_noon().date(), date);
    assert_eq!(west.solar_noon().date(), assert_some!(date.next_day()));
}

fn occurring(event: SolarEvent) -> OffsetDateTime {
    match event {
        SolarEvent::Occurs(instant) => instant,
        SolarEvent::AlwaysAbove | SolarEvent::AlwaysBelow => panic!("expected a solar event"),
    }
}

fn assert_near(actual: OffsetDateTime, expected: OffsetDateTime) {
    assert_le!((actual - expected).whole_seconds().abs(), 120);
}
