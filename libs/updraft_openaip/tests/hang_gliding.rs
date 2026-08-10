use claims::assert_ok;
use updraft_openaip::hang_gliding::HangGlidingSite;

const DATASET: &[u8] = include_bytes!("../../../testdata/openaip/hang_gliding_sites.json");

#[test]
fn deserializes_the_dataset() {
    let records: Vec<HangGlidingSite> = assert_ok!(serde_json::from_slice(DATASET));

    insta::assert_debug_snapshot!(records);
}
