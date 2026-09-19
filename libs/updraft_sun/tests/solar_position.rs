use approx::assert_abs_diff_eq;
use claims::{assert_ge, assert_gt, assert_lt, assert_ok};
use time::macros::{datetime, offset};
use updraft_geo::LatLon;
use updraft_sun::{InvalidLocation, solar_position};

#[test]
fn solar_position_matches_the_nrel_spa_reference() {
    // NREL SPA report example A.4, converted from a zenith angle to elevation.
    let location = LatLon::from_degrees(39.742476, -105.1786);
    let position = assert_ok!(solar_position(location, datetime!(2003-10-17 19:30:30 UTC)));

    assert_abs_diff_eq!(position.azimuth().as_degrees(), 194.34024, epsilon = 0.02);
    assert_abs_diff_eq!(position.elevation().as_degrees(), 39.88838, epsilon = 0.02);
}

#[test]
fn azimuth_is_normalized_and_elevation_is_signed() {
    let location = LatLon::from_degrees(50.0, 6.0);
    for instant in [
        datetime!(2026-06-21 00:00 UTC),
        datetime!(2026-06-21 12:00 UTC),
        datetime!(2026-12-21 00:00 UTC),
        datetime!(2026-12-21 12:00 UTC),
    ] {
        let position = assert_ok!(solar_position(location, instant));
        assert_ge!(position.azimuth().as_degrees(), 0.0);
        assert_lt!(position.azimuth().as_degrees(), 360.0);
    }

    let night = assert_ok!(solar_position(location, datetime!(2026-06-21 00:00 UTC)));
    assert_lt!(night.elevation().as_degrees(), 0.0);
    let day = assert_ok!(solar_position(location, datetime!(2026-06-21 12:00 UTC)));
    assert_gt!(day.elevation().as_degrees(), 0.0);
}

#[test]
fn invalid_locations_are_rejected() {
    for location in [
        LatLon::from_degrees(90.1, 0.0),
        LatLon::from_degrees(f64::NAN, 0.0),
        LatLon::from_degrees(0.0, f64::INFINITY),
    ] {
        assert_eq!(
            solar_position(location, datetime!(2026-06-21 12:00 UTC)),
            Err(InvalidLocation)
        );
    }
}

#[test]
fn equivalent_offset_date_times_produce_the_same_position() {
    let location = LatLon::from_degrees(50.0, 6.0);
    let utc = datetime!(2026-06-21 12:00 UTC);

    assert_eq!(
        solar_position(location, utc),
        solar_position(location, utc.to_offset(offset!(+2)))
    );
}
