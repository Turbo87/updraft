use claims::assert_ok;
use updraft_openaip::obstacle::Obstacle;

const DATASET: &[u8] = include_bytes!("../../../testdata/openaip/obstacles.json");

#[test]
fn deserializes_the_dataset() {
    let records: Vec<Obstacle> = assert_ok!(serde_json::from_slice(DATASET));

    insta::assert_debug_snapshot!(records);
}
