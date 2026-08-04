use approx::assert_abs_diff_eq;
use claims::{assert_err_eq, assert_ge, assert_gt, assert_le, assert_lt, assert_ok};
use std::assert_matches;
use updraft_airspace::{
    AirspaceAltitude, AirspaceClass, AirspaceDataset, AirspaceGeometryError, AirspaceId,
    AirspaceImportError, AirspaceParseError, AirspaceType,
};
use updraft_geo::{LatLon, Polygon};
use updraft_units::{Angle, Length, MslAltitude, PressureAltitude};

/// The maximum distance between a normalized curve and its chord.
const MAX_AIRSPACE_CURVE_ERROR: Length = Length::from_meters(1.);

const POLYGON: &[u8] = include_bytes!("../../../testdata/airspace/polygon.txt");
const CIRCLE: &[u8] = include_bytes!("../../../testdata/airspace/circle.txt");
const DB_CLOCKWISE: &[u8] = include_bytes!("../../../testdata/airspace/db_clockwise.txt");
const DB_COUNTERCLOCKWISE: &[u8] =
    include_bytes!("../../../testdata/airspace/db_counterclockwise.txt");
const DA_CLOCKWISE: &[u8] = include_bytes!("../../../testdata/airspace/da_clockwise.txt");
const DA_COUNTERCLOCKWISE: &[u8] =
    include_bytes!("../../../testdata/airspace/da_counterclockwise.txt");
const PARSER_ERROR: &[u8] = include_bytes!("../../../testdata/airspace/parser_error.txt");
const CLASS_TYPES: &[u8] = include_bytes!("../../../testdata/airspace/class_types.txt");
const ALTITUDES: &[u8] = include_bytes!("../../../testdata/airspace/altitudes.txt");
const UNSUPPORTED_ALTITUDE: &[u8] =
    include_bytes!("../../../testdata/airspace/unsupported_altitude.txt");
const LEGACY_NONE: &[u8] = include_bytes!("../../../testdata/airspace/legacy_none.txt");
const ONE_BAD_AIRSPACE: &[u8] = include_bytes!("../../../testdata/airspace/one_bad_airspace.txt");

/// Returns the canonical dataset for valid fixture bytes.
fn parse_fixture(bytes: &[u8]) -> AirspaceDataset {
    assert_ok!(AirspaceDataset::from_openair(bytes))
}

/// Returns the only polygon in a valid one-airspace fixture.
fn only_polygon(bytes: &[u8]) -> Polygon {
    parse_fixture(bytes).airspaces()[0].polygon.clone()
}

/// Returns the vertex bearings from the specified center, in degrees.
fn bearings(center: LatLon, ring: &Polygon) -> Vec<f64> {
    ring.vertices()
        .iter()
        .map(|vertex| center.bearing(*vertex).as_degrees())
        .collect()
}

/// Returns the maximum chord error for a circle with the specified radius and sweep angle.
fn curve_error(radius: Length, sweep: Angle, vertex_count: usize) -> Length {
    let segment_count = vertex_count - 1;
    let step = sweep / segment_count as f64;
    radius * (1. - (step / 2.).cos())
}

/// Checks that equal-angle segments do not exceed the specified chord-error limit.
fn assert_curve_error_bound(
    radius: Length,
    sweep: Angle,
    vertex_count: usize,
    error_budget: Length,
) {
    assert_le!(curve_error(radius, sweep, vertex_count), error_budget);
}

/// Checks that each vertex radius matches linear interpolation within `1e-6` metres.
fn assert_linearly_interpolated_radii(
    center: LatLon,
    ring: &Polygon,
    start_radius: Length,
    end_radius: Length,
) {
    let segment_count = ring.vertices().len() - 1;
    for (index, vertex) in ring.vertices().iter().enumerate() {
        let fraction = index as f64 / segment_count as f64;
        let expected_radius = start_radius + (end_radius - start_radius) * fraction;
        assert_abs_diff_eq!(
            center.distance(*vertex).as_meters(),
            expected_radius.as_meters(),
            epsilon = 1e-6
        );
    }
}

