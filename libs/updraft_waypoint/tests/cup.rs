use claims::{assert_err, assert_ok, assert_some_eq};
use updraft_waypoint::{WaypointDataset, WaypointKind};

const HEADER: &str = "name,code,country,lat,lon,elev,style,rwdir,rwlen,freq,desc\n";
const FIELD: &str = "Field,EDXX,DE,5000.000N,00600.000E,100m,2,90,800m,123.500,Notes\n";

#[test]
fn imports_valid_rows_and_retains_skipped_row_diagnostics() {
    let source = format!("{HEADER}{FIELD}Bad,,,invalid,00600.000E,0m,1\n{FIELD}");
    let dataset = assert_ok!(WaypointDataset::from_cup(source.as_bytes()));
    assert_eq!(dataset.waypoints().len(), 2);
    assert_some_eq!(dataset.warnings()[0].line, 3);
    assert_eq!(dataset.waypoints()[0].name, "Field");
    assert_eq!(dataset.waypoints()[0].kind, WaypointKind::GrassAirfield);
    assert_eq!(
        dataset.waypoints()[0].position,
        updraft_geo::LatLon::from_degrees(50.0, 6.0)
    );
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
    assert_eq!(
        dataset.waypoints()[0].elevation.into_inner().as_meters(),
        304.8
    );
}

#[test]
fn maps_runway_dimensions_and_direction() {
    let source = format!("{HEADER}{FIELD}");
    let dataset = assert_ok!(WaypointDataset::from_cup(source.as_bytes()));
    let point = &dataset.waypoints()[0];
    claims::assert_some_eq!(point.runway_direction, 90);
    claims::assert_some_eq!(
        point.runway_length,
        updraft_units::Length::from_meters(800.0)
    );
    claims::assert_none!(point.runway_width);
}
