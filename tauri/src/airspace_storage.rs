use crate::source_files::SourceFiles;
use std::{io, path::PathBuf, sync::Arc};
use updraft_airspace::{AirspaceDataset, AirspaceImportError};
use updraft_core::{AirspaceCatalog, AirspaceLoadError, AirspaceSource};

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
        let sources = self.files.entries(|path, error| {
            tracing::warn!(%error, path = %path.display(), "Could not read stored airspace directory");
            Ok(())
        })?;
        let mut catalog = AirspaceCatalog::default();
        for (name, path) in sources {
            let source = if self.disabled_path(&name).try_exists()? {
                AirspaceSource::Disabled
            } else {
                load_airspace(&path)
            };
            catalog.sources.insert(name, source);
        }
        Ok(catalog)
    }

    /// Stores the original bytes even when parsing returns an error.
    pub fn import_airspace(
        &self,
        bytes: &[u8],
        name: &str,
    ) -> io::Result<Result<Arc<AirspaceDataset>, AirspaceLoadError>> {
        self.files.replace(name, bytes)?;
        self.persist_enabled(name, true)?;
        Ok(parse_airspace(bytes))
    }

    pub fn remove(&self, name: &str) -> io::Result<()> {
        self.files.remove(name)?;
        self.persist_enabled(name, true)
    }

    pub fn set_enabled(&self, name: &str, enabled: bool) -> io::Result<AirspaceSource> {
        self.persist_enabled(name, enabled)?;
        Ok(if enabled {
            load_airspace(&self.files.path(name))
        } else {
            AirspaceSource::Disabled
        })
    }

    fn disabled_path(&self, name: &str) -> PathBuf {
        self.files.path(name).with_extension("disabled")
    }

    fn persist_enabled(&self, name: &str, enabled: bool) -> io::Result<()> {
        let path = self.disabled_path(name);
        if !enabled {
            return std::fs::File::create(path)?.sync_all();
        }
        match std::fs::remove_file(path) {
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
            result => result,
        }
    }
}

fn load_airspace(path: &std::path::Path) -> AirspaceSource {
    match std::fs::read(path) {
        Ok(bytes) => parse_airspace(&bytes).into(),
        Err(error) => {
            tracing::warn!(%error, "Could not read stored airspace source");
            AirspaceSource::Unavailable(AirspaceLoadError::ReadFailed)
        }
    }
}

