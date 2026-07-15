use std::path::{Path, PathBuf};

use anyhow::Context as _;
use ts_rs::TS as _;

use super::{Change, Snapshot};

/// Directory containing the TypeScript bindings committed for the frontend.
pub fn committed_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../frontend/src/lib/protocol/generated")
}

/// Generates the TypeScript bindings derived from the wire types.
pub fn generate(output_dir: &Path) -> anyhow::Result<()> {
    match std::fs::remove_dir_all(output_dir) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => {
            return Err(error).with_context(|| {
                format!(
                    "failed to remove generated bindings directory {}",
                    output_dir.display()
                )
            });
        }
    }

    std::fs::create_dir_all(output_dir).with_context(|| {
        format!(
            "failed to create generated bindings directory {}",
            output_dir.display()
        )
    })?;

    let config = ts_rs::Config::new().with_out_dir(output_dir);
    Snapshot::export_all(&config).context("failed to generate snapshot bindings")?;
    Change::export_all(&config).context("failed to generate change bindings")?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::path::Path;

    use claims::{assert_ok, assert_ok_eq, assert_some};

    use super::{committed_dir, generate};

    const REGENERATE_COMMAND: &str =
        "cargo run -p updraft_server --features ts --example generate_protocol_bindings";

    fn read_dir_files(dir: &Path) -> BTreeMap<String, String> {
        let entries = std::fs::read_dir(dir)
            .unwrap_or_else(|error| panic!("failed to read {}: {error}", dir.display()));

        entries
            .map(|entry| {
                let entry = assert_ok!(entry, "failed to read entry in {}", dir.display());
                let path = entry.path();
                let name = assert_some!(
                    path.file_name(),
                    "directory entry has no file name: {}",
                    path.display()
                )
                .to_string_lossy()
                .into_owned();
                let contents = assert_ok!(
                    std::fs::read_to_string(&path),
                    "failed to read generated binding {}",
                    path.display()
                );
                (name, contents)
            })
            .collect()
    }

    #[test]
    fn committed_bindings_are_up_to_date() {
        let generated = tempfile::tempdir().expect("failed to create temporary bindings directory");
        assert_ok_eq!(generate(generated.path()), ());

        let committed = read_dir_files(&committed_dir());
        let regenerated = read_dir_files(generated.path());

        assert_eq!(
            committed.keys().collect::<Vec<_>>(),
            regenerated.keys().collect::<Vec<_>>(),
            "committed TypeScript binding files are out of date, run `{REGENERATE_COMMAND}`"
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