/// Checks that vertex bearings use equal clockwise steps of less than 180 degrees.
fn assert_clockwise(center: LatLon, ring: &Polygon) {
    let bearings = bearings(center, ring);
    let mut expected_step = None;
    for pair in bearings.windows(2) {
        let delta = (pair[1] - pair[0]).rem_euclid(360.);
        assert_gt!(delta, 0.);
        assert_lt!(delta, 180.);
        let expected = *expected_step.get_or_insert(delta);
        assert_abs_diff_eq!(delta, expected, epsilon = 1e-8);
    }
}

/// Checks that vertex bearings use equal counterclockwise steps of less than 180 degrees.
fn assert_counterclockwise(center: LatLon, ring: &Polygon) {
    let bearings = bearings(center, ring);
    let mut expected_step = None;
    for pair in bearings.windows(2) {
        let delta = (pair[0] - pair[1]).rem_euclid(360.);
        assert_gt!(delta, 0.);
        assert_lt!(delta, 180.);
        let expected = *expected_step.get_or_insert(delta);
        assert_abs_diff_eq!(delta, expected, epsilon = 1e-8);
    }
}

/// Verifies that polygon import preserves source order and does not repeat the closing vertex.
#[test]
fn parses_polygon_airspace_without_a_closing_vertex() {
    let dataset = parse_fixture(POLYGON);
    let airspace = &dataset.airspaces()[0];

    assert_eq!(dataset.airspaces().len(), 1);
    assert_eq!(airspace.id.0, 0);
    assert_eq!(airspace.name.as_deref(), Some("Polygon"));
    assert_eq!(airspace.class, AirspaceClass::D);
    assert_eq!(airspace.type_code, AirspaceType::Other);
    assert_eq!(airspace.lower_limit, AirspaceAltitude::Ground);
    assert_eq!(
        airspace.upper_limit,
        AirspaceAltitude::Msl(MslAltitude::new(Length::from_feet(5000.)))
    );
    assert_eq!(
        airspace.polygon.vertices(),
        &[
            LatLon::from_degrees(50., 10.),
            LatLon::from_degrees(50., 10. + 1. / 60.),
            LatLon::from_degrees(50. + 1. / 60., 10. + 1. / 60.),
            LatLon::from_degrees(50. + 1. / 60., 10.),
        ]
    );
}

/// Verifies that circle conversion preserves the radius.
/// Verifies that conversion uses clockwise steps and meets the chord-error limit.
#[test]
fn normalizes_circle_within_the_curve_error_bound() {
    let center = LatLon::from_degrees(50., 10.);
    let radius = Length::from_nautical_miles(2.);
    let ring = only_polygon(CIRCLE);

    assert_ge!(ring.vertices().len(), 3);
    assert_ne!(ring.vertices().first(), ring.vertices().last());
    for vertex in ring.vertices() {
        assert_abs_diff_eq!(
            center.distance(*vertex).as_meters(),
            radius.as_meters(),
            epsilon = 1e-6
        );
    }
    assert_clockwise(center, &ring);
    let ring_bearings = bearings(center, &ring);
    let step = ring_bearings[1] - ring_bearings[0];
    let closing_step = (ring_bearings[0] - ring_bearings[ring_bearings.len() - 1]).rem_euclid(360.);
    assert_abs_diff_eq!(closing_step, step, epsilon = 1e-8);
    assert_curve_error_bound(
        radius,
        Angle::from_degrees(360.),
        ring.vertices().len() + 1,
        MAX_AIRSPACE_CURVE_ERROR,
    );
}

