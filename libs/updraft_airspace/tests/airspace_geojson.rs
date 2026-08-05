use claims::{assert_none, assert_ok};
use serde_json::json;
use updraft_airspace::{AirspaceActivity, AirspaceAltitude, AirspaceDataset};
use updraft_units::{Length, PressureAltitude};

const POLYGON: &[u8] = include_bytes!("../../../testdata/airspace/polygon.txt");
const ALTITUDES: &[u8] = include_bytes!("../../../testdata/airspace/altitudes.txt");

#[test]
fn projects_airspace_as_a_closed_geojson_polygon() {
    let dataset = assert_ok!(AirspaceDataset::from_openair(POLYGON));

    insta::assert_json_snapshot!(dataset.airspaces()[0].to_geojson());
}

#[test]
fn projects_required_openaip_classification_properties() {
    let legacy_class = b"AC R\nAL GND\nAH FL100\nDP 50:00:00 N 010:00:00 E\nDP 50:00:00 N 010:01:00 E\nDP 50:01:00 N 010:00:00 E\n";
    let dataset = assert_ok!(AirspaceDataset::from_openair(legacy_class));

    let properties = &dataset.airspaces()[0].to_geojson()["properties"];
    assert_eq!(properties["icaoClass"], json!(8));
    assert_eq!(properties["type"], json!(1));
    assert_none!(properties.get("activity"));
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

#[test]
fn projects_openaip_vertical_limits() {
    let dataset = assert_ok!(AirspaceDataset::from_openair(ALTITUDES));
    let properties = dataset
        .airspaces()
        .iter()
        .map(|airspace| airspace.to_geojson()["properties"].clone())
        .collect::<Vec<_>>();

    insta::assert_json_snapshot!(properties);
}

#[test]
fn projects_optional_openaip_vertical_constraints() {
    let dataset = assert_ok!(AirspaceDataset::from_openair(POLYGON));
    let mut airspace = dataset.airspaces()[0].clone();
    airspace.lower_limit_min = Some(AirspaceAltitude::Agl(Length::from_feet(500.)));
    airspace.upper_limit_max = Some(AirspaceAltitude::FlightLevel(PressureAltitude::new(
        Length::from_feet(12_000.),
    )));

    insta::assert_json_snapshot!(airspace.to_geojson()["properties"]);
}

#[test]
fn projects_openaip_operational_flags() {
    let dataset = assert_ok!(AirspaceDataset::from_openair(POLYGON));
    let mut airspace = dataset.airspaces()[0].clone();
    airspace.on_demand = Some(true);
    airspace.on_request = Some(false);
    airspace.by_notam = Some(true);
    airspace.special_agreement = Some(false);
    airspace.request_compliance = Some(true);

    let properties = &airspace.to_geojson()["properties"];
    assert_eq!(properties["onDemand"], json!(true));
    assert_eq!(properties["onRequest"], json!(false));
    assert_eq!(properties["byNotam"], json!(true));
    assert_eq!(properties["specialAgreement"], json!(false));
    assert_eq!(properties["requestCompliance"], json!(true));
}

#[test]
fn omits_absent_openaip_operational_flags() {
    let dataset = assert_ok!(AirspaceDataset::from_openair(POLYGON));
    let properties = &dataset.airspaces()[0].to_geojson()["properties"];

    for property in [
        "onDemand",
        "onRequest",
        "byNotam",
        "specialAgreement",
        "requestCompliance",
    ] {
        assert_none!(properties.get(property));
    }
}

#[test]
fn projects_one_country_as_a_scalar() {
    let dataset = assert_ok!(AirspaceDataset::from_openair(POLYGON));
    let mut airspace = dataset.airspaces()[0].clone();
    airspace.country_codes = vec!["DE".into()];

    assert_eq!(airspace.to_geojson()["properties"]["country"], json!("DE"));
}

#[test]
fn projects_multiple_countries_as_an_ordered_array() {
    let dataset = assert_ok!(AirspaceDataset::from_openair(POLYGON));
    let mut airspace = dataset.airspaces()[0].clone();
    airspace.country_codes = vec!["DE".into(), "AT".into()];

    assert_eq!(
        airspace.to_geojson()["properties"]["country"],
        json!(["DE", "AT"])
    );
}

#[test]
fn omits_an_empty_country_collection() {
    let dataset = assert_ok!(AirspaceDataset::from_openair(POLYGON));

    assert_none!(dataset.airspaces()[0].to_geojson()["properties"].get("country"));
}

#[test]
fn preserves_an_unrecognized_country_value() {
    let dataset = assert_ok!(AirspaceDataset::from_openair(POLYGON));
    let mut airspace = dataset.airspaces()[0].clone();
    airspace.country_codes = vec!["UNKNOWN".into()];

    assert_eq!(
        airspace.to_geojson()["properties"]["country"],
        json!("UNKNOWN")
    );
}

#[test]
fn projects_openaip_frequencies() {
    let bytes = b"AC D\nAF 123.45\nAG TOWER\nAL GND\nAH FL100\nDP 50:00:00 N 010:00:00 E\nDP 50:00:00 N 010:01:00 E\nDP 50:01:00 N 010:00:00 E\n";
    let dataset = assert_ok!(AirspaceDataset::from_openair(bytes));
    let mut airspace = dataset.airspaces()[0].clone();
    let mut secondary = airspace.frequencies[0].clone();
    secondary.name = None;
    secondary.primary = None;
    secondary.remarks = Some("GUARD".into());
    airspace.frequencies.push(secondary);

    assert_eq!(
        airspace.to_geojson()["properties"]["frequencies"],
        json!([
            {
                "value": "123.450",
                "unit": 2,
                "name": "TOWER",
                "primary": true,
            },
            {
                "value": "123.450",
                "unit": 2,
                "remarks": "GUARD",
            },
        ])
    );
}

#[test]
fn projects_openaip_transponder_settings() {
    let bytes = b"AC D\nAX 123\nAL GND\nAH FL100\nDP 50:00:00 N 010:00:00 E\nDP 50:00:00 N 010:01:00 E\nDP 50:01:00 N 010:00:00 E\n";
    let dataset = assert_ok!(AirspaceDataset::from_openair(bytes));
    let mut airspace = dataset.airspaces()[0].clone();
    let mut secondary = airspace.transponder_settings[0].clone();
    secondary.primary = false;
    secondary.remarks = Some("WHEN ACTIVE".into());
    airspace.transponder_settings.push(secondary);

    assert_eq!(
        airspace.to_geojson()["properties"]["transponderSettings"],
        json!([
            {
                "code": "0123",
                "primary": true,
            },
            {
                "code": "0123",
                "primary": false,
                "remarks": "WHEN ACTIVE",
            },
        ])
    );
}
