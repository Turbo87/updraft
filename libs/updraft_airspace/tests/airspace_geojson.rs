use claims::{assert_none, assert_ok};
use serde_json::json;
use updraft_airspace::{AirspaceActivity, AirspaceDataset};

const POLYGON: &[u8] = include_bytes!("../../../testdata/airspace/polygon.txt");

#[test]
fn projects_airspace_as_a_closed_geojson_polygon() {
    let dataset = assert_ok!(AirspaceDataset::from_openair(POLYGON));

    insta::assert_json_snapshot!(dataset.airspaces()[0].to_geojson());
}

#[test]
fn projects_required_openaip_classification_properties() {
    let legacy_class = b"AC R\nAL GND\nAH FL100\nDP 50:00:00 N 010:00:00 E\nDP 50:00:00 N 010:01:00 E\nDP 50:01:00 N 010:00:00 E\n";
    let dataset = assert_ok!(AirspaceDataset::from_openair(legacy_class));

    assert_eq!(
        dataset.airspaces()[0].to_geojson()["properties"],
        json!({
            "icaoClass": 8,
            "type": 1,
        })
    );
}

#[test]
fn projects_optional_openaip_activity() {
    let dataset = assert_ok!(AirspaceDataset::from_openair(POLYGON));
    let mut airspace = dataset.airspaces()[0].clone();
    airspace.activity = Some(AirspaceActivity::HangGlidingOrParagliding);

    assert_eq!(airspace.to_geojson()["properties"]["activity"], json!(5));
}

#[test]
fn projects_optional_airspace_name() {
    let dataset = assert_ok!(AirspaceDataset::from_openair(POLYGON));

    assert_eq!(
        dataset.airspaces()[0].to_geojson()["properties"]["name"],
        json!("Polygon")
    );
}

#[test]
fn omits_absent_airspace_name() {
    let dataset = assert_ok!(AirspaceDataset::from_openair(POLYGON));
    let mut airspace = dataset.airspaces()[0].clone();
    airspace.name = None;

    assert_none!(airspace.to_geojson()["properties"].get("name"));
}
