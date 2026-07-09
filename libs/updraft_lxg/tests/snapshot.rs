//! Snapshot of a decoded real-world preset, so changes to the parsed
//! structure are reviewed deliberately.

use updraft_lxg::LxgFile;

const DG800B: &[u8] = include_bytes!("fixtures/dg-800b.lxg");

#[test]
fn decoded_dg800b_matches_snapshot() {
    let file = LxgFile::from_bytes(DG800B).expect("fixture should decode");
    insta::assert_debug_snapshot!(file);
}