/// Verifies that a small circle still has three vertices.
#[test]
fn uses_at_least_three_vertices_for_a_small_circle() {
    let bytes = b"AC D\nAL GND\nAH FL100\nV X=50:00:00 N 010:00:00 E\nDC 0.0001\n";

    assert_eq!(only_polygon(bytes).vertices().len(), 3);
}

/// Verifies that `DB` conversion preserves endpoints and direction.
/// Verifies that conversion interpolates the radius and meets the chord-error limit.
#[test]
fn normalizes_db_arcs_with_interpolated_radii_exact_endpoints_and_direction() {
    let center = LatLon::from_degrees(50., 10.);
    let start = LatLon::from_degrees(50. + 1. / 60., 10.);
    let end = LatLon::from_degrees(50. - 60.025 / 3600., 10.);
    let start_radius = center.distance(start);
    let end_radius = center.distance(end);
    let maximum_radius = if start_radius > end_radius {
        start_radius
    } else {
        end_radius
    };

    let clockwise = only_polygon(DB_CLOCKWISE);
    assert_eq!(clockwise.vertices().first(), Some(&start));
    assert_eq!(clockwise.vertices().last(), Some(&end));
    assert_clockwise(center, &clockwise);
    assert_linearly_interpolated_radii(center, &clockwise, start_radius, end_radius);
    assert_curve_error_bound(
        maximum_radius,
        Angle::from_degrees(180.),
        clockwise.vertices().len(),
        MAX_AIRSPACE_CURVE_ERROR,
    );

    let counterclockwise = only_polygon(DB_COUNTERCLOCKWISE);
    assert_eq!(counterclockwise.vertices().first(), Some(&start));
    assert_eq!(counterclockwise.vertices().last(), Some(&end));
    assert_counterclockwise(center, &counterclockwise);
    assert_linearly_interpolated_radii(center, &counterclockwise, start_radius, end_radius);
    assert_curve_error_bound(
        maximum_radius,
        Angle::from_degrees(180.),
        counterclockwise.vertices().len(),
        MAX_AIRSPACE_CURVE_ERROR,
    );
}

/// Verifies that `DB` conversion accepts and interpolates a large endpoint-radius difference.
#[test]
fn normalizes_db_arcs_with_large_radius_differences() {
    let bytes = b"AC D\nAL GND\nAH FL100\nV X=50:00:00 N 010:00:00 E\nDB 50:01:00 N 010:00:00 E, 50:00:00 N 010:01:00 E\n";
    let center = LatLon::from_degrees(50., 10.);
    let start = LatLon::from_degrees(50. + 1. / 60., 10.);
    let end = LatLon::from_degrees(50., 10. + 1. / 60.);
    let ring = only_polygon(bytes);

    assert_eq!(ring.vertices().first(), Some(&start));
    assert_eq!(ring.vertices().last(), Some(&end));
    assert_linearly_interpolated_radii(center, &ring, center.distance(start), center.distance(end));
}

/// Verifies that `DA` conversion preserves endpoints, direction, radius, and chord error.
#[test]
fn normalizes_da_arcs_with_exact_endpoints_and_direction() {
    let center = LatLon::from_degrees(50., 10.);
    let radius = Length::from_nautical_miles(1.);
    let start = center.destination(Angle::from_degrees(20.), radius);
    let end = center.destination(Angle::from_degrees(140.), radius);

    let clockwise = only_polygon(DA_CLOCKWISE);
    assert_eq!(clockwise.vertices().first(), Some(&start));
    assert_eq!(clockwise.vertices().last(), Some(&end));
    assert_clockwise(center, &clockwise);
    assert_curve_error_bound(
        radius,
        Angle::from_degrees(120.),
        clockwise.vertices().len(),
        MAX_AIRSPACE_CURVE_ERROR,
    );

    let counterclockwise = only_polygon(DA_COUNTERCLOCKWISE);
    assert_eq!(counterclockwise.vertices().first(), Some(&start));
    assert_eq!(counterclockwise.vertices().last(), Some(&end));
    assert_counterclockwise(center, &counterclockwise);
    assert_curve_error_bound(
        radius,
        Angle::from_degrees(240.),
        counterclockwise.vertices().len(),
        MAX_AIRSPACE_CURVE_ERROR,
    );
}

