use claims::assert_ok;
use updraft_openaip::reporting_point::ReportingPoint;

const DATASET: &[u8] = include_bytes!("../../../testdata/openaip/reporting_points.json");

#[test]
fn deserializes_the_dataset() {
    let records: Vec<ReportingPoint> = assert_ok!(serde_json::from_slice(DATASET));

    insta::assert_debug_snapshot!(records);
}
