use std::{
    io::{self, Write},
    path::PathBuf,
    sync::Arc,
};
use tempfile::{NamedTempFile, TempPath};
use updraft_airspace::{AirspaceDataset, AirspaceImportError};
use updraft_core::{AirspaceCatalog, AirspaceLoadError};

/// Original OpenAir bytes are stored separately under encoded source names.
#[derive(Clone, Debug)]
pub struct AirspaceStorage {
    directory: PathBuf,
}

/// Restores the previous source if core activation fails.
#[derive(Debug)]
pub struct AirspaceSourceChange {
    path: PathBuf,
    previous: Option<TempPath>,
}

impl AirspaceSourceChange {
    pub fn rollback(self) -> io::Result<()> {
        match self.previous {
            Some(previous) => previous.persist(self.path).map_err(|e| e.error),
            None => std::fs::remove_file(self.path),
        }
    }
}

impl AirspaceStorage {
    pub fn new(data_directory: impl Into<PathBuf>) -> Self {
        Self {
            directory: data_directory.into().join("airspaces"),
        }
    }

    fn source_path(&self, name: &str) -> PathBuf {
        let encoded = hex::encode(name);
        let mut path = self.directory.clone();
        for chunk in encoded.as_bytes().chunks(250) {
            path.push(std::str::from_utf8(chunk).expect("Hexadecimal names are ASCII"));
        }
        path.set_extension("txt");
        path
    }

    pub fn load(&self) -> io::Result<AirspaceCatalog> {
        let entries = match std::fs::read_dir(&self.directory) {
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
    ) -> Result<(Arc<AirspaceDataset>, AirspaceSourceChange), AirspaceStorageError> {
        let dataset = Arc::new(AirspaceDataset::from_openair(bytes)?);
        let path = self.source_path(name);
        std::fs::create_dir_all(
            path.parent()
                .expect("Airspace sources have a parent directory"),
        )?;
        let previous = self.backup(&path)?;
        self.prepare(bytes)?
            .persist(&path)
            .map_err(|error| error.error)?;
        Ok((dataset, AirspaceSourceChange { path, previous }))
    }

    pub fn remove(&self, name: &str) -> io::Result<AirspaceSourceChange> {
        let path = self.source_path(name);
        let previous = self.backup(&path)?;
        std::fs::remove_file(&path)?;
        Ok(AirspaceSourceChange { path, previous })
    }

    fn backup(&self, path: &std::path::Path) -> io::Result<Option<TempPath>> {
        let backup = NamedTempFile::new_in(&self.directory)?.into_temp_path();
        std::fs::remove_file(&backup)?;
        match std::fs::hard_link(path, &backup) {
            Ok(()) => Ok(Some(backup)),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error),
        }
    }

    fn prepare(&self, bytes: &[u8]) -> io::Result<NamedTempFile> {
        let mut file = NamedTempFile::new_in(&self.directory)?;
        file.write_all(bytes)?;
        file.as_file().sync_all()?;
        Ok(file)
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
            assert_ok!(std::fs::read(storage.source_path("a.txt"))),
            CIRCLE
        );
        assert_eq!(
            assert_ok!(std::fs::read(storage.source_path("b.txt"))),
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
            storage.source_path("parse.txt"),
            PARSER_ERROR
        ));
        assert_ok!(std::fs::write(
            storage.source_path("geometry.txt"),
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
                assert_ok!(std::fs::read(storage.source_path("a.txt"))),
                POLYGON
            );
        }
        let (_, change) = assert_ok!(storage.import_airspace(CIRCLE, "a.txt"));
        assert_ok!(change.rollback());
        assert_eq!(
            assert_ok!(std::fs::read(storage.source_path("a.txt"))),
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
            assert_ok!(std::fs::read(storage.source_path("a.txt"))),
            POLYGON
        );
        assert_eq!(
            assert_ok!(std::fs::read(storage.source_path("b.txt"))),
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
            assert_eq!(
                assert_ok!(std::fs::read(storage.source_path(name))),
                POLYGON
            );
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
        let path = storage.source_path("a.txt");
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
        let path = storage.source_path(&name);
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
}
