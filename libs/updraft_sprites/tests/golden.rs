//! Verifies that the committed spritesheet matches the SVG sources. `spreet`
//! renders deterministically, so a byte comparison is stable across machines.

use std::collections::BTreeMap;
use std::path::Path;
use updraft_sprites::{committed_output_dir, generate, svg_source_dir};

/// Reads every file in `dir` into a map of file name to contents.
fn read_dir_files(dir: &Path) -> BTreeMap<String, Vec<u8>> {
    std::fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", dir.display()))
        .map(|entry| {
            let path = entry.expect("failed to read dir entry").path();
            let name = path
                .file_name()
                .expect("dir entry has no file name")
                .to_string_lossy()
                .into_owned();
            (name, std::fs::read(&path).expect("failed to read file"))
        })
        .collect()
}

#[test]
fn committed_spritesheet_is_up_to_date() {
    let generated = tempfile::tempdir().expect("failed to create temp dir");
    generate(&svg_source_dir(), generated.path()).expect("failed to generate spritesheet");

    let committed = read_dir_files(&committed_output_dir());
    let regenerated = read_dir_files(generated.path());

    assert_eq!(
        committed, regenerated,
        "committed spritesheet is out of date, run `cargo run -p updraft_sprites`"
    );
}
