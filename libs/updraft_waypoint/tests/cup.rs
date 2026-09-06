use claims::{assert_err, assert_none, assert_ok, assert_some_eq};
use updraft_geo::LatLon;
use updraft_units::Length;
use updraft_waypoint::{WaypointDataset, WaypointKind};

const HEADER: &str = "name,code,country,lat,lon,elev,style,rwdir,rwlen,freq,desc\n";
const FIELD: &str = "Field,EDXX,DE,5000.000N,00600.000E,100m,2,90,800m,123.500,Notes\n";

#[test]
fn imports_valid_rows_and_retains_skipped_row_diagnostics() {
    let source = format!("{HEADER}{FIELD}Bad,,,invalid,00600.000E,0m,1\n{FIELD}");
    let dataset = assert_ok!(WaypointDataset::from_cup(source.as_bytes()));
    assert_eq!(dataset.waypoints().len(), 2);
    assert_some_eq!(dataset.warnings()[0].line, 3);
    let point = &dataset.waypoints()[0];
    assert_eq!(point.name, "Field");
    assert_eq!(point.kind, WaypointKind::GrassAirfield);
    assert_eq!(point.position, LatLon::from_degrees(50.0, 6.0));
}

#[test]
fn rejects_invalid_headers_and_empty_results() {
    assert_err!(WaypointDataset::from_cup(b"wrong,header\n"));
    assert_err!(WaypointDataset::from_cup(HEADER.as_bytes()));
    let source = format!("{HEADER}Bad,,,invalid,00600.000E,0m,1\n");
    assert_err!(WaypointDataset::from_cup(source.as_bytes()));
}

#[test]
fn propagates_cup_parser_errors() {
    let source =
        format!("{HEADER}{FIELD}-----Related Tasks-----\nTask,Field\nOptions,NearDis=invalid\n");
    assert_err!(WaypointDataset::from_cup(source.as_bytes()));
}

#[test]
fn retains_waypoints_with_invalid_optional_fields() {
    let source = format!("{HEADER}{}", FIELD.replace(",90,", ",invalid,"));
    let dataset = assert_ok!(WaypointDataset::from_cup(source.as_bytes()));
    assert_eq!(dataset.waypoints().len(), 1);
    assert_eq!(dataset.warnings().len(), 1);
}

#[test]
fn maps_elevation_to_mean_sea_level_meters() {
    let source = format!("{HEADER}{}", FIELD.replace("100m", "1000ft"));
    let dataset = assert_ok!(WaypointDataset::from_cup(source.as_bytes()));
    let elevation = dataset.waypoints()[0].elevation.into_inner();
    assert_eq!(elevation.as_meters(), 304.8);
}

#[test]
fn maps_runway_dimensions_and_direction() {
    let source = format!("{HEADER}{FIELD}");
    let dataset = assert_ok!(WaypointDataset::from_cup(source.as_bytes()));
    let point = &dataset.waypoints()[0];
    assert_some_eq!(point.runway_direction, 90);
    assert_some_eq!(point.runway_length, Length::from_meters(800.0));
    assert_none!(point.runway_width);
}

#[test]
fn retains_radio_frequency_text() {
    let source = format!("{HEADER}{FIELD}");
    let dataset = assert_ok!(WaypointDataset::from_cup(source.as_bytes()));
    assert_eq!(dataset.waypoints()[0].frequency, "123.500");
}

#[test]
fn retains_multiline_notes() {
    let source = format!("{HEADER}{}", FIELD.replace("Notes", "\"First\nLast\""));
    let dataset = assert_ok!(WaypointDataset::from_cup(source.as_bytes()));
    assert_eq!(dataset.waypoints()[0].notes, "First\nLast");
}

#[test]
#[should_panic(expected = "PosOverflow")]
fn cup_dependency_panics_when_longitude_degrees_exceed_u8() {
    let source = format!("{HEADER}{FIELD}Bad,,,5000.000N,99900.000E,0m,1\n");
    let _ = WaypointDataset::from_cup(source.as_bytes());
}