/// Verifies that a source parser error has no airspace ID.
#[test]
fn separates_source_parser_errors() {
    assert_err_eq!(
        AirspaceDataset::from_openair(PARSER_ERROR),
        AirspaceImportError::Parse {
            airspace_id: None,
            kind: AirspaceParseError::SourceParser("Parse error (unexpected \"ZZ\")".to_string()),
        }
    );
}

/// Verifies the canonical mapping for modern classes and explicit types.
#[test]
fn maps_modern_classes_and_normalized_types() {
    let dataset = parse_fixture(CLASS_TYPES);
    let airspaces = dataset.airspaces();

    assert_eq!(
        airspaces
            .iter()
            .map(|airspace| airspace.id.0)
            .collect::<Vec<_>>(),
        (0..18).collect::<Vec<_>>()
    );
    assert_eq!(
        airspaces[..8]
            .iter()
            .map(|airspace| airspace.class)
            .collect::<Vec<_>>(),
        vec![
            AirspaceClass::A,
            AirspaceClass::B,
            AirspaceClass::C,
            AirspaceClass::D,
            AirspaceClass::E,
            AirspaceClass::F,
            AirspaceClass::G,
            AirspaceClass::Unclassified,
        ]
    );
    assert_eq!(airspaces[0].type_code, AirspaceType::ControlArea);
    assert_eq!(airspaces[4].type_code, AirspaceType::RadioMandatoryZone);
    assert_eq!(airspaces[6].type_code, AirspaceType::Other);
}

/// Verifies that legacy classes become complete OpenAIP classifications.
#[test]
fn normalizes_legacy_classes_to_openaip_classifications() {
    let dataset = parse_fixture(CLASS_TYPES);
    let airspaces = dataset.airspaces();

    assert!(
        airspaces[8..16]
            .iter()
            .all(|airspace| airspace.class == AirspaceClass::Unclassified)
    );
    assert_eq!(
        airspaces[8..16]
            .iter()
            .map(|airspace| airspace.type_code)
            .collect::<Vec<_>>(),
        vec![
            AirspaceType::ControlledTowerRegion,
            AirspaceType::Restricted,
            AirspaceType::Danger,
            AirspaceType::Prohibited,
            AirspaceType::Other,
            AirspaceType::GlidingSector,
            AirspaceType::RadioMandatoryZone,
            AirspaceType::TransponderMandatoryZone,
        ]
    );
    assert_eq!(airspaces[16].class, AirspaceClass::Unclassified);
    assert_eq!(airspaces[16].type_code, AirspaceType::Restricted);
    assert_eq!(airspaces[17].class, AirspaceClass::G);
    assert_eq!(airspaces[17].type_code, AirspaceType::Other);
}

/// Verifies that an unsupported explicit type retains the legacy classification.
#[test]
fn retains_legacy_type_for_explicit_none() {
    let dataset = parse_fixture(LEGACY_NONE);
    let airspace = &dataset.airspaces()[0];

    assert_eq!(airspace.class, AirspaceClass::Unclassified);
    assert_eq!(airspace.type_code, AirspaceType::Restricted);
}

/// Verifies that an empty explicit type becomes `Other` for a modern class.
#[test]
fn maps_an_empty_explicit_type_to_other() {
    let bytes = b"AC D\nAY \nAL GND\nAH FL100\nDP 50:00:00 N 010:00:00 E\nDP 50:00:00 N 010:01:00 E\nDP 50:01:00 N 010:00:00 E\n";
    let dataset = parse_fixture(bytes);
    let airspace = &dataset.airspaces()[0];

    assert_eq!(airspace.class, AirspaceClass::D);
    assert_eq!(airspace.type_code, AirspaceType::Other);
}

