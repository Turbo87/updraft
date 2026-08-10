use claims::assert_ok;
use updraft_openaip::airspace::Airspace;

const AIRSPACES: &[u8] = include_bytes!("../../../testdata/openaip/airspaces.json");
const MULTI_RING: &[u8] = include_bytes!("../../../testdata/openaip/airspace_multi_ring.json");

#[test]
fn deserializes_the_dataset() {
    let airspaces: Vec<Airspace> = assert_ok!(serde_json::from_slice(AIRSPACES));

    insta::assert_debug_snapshot!(airspaces);
}

#[test]
fn accepts_more_than_one_polygon_ring() {
    let airspaces: Vec<Airspace> = assert_ok!(serde_json::from_slice(MULTI_RING));

    let rings: Vec<Vec<usize>> = airspaces
        .iter()
        .map(|airspace| airspace.geometry.coordinates.iter().map(Vec::len).collect())
        .collect();
    assert_eq!(rings, vec![vec![158, 132]]);
}
