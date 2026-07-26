use crate::topic::Topic;
use std::path::{Path, PathBuf};
use ts_rs::TS as _;

/// Directory holding the TypeScript bindings committed for the frontend.
pub fn committed_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../frontend/src/lib/protocol/generated")
}

/// Writes the TypeScript bindings derived from the wire types.
pub fn generate(output_dir: &Path) -> std::io::Result<()> {
    match std::fs::remove_dir_all(output_dir) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error),
    }
    std::fs::create_dir_all(output_dir)?;

    let config = ts_rs::Config::new().with_out_dir(output_dir);
    Topic::export_all(&config).map_err(std::io::Error::other)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{committed_dir, generate};
    use claims::{assert_ok, assert_some};
    use std::collections::BTreeMap;
    use std::path::Path;

    const REGENERATE_COMMAND: &str =
        "cargo run -p updraft_core --features ts --example generate_protocol_bindings";

    fn read_dir_files(dir: &Path) -> BTreeMap<String, String> {
        let entries = assert_ok!(std::fs::read_dir(dir), "failed to read {}", dir.display());

        entries
            .map(|entry| {
                let path = assert_ok!(entry).path();
                let name = assert_some!(path.file_name())
                    .to_string_lossy()
                    .into_owned();
                let contents = assert_ok!(
                    std::fs::read_to_string(&path),
                    "failed to read {}",
                    path.display()
                );
                (name, contents)
            })
            .collect()
    }

    #[test]
    fn committed_bindings_are_up_to_date() {
        let generated = assert_ok!(tempfile::tempdir());
        assert_ok!(generate(generated.path()));

        let committed = read_dir_files(&committed_dir());
        let regenerated = read_dir_files(generated.path());

        assert_eq!(
            committed.keys().collect::<Vec<_>>(),
            regenerated.keys().collect::<Vec<_>>(),
            "committed TypeScript bindings are out of date, run `{REGENERATE_COMMAND}`"
        );

        for (name, committed) in committed {
            let regenerated = assert_some!(regenerated.get(&name));
            assert_eq!(
                committed, *regenerated,
                "committed TypeScript binding {name} is out of date, run `{REGENERATE_COMMAND}`"
            );
        }
    }
}
