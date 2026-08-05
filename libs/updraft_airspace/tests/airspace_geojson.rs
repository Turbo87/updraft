use claims::assert_ok;
use serde_json::json;
use updraft_airspace::AirspaceDataset;

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
            "class": 8,
            "type": 1,
        })
    );
}