/// Verifies that supported altitude forms use the correct typed units.
#[test]
fn maps_supported_altitudes_to_typed_lengths() {
    let dataset = parse_fixture(ALTITUDES);
    let airspaces = dataset.airspaces();

    assert_eq!(airspaces[0].lower_limit, AirspaceAltitude::Ground);
    assert_eq!(
        airspaces[0].upper_limit,
        AirspaceAltitude::Msl(MslAltitude::new(Length::from_feet(5000.)))
    );
    assert_eq!(
        airspaces[1].lower_limit,
        AirspaceAltitude::Agl(Length::from_feet(1000.))
    );
    assert_eq!(
        airspaces[1].upper_limit,
        AirspaceAltitude::FlightLevel(PressureAltitude::new(Length::from_feet(10_000.)))
    );
    assert_eq!(
        airspaces[2].lower_limit,
        AirspaceAltitude::Msl(MslAltitude::new(Length::from_feet(250.)))
    );
    assert_eq!(airspaces[2].upper_limit, AirspaceAltitude::Unlimited);
}

/// Verifies that the importer rejects an unsupported altitude form.
#[test]
fn rejects_unsupported_altitudes() {
    assert_err_eq!(
        AirspaceDataset::from_openair(UNSUPPORTED_ALTITUDE),
        AirspaceImportError::Parse {
            airspace_id: Some(AirspaceId(0)),
            kind: AirspaceParseError::UnsupportedAltitude,
        }
    );
}

/// Verifies that invalid coordinates, radii, and rings return their geometry error kinds.
#[test]
fn rejects_invalid_geometry() {
    let invalid_coordinate = b"AC D\nAL GND\nAH FL100\nDP 90:60:00 N 010:00:00 E\nDP 50:00:00 N 010:01:00 E\nDP 50:01:00 N 010:00:00 E\n";
    let invalid_radius = b"AC D\nAL GND\nAH FL100\nV X=50:00:00 N 010:00:00 E\nDC NaN\n";
    let invalid_ring = b"AC D\nAL GND\nAH FL100\nDP 50:00:00 N 010:00:00 E\nDP 50:00:00 N 010:01:00 E\nDP 50:00:00 N 010:00:00 E\n";

    for (bytes, expected) in [
        (
            invalid_coordinate.as_slice(),
            AirspaceGeometryError::InvalidCoordinate,
        ),
        (
            invalid_radius.as_slice(),
            AirspaceGeometryError::InvalidRadius,
        ),
        (invalid_ring.as_slice(), AirspaceGeometryError::InvalidRing),
    ] {
        assert_matches!(
            AirspaceDataset::from_openair(bytes),
            Err(AirspaceImportError::Geometry {
                airspace_id,
                kind,
            }) if airspace_id == AirspaceId(0) && kind == expected
        );
    }
}

/// Verifies that one invalid airspace rejects the complete source.
#[test]
fn rejects_the_complete_file_when_one_airspace_is_invalid() {
    assert_matches!(
        AirspaceDataset::from_openair(ONE_BAD_AIRSPACE),
        Err(AirspaceImportError::Geometry {
            airspace_id,
            kind: AirspaceGeometryError::InvalidRadius,
        }) if airspace_id == AirspaceId(1)
    );
}

/// Verifies that source parsing finishes before geometry conversion starts.
#[test]
fn collects_parser_results_before_normalizing_geometry() {
    let bytes = b"AC D\nAL GND\nAH FL100\nV X=50:00:00 N 010:00:00 E\nDC 0\nAC D\nAL GND\nAH FL100\nZZ unsupported record\n";

    assert_err_eq!(
        AirspaceDataset::from_openair(bytes),
        AirspaceImportError::Parse {
            airspace_id: None,
            kind: AirspaceParseError::SourceParser("Parse error (unexpected \"ZZ\")".to_string()),
        }
    );
}