fn parse_airspace(bytes: &[u8]) -> Result<Arc<AirspaceDataset>, AirspaceLoadError> {
    AirspaceDataset::from_openair(bytes)
        .map(Arc::new)
        .map_err(|error| {
            tracing::warn!(%error, "Could not parse stored airspace source");
            match error {
                AirspaceImportError::Parse { .. } => AirspaceLoadError::ParseFailed,
                AirspaceImportError::Geometry { .. } => AirspaceLoadError::GeometryFailed,
            }
        })
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

    #[cfg(unix)]
    #[test]
    #[traced_test]
    fn disabled_sources_are_not_read_at_startup_and_enable_parses_again() {
        use std::os::unix::fs::PermissionsExt;

        let directory = assert_ok!(tempdir());
        let storage = AirspaceStorage::new(directory.path());
        assert_ok!(assert_ok!(storage.import_airspace(POLYGON, "a.txt")));
        assert_eq!(
            assert_ok!(storage.set_enabled("a.txt", false)),
            AirspaceSource::Disabled
        );
        let path = storage.files.path("a.txt");
        assert_ok!(std::fs::write(&path, PARSER_ERROR));
        assert_ok!(std::fs::set_permissions(
            &path,
            std::fs::Permissions::from_mode(0o000)
        ));
        let catalog = assert_ok!(AirspaceStorage::new(directory.path()).load());
        assert_eq!(catalog.sources["a.txt"], AirspaceSource::Disabled);
        assert!(!logs_contain("Could not read stored airspace source"));
        assert!(!logs_contain("Could not parse stored airspace source"));
        assert_ok!(std::fs::set_permissions(
            &path,
            std::fs::Permissions::from_mode(0o600)
        ));
        assert_eq!(
            assert_ok!(storage.set_enabled("a.txt", true)),
            AirspaceSource::Unavailable(AirspaceLoadError::ParseFailed)
        );
        assert_eq!(
            assert_ok!(storage.load()).sources["a.txt"],
            AirspaceSource::Unavailable(AirspaceLoadError::ParseFailed)
        );
        assert!(logs_contain("Could not parse stored airspace source"));
    }

    #[test]
    #[traced_test]
    fn import_enables_disabled_sources_and_removal_clears_activation() {
        let directory = assert_ok!(tempdir());
        let storage = AirspaceStorage::new(directory.path());
        assert_ok!(assert_ok!(storage.import_airspace(POLYGON, "a.txt")));
        assert_ok!(storage.set_enabled("a.txt", false));
        assert_ok!(assert_ok!(storage.import_airspace(CIRCLE, "a.txt")));
        std::assert_matches!(
            assert_ok!(storage.load()).sources["a.txt"],
            AirspaceSource::Active(_)
        );
        assert_ok!(storage.set_enabled("a.txt", false));
        assert_eq!(
            assert_ok!(storage.import_airspace(PARSER_ERROR, "a.txt")),
            Err(AirspaceLoadError::ParseFailed)
        );
        assert_eq!(
            assert_ok!(storage.load()).sources["a.txt"],
            AirspaceSource::Unavailable(AirspaceLoadError::ParseFailed)
        );
        assert_eq!(
            assert_ok!(storage.set_enabled("a.txt", false)),
            AirspaceSource::Disabled
        );
        assert_eq!(
            assert_ok!(storage.load()).sources["a.txt"],
            AirspaceSource::Disabled
        );
        assert!(logs_contain("Could not parse stored airspace source"));
        assert_ok!(storage.remove("a.txt"));
        assert!(assert_ok!(storage.load()).sources.is_empty());
        assert_ok!(assert_ok!(storage.import_airspace(POLYGON, "a.txt")));
        std::assert_matches!(
            assert_ok!(storage.load()).sources["a.txt"],
            AirspaceSource::Active(_)
        );
    }

    #[test]
    fn reloads_two_sources_without_replacing_the_first_file() {
        let directory = assert_ok!(tempdir());
        let storage = AirspaceStorage::new(directory.path());
        assert_ok!(assert_ok!(storage.import_airspace(POLYGON, "a.txt")));
        assert_ok!(assert_ok!(storage.import_airspace(CIRCLE, "b.txt")));
        assert_eq!(assert_ok!(storage.load()).sources.len(), 2);
        assert_ok!(assert_ok!(storage.import_airspace(CIRCLE, "a.txt")));
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
        std::assert_matches!(&catalog.sources["a.txt"], AirspaceSource::Active(_));
        assert_eq!(catalog.sources["a.txt"], catalog.sources["b.txt"]);
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
        assert_ok!(assert_ok!(storage.import_airspace(CIRCLE, "airspace.txt")));
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
        assert_ok!(assert_ok!(storage.import_airspace(POLYGON, "valid.txt")));
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
        std::assert_matches!(&catalog.sources["valid.txt"], AirspaceSource::Active(_));
        assert_eq!(
            catalog.sources["parse.txt"],
            AirspaceSource::Unavailable(AirspaceLoadError::ParseFailed)
        );
        assert_eq!(
            catalog.sources["geometry.txt"],
            AirspaceSource::Unavailable(AirspaceLoadError::GeometryFailed)
        );
        assert!(logs_contain("Could not parse stored airspace source"));
    }

    #[test]
    #[traced_test]
    fn invalid_imports_retain_bytes_and_reload_errors() {
        let directory = assert_ok!(tempdir());
        let storage = AirspaceStorage::new(directory.path());
        assert_ok!(assert_ok!(storage.import_airspace(POLYGON, "a.txt")));
        assert_ok!(assert_ok!(storage.import_airspace(CIRCLE, "b.txt")));
        for (bytes, error) in [
            (PARSER_ERROR, AirspaceLoadError::ParseFailed),
            (GEOMETRY_ERROR, AirspaceLoadError::GeometryFailed),
        ] {
            for name in ["a.txt", "new.txt"] {
                assert_eq!(assert_ok!(storage.import_airspace(bytes, name)), Err(error));
                assert_eq!(assert_ok!(std::fs::read(storage.files.path(name))), bytes);
                let catalog = assert_ok!(AirspaceStorage::new(directory.path()).load());
                assert_eq!(catalog.sources[name], AirspaceSource::Unavailable(error));
                std::assert_matches!(&catalog.sources["b.txt"], AirspaceSource::Active(_));
            }
        }
        assert!(logs_contain("Could not parse stored airspace source"));
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
            assert_ok!(assert_ok!(storage.import_airspace(POLYGON, name)));
        }
        assert_eq!(assert_ok!(storage.load()).sources.len(), names.len());
        for name in &names {
            assert_ok!(storage.remove(name));
            assert!(!storage.files.path(name).exists());
        }
        assert!(!directory.path().join("a.txt").exists());
    }

    #[cfg(unix)]
    #[test]
    #[traced_test]
    fn unreadable_sources_can_be_removed_or_replaced() {
        use std::os::unix::fs::PermissionsExt;
        let directory = assert_ok!(tempdir());
        let storage = AirspaceStorage::new(directory.path());
        assert_ok!(assert_ok!(storage.import_airspace(CIRCLE, "b.txt")));
        let path = storage.files.path("a.txt");
        for replace in [false, true] {
            assert_ok!(assert_ok!(storage.import_airspace(POLYGON, "a.txt")));
            assert_ok!(std::fs::set_permissions(
                &path,
                std::fs::Permissions::from_mode(0o000)
            ));
            let catalog = assert_ok!(storage.load());
            assert_eq!(
                catalog.sources["a.txt"],
                AirspaceSource::Unavailable(AirspaceLoadError::ReadFailed)
            );
            std::assert_matches!(&catalog.sources["b.txt"], AirspaceSource::Active(_));
            if replace {
                assert_ok!(assert_ok!(storage.import_airspace(CIRCLE, "a.txt")));
                assert_eq!(assert_ok!(std::fs::read(&path)), CIRCLE);
            } else {
                assert_ok!(storage.remove("a.txt"));
                assert!(!path.exists());
            }
            assert_eq!(
                assert_ok!(std::fs::read(storage.files.path("b.txt"))),
                CIRCLE
            );
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
        assert_ok!(assert_ok!(storage.import_airspace(POLYGON, "valid.txt")));
        let name = format!("{}.txt", "a".repeat(150));
        assert_ok!(assert_ok!(storage.import_airspace(POLYGON, &name)));
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
        assert_ok!(assert_ok!(storage.import_airspace(POLYGON, "valid.txt")));
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
