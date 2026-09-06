use crate::source_files::{SourceChange, SourceFiles};
use std::{io, path::PathBuf, sync::Arc};
use updraft_airspace::{AirspaceDataset, AirspaceImportError};
use updraft_core::{AirspaceCatalog, AirspaceLoadError};

/// Original OpenAir bytes are stored separately under encoded source names.
#[derive(Clone, Debug)]
pub struct AirspaceStorage {
    files: SourceFiles,
}

impl AirspaceStorage {
    pub fn new(data_directory: impl Into<PathBuf>) -> Self {
        Self {
            files: SourceFiles::new(data_directory.into().join("airspaces"), "txt"),
        }
    }

    pub fn load(&self) -> io::Result<AirspaceCatalog> {
        let entries = match std::fs::read_dir(&self.files.directory) {
            Ok(entries) => entries,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                return Ok(AirspaceCatalog::default());
            }
            Err(error) => return Err(error),
        };
        let mut catalog = AirspaceCatalog::default();
        let mut directories = vec![(entries, String::new())];
        while let Some((entries, prefix)) = directories.pop() {
            for entry in entries {
                let entry = entry?;
                let path = entry.path();
                if entry.file_type()?.is_dir() {
                    let component = entry.file_name().to_string_lossy().into_owned();
                    if component.len() == 250
                        && component.bytes().all(|byte| byte.is_ascii_hexdigit())
                    {
                        match std::fs::read_dir(&path) {
                            Ok(entries) => {
                                directories.push((entries, format!("{prefix}{component}")))
                            }
                            Err(error) => {
                                tracing::warn!(%error, path = %path.display(), "Could not read stored airspace directory")
                            }
                        }
                    }
                    continue;
                }
                if path.extension().and_then(|s| s.to_str()) != Some("txt") {
                    continue;
                }
                let stem = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or_default();
                let name = hex::decode(format!("{prefix}{stem}"))
                    .ok()
                    .and_then(|name| String::from_utf8(name).ok())
                    .ok_or_else(|| {
                        io::Error::new(
                            io::ErrorKind::InvalidData,
                            "Invalid stored airspace filename",
                        )
                    })?;
                let dataset =
                    match std::fs::read(&path) {
                        Ok(bytes) => AirspaceDataset::from_openair(&bytes).map(Arc::new).map_err(
                            |error| {
                                tracing::warn!(%error, "Could not parse stored airspace source");
                                match error {
                                    AirspaceImportError::Parse { .. } => {
                                        AirspaceLoadError::ParseFailed
                                    }
                                    AirspaceImportError::Geometry { .. } => {
                                        AirspaceLoadError::GeometryFailed
                                    }
                                }
                            },
                        ),
                        Err(error) => {
                            tracing::warn!(%error, "Could not read stored airspace source");
                            Err(AirspaceLoadError::ReadFailed)
                        }
                    };
                catalog.sources.insert(name, dataset);
            }
        }
        Ok(catalog)
    }

    pub fn import_airspace(
        &self,
        bytes: &[u8],
        name: &str,
    ) -> Result<(Arc<AirspaceDataset>, SourceChange), AirspaceStorageError> {
        let dataset = Arc::new(AirspaceDataset::from_openair(bytes)?);
        let change = self.files.replace(name, bytes)?;
        Ok((dataset, change))
    }

    pub fn remove(&self, name: &str) -> io::Result<SourceChange> {
        self.files.remove(name)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum AirspaceStorageError {
    #[error(transparent)]
    Import(#[from] AirspaceImportError),
    #[error(transparent)]
    Io(#[from] io::Error),
}

#[cfg(test)]
mod tests {
    use super::*;
    use claims::{assert_err, assert_ok};
    use tempfile::tempdir;
    use tracing_test::traced_test;

    const POLYGON: &[u8] = include_bytes!("../../testdata/airspace/polygon.txt");
    const CIRCLE: &[u8] = include_bytes!("../../testdata/airspace/circle.txt");
    const PARSER_ERROR: &[u8] = include_bytes!("../../testdata/airspace/parser_error.txt");
    const GEOMETRY_ERROR: &[u8] = b"AC D\nAL GND\nAH FL100\nDP 50:00:00 N 010:00:00 E\nDP 50:00:00 N 010:01:00 E\nDP 50:00:00 N 010:00:00 E\n";

    #[test]
    fn reloads_two_sources_without_replacing_the_first_file() {
        let directory = assert_ok!(tempdir());
        let storage = AirspaceStorage::new(directory.path());
        assert_ok!(storage.import_airspace(POLYGON, "a.txt"));
        assert_ok!(storage.import_airspace(CIRCLE, "b.txt"));
        assert_eq!(assert_ok!(storage.load()).sources.len(), 2);
        assert_ok!(storage.import_airspace(CIRCLE, "a.txt"));
        let catalog = assert_ok!(AirspaceStorage::new(directory.path()).load());
        assert_eq!(catalog.sources.len(), 2);
        assert_eq!(
            assert_ok!(std::fs::read(storage.files.path("a.txt"))),
            CIRCLE
        );
        assert_eq!(
            assert_ok!(std::fs::read(storage.files.path("b.txt"))),
            CIRCLE
        );
        assert_eq!(
            assert_ok!(catalog.sources["a.txt"].as_ref()),
            assert_ok!(catalog.sources["b.txt"].as_ref())
        );
    }

    #[test]
    fn old_single_source_files_are_not_loaded_or_changed() {
        let directory = assert_ok!(tempdir());
        let storage = AirspaceStorage::new(directory.path());
        assert_eq!(assert_ok!(storage.load()).sources.len(), 0);
        assert_ok!(std::fs::write(
            directory.path().join("airspace.txt"),
            POLYGON
        ));
        assert_ok!(std::fs::write(
            directory.path().join("airspace.json"),
            b"{}"
        ));
        assert_eq!(assert_ok!(storage.load()).sources.len(), 0);
        assert_ok!(storage.import_airspace(CIRCLE, "airspace.txt"));
        assert_eq!(
            assert_ok!(std::fs::read(directory.path().join("airspace.txt"))),
            POLYGON
        );
    }

    #[test]
    #[traced_test]
    fn invalid_stored_sources_do_not_hide_valid_sources() {
        let directory = assert_ok!(tempdir());
        let storage = AirspaceStorage::new(directory.path());
        assert_ok!(storage.import_airspace(POLYGON, "valid.txt"));
        assert_ok!(std::fs::write(
            storage.files.path("parse.txt"),
            PARSER_ERROR
        ));
        assert_ok!(std::fs::write(
            storage.files.path("geometry.txt"),
            GEOMETRY_ERROR
        ));
        let catalog = assert_ok!(storage.load());
        assert_eq!(catalog.sources.len(), 3);
        assert_ok!(&catalog.sources["valid.txt"]);
        assert_eq!(
            catalog.sources["parse.txt"],
            Err(AirspaceLoadError::ParseFailed)
        );
        assert_eq!(
            catalog.sources["geometry.txt"],
            Err(AirspaceLoadError::GeometryFailed)
        );
        assert!(logs_contain("Could not parse stored airspace source"));
    }

    #[test]
    fn failed_import_and_rollback_preserve_original_bytes() {
        let directory = assert_ok!(tempdir());
        let storage = AirspaceStorage::new(directory.path());
        assert_ok!(storage.import_airspace(POLYGON, "a.txt"));
        assert_ok!(storage.import_airspace(CIRCLE, "b.txt"));
        for bytes in [PARSER_ERROR, GEOMETRY_ERROR] {
            assert_err!(storage.import_airspace(bytes, "a.txt"));
            assert_eq!(
                assert_ok!(std::fs::read(storage.files.path("a.txt"))),
                POLYGON
            );
        }
        let (_, change) = assert_ok!(storage.import_airspace(CIRCLE, "a.txt"));
        assert_ok!(change.rollback());
        assert_eq!(
            assert_ok!(std::fs::read(storage.files.path("a.txt"))),
            POLYGON
        );
        let change = assert_ok!(storage.remove("a.txt"));
        assert_eq!(
            assert_ok!(storage.load())
                .sources
                .keys()
                .collect::<Vec<_>>(),
            vec!["b.txt"]
        );
        assert_ok!(change.rollback());
        assert_eq!(
            assert_ok!(std::fs::read(storage.files.path("a.txt"))),
            POLYGON
        );
        assert_eq!(
            assert_ok!(std::fs::read(storage.files.path("b.txt"))),
            CIRCLE
        );
        let (_, change) = assert_ok!(storage.import_airspace(CIRCLE, "c.txt"));
        assert_ok!(change.rollback());
        assert_eq!(assert_ok!(storage.load()).sources.len(), 2);
    }

    #[test]
    fn filenames_are_exact_and_cannot_escape_storage() {
        let directory = assert_ok!(tempdir());
        let storage = AirspaceStorage::new(directory.path());
        let names = [
            "a.txt".into(),
            "A.txt".into(),
            "../a.txt".into(),
            format!("{}.txt", "ä".repeat(100)),
        ];
        for name in &names {
            assert_ok!(storage.import_airspace(POLYGON, name));
        }
        assert_eq!(assert_ok!(storage.load()).sources.len(), names.len());
        for name in &names {
            let change = assert_ok!(storage.remove(name));
            assert_ok!(change.rollback());
            assert_eq!(assert_ok!(std::fs::read(storage.files.path(name))), POLYGON);
        }
        assert!(!directory.path().join("a.txt").exists());
    }

    #[cfg(unix)]
    #[test]
    #[traced_test]
    fn unreadable_sources_can_be_removed_or_replaced_and_restored() {
        use std::os::unix::fs::PermissionsExt;
        let directory = assert_ok!(tempdir());
        let storage = AirspaceStorage::new(directory.path());
        assert_ok!(storage.import_airspace(CIRCLE, "b.txt"));
        let path = storage.files.path("a.txt");
        for replace in [false, true] {
            assert_ok!(storage.import_airspace(POLYGON, "a.txt"));
            assert_ok!(std::fs::set_permissions(
                &path,
                std::fs::Permissions::from_mode(0o000)
            ));
            let catalog = assert_ok!(storage.load());
            assert_eq!(catalog.sources["a.txt"], Err(AirspaceLoadError::ReadFailed));
            assert_ok!(&catalog.sources["b.txt"]);
            let change = if replace {
                assert_ok!(storage.import_airspace(CIRCLE, "a.txt")).1
            } else {
                assert_ok!(storage.remove("a.txt"))
            };
            assert_ok!(change.rollback());
            assert_eq!(
                assert_ok!(std::fs::metadata(&path)).permissions().mode() & 0o777,
                0
            );
            assert_ok!(std::fs::set_permissions(
                &path,
                std::fs::Permissions::from_mode(0o600)
            ));
            assert_eq!(assert_ok!(std::fs::read(&path)), POLYGON);
        }
        assert!(logs_contain("Could not read stored airspace source"));
    }
    #[cfg(unix)]
    #[test]
    #[traced_test]
    fn inaccessible_source_subtree_does_not_hide_other_sources() {
        use std::os::unix::fs::PermissionsExt;
        let directory = assert_ok!(tempdir());
        let storage = AirspaceStorage::new(directory.path());
        assert_ok!(storage.import_airspace(POLYGON, "valid.txt"));
        let name = format!("{}.txt", "a".repeat(150));
        assert_ok!(storage.import_airspace(POLYGON, &name));
        let path = storage.files.path(&name);
        let parent = path.parent().unwrap();
        assert_ok!(std::fs::set_permissions(
            parent,
            std::fs::Permissions::from_mode(0o000)
        ));
        let loaded = storage.load();
        assert_ok!(std::fs::set_permissions(
            parent,
            std::fs::Permissions::from_mode(0o700)
        ));
        let catalog = assert_ok!(loaded);
        assert_eq!(
            catalog.sources.keys().collect::<Vec<_>>(),
            vec!["valid.txt"]
        );
        assert!(logs_contain("Could not read stored airspace directory"));
        assert_eq!(assert_ok!(storage.load()).sources.len(), 2);
    }
    #[test]
    fn malformed_stored_filenames_fail_catalog_loading() {
        let directory = assert_ok!(tempdir());
        let storage = AirspaceStorage::new(directory.path());
        assert_ok!(storage.import_airspace(POLYGON, "valid.txt"));
        assert_ok!(std::fs::write(
            directory.path().join("airspaces/zz.txt"),
            POLYGON
        ));
        assert_eq!(
            assert_err!(storage.load()).kind(),
            io::ErrorKind::InvalidData
        );
    }
}
