#![cfg(feature = "gen")]

use updraft_egm96::downsample::{committed_grid_path, downsample, source_path};

#[test]
fn committed_grid_is_up_to_date() {
    let dac = std::fs::read(source_path()).expect("read WW15MGH.DAC source");
    let regenerated = downsample(&dac);
    let committed = std::fs::read(committed_grid_path()).expect("read committed grid");

    assert_eq!(
        committed, regenerated,
        "committed grid is stale. run `cargo run -p updraft_egm96 --features gen` to regenerate."
    );
}
