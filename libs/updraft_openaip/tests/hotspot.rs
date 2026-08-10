use claims::assert_ok;
use updraft_openaip::hotspot::Hotspot;

const DATASET: &[u8] = include_bytes!("../../../testdata/openaip/hotspots.json");

#[test]
fn deserializes_the_dataset() {
    let records: Vec<Hotspot> = assert_ok!(serde_json::from_slice(DATASET));

    insta::assert_debug_snapshot!(records);
}
