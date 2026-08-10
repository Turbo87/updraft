use claims::assert_ok;
use updraft_openaip::navaid::Navaid;

const DATASET: &[u8] = include_bytes!("../../../testdata/openaip/navaids.json");

#[test]
fn deserializes_the_dataset() {
    let records: Vec<Navaid> = assert_ok!(serde_json::from_slice(DATASET));

    insta::assert_debug_snapshot!(records);
}
