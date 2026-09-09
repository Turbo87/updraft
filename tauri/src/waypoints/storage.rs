use crate::source_files::SourceFiles;
use std::{io, path::PathBuf, sync::Arc};
use updraft_core::{WaypointCatalog, WaypointLoadError, WaypointSource};
use updraft_waypoint::WaypointDataset;

/// Original CUP bytes are stored separately under encoded source names.
#[derive(Clone, Debug)]
pub struct WaypointStorage {
    files: SourceFiles,
}

impl WaypointStorage {
    pub fn new(data_directory: PathBuf) -> Self {
        Self {
            files: SourceFiles::new(data_directory.join("waypoints"), "cup"),
        }
    }

    pub fn load(&self) -> io::Result<WaypointCatalog> {
        let mut catalog = WaypointCatalog::default();
        for (name, path) in self.files.entries(|_, error| Err(error))? {
            let source = if self.disabled_path(&name).try_exists()? {
                WaypointSource::Disabled
            } else {
                load_waypoints(&path)
            };
            catalog.sources.insert(name, source);
        }
        Ok(catalog)
    }

    /// Stores the original bytes even when parsing returns an error.
    pub fn import(
        &self,
        name: &str,
        bytes: &[u8],
    ) -> io::Result<Result<Arc<WaypointDataset>, WaypointLoadError>> {
        self.files.replace(name, bytes)?;
        self.persist_enabled(name, true)?;
        Ok(parse_waypoints(bytes))
    }

    pub fn remove(&self, name: &str) -> io::Result<()> {
        self.files.remove(name)?;
        self.persist_enabled(name, true)
    }

    pub fn set_enabled(&self, name: &str, enabled: bool) -> io::Result<WaypointSource> {
        self.persist_enabled(name, enabled)?;
        Ok(if enabled {
            load_waypoints(&self.files.path(name))
        } else {
            WaypointSource::Disabled
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

fn load_waypoints(path: &std::path::Path) -> WaypointSource {
    match std::fs::read(path) {
        Ok(bytes) => parse_waypoints(&bytes).into(),
        Err(error) => {
            tracing::warn!(%error, "Could not read stored waypoint source");
            WaypointSource::Unavailable(WaypointLoadError::ReadFailed)
        }
    }
}

fn parse_waypoints(bytes: &[u8]) -> Result<Arc<WaypointDataset>, WaypointLoadError> {
    WaypointDataset::from_cup(bytes)
        .map(Arc::new)
        .map_err(|error| {
            tracing::warn!(%error, "Could not parse stored waypoint source");
            WaypointLoadError::ParseFailed
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use claims::{assert_err, assert_ok};

    const CUP: &[u8] =
        b"name,code,country,lat,lon,elev,style\nField,,,5000.000N,00600.000E,100m,2\n";

    fn first_waypoint_name<'a>(catalog: &'a WaypointCatalog, name: &str) -> &'a str {
        let WaypointSource::Active(dataset) = &catalog.sources[name] else {
            panic!("an active source")
        };
        &dataset.waypoints()[0].name
    }

    #[cfg(unix)]
    #[test]
    #[tracing_test::traced_test]
    fn disabled_sources_are_not_read_at_startup_and_enable_parses_again() {
        use std::os::unix::fs::PermissionsExt;

        let directory = assert_ok!(tempfile::tempdir());
        let storage = WaypointStorage::new(directory.path().to_owned());
        assert_ok!(assert_ok!(storage.import("a.cup", CUP)));
        assert_eq!(
            assert_ok!(storage.set_enabled("a.cup", false)),
            WaypointSource::Disabled
        );
        let path = storage.files.path("a.cup");
        assert_ok!(std::fs::write(&path, b"invalid"));
        assert_ok!(std::fs::set_permissions(
            &path,
            std::fs::Permissions::from_mode(0o000)
        ));
        let catalog = assert_ok!(WaypointStorage::new(directory.path().to_owned()).load());
        assert_eq!(catalog.sources["a.cup"], WaypointSource::Disabled);
        assert!(!logs_contain("Could not read stored waypoint source"));
        assert!(!logs_contain("Could not parse stored waypoint source"));
        assert_ok!(std::fs::set_permissions(
            &path,
            std::fs::Permissions::from_mode(0o600)
        ));
        assert_eq!(
            assert_ok!(storage.set_enabled("a.cup", true)),
            WaypointSource::Unavailable(WaypointLoadError::ParseFailed)
        );
        assert_eq!(
            assert_ok!(storage.load()).sources["a.cup"],
            WaypointSource::Unavailable(WaypointLoadError::ParseFailed)
        );
        assert!(logs_contain("Could not parse stored waypoint source"));
    }

    #[test]
    #[tracing_test::traced_test]
    fn import_enables_disabled_sources_and_removal_clears_activation() {
        let directory = assert_ok!(tempfile::tempdir());
        let storage = WaypointStorage::new(directory.path().to_owned());
        let bytes = format!(
            "{}Bad,,,bad,00600.000E,0m,1\n",
            String::from_utf8_lossy(CUP)
        );
        let original = assert_ok!(assert_ok!(storage.import("a.cup", bytes.as_bytes())));
        assert_eq!(original.warnings().len(), 1);
        assert_ok!(storage.set_enabled("a.cup", false));
        assert_eq!(
            assert_ok!(storage.set_enabled("a.cup", true)),
            WaypointSource::Active(original)
        );
        assert_ok!(storage.set_enabled("a.cup", false));
        assert_ok!(assert_ok!(storage.import("a.cup", CUP)));
        std::assert_matches!(
            assert_ok!(storage.load()).sources["a.cup"],
            WaypointSource::Active(_)
        );
        assert_ok!(storage.set_enabled("a.cup", false));
        assert_eq!(
            assert_ok!(storage.import("a.cup", b"invalid")),
            Err(WaypointLoadError::ParseFailed)
        );
        assert_eq!(
            assert_ok!(storage.load()).sources["a.cup"],
            WaypointSource::Unavailable(WaypointLoadError::ParseFailed)
        );
        assert_eq!(
            assert_ok!(storage.set_enabled("a.cup", false)),
            WaypointSource::Disabled
        );
        assert_eq!(
            assert_ok!(storage.load()).sources["a.cup"],
            WaypointSource::Disabled
        );
        assert!(logs_contain("Could not parse stored waypoint source"));
        assert_ok!(storage.remove("a.cup"));
        assert!(assert_ok!(storage.load()).sources.is_empty());
        assert_ok!(assert_ok!(storage.import("a.cup", CUP)));
        std::assert_matches!(
            assert_ok!(storage.load()).sources["a.cup"],
            WaypointSource::Active(_)
        );
    }

    #[test]
    fn restores_sources_and_replaces_only_the_matching_name() {
        let dir = assert_ok!(tempfile::tempdir());
        let storage = WaypointStorage::new(dir.path().to_owned());
        assert_ok!(assert_ok!(storage.import("a.cup", CUP)));
        assert_ok!(assert_ok!(storage.import("b.cup", CUP)));
        let replacement = String::from_utf8_lossy(CUP).replace("Field", "Replacement");
        assert_ok!(assert_ok!(storage.import("a.cup", replacement.as_bytes())));
        let catalog = assert_ok!(storage.load());
        assert_eq!(catalog.sources.len(), 2);
        assert_eq!(first_waypoint_name(&catalog, "a.cup"), "Replacement");
        assert_eq!(first_waypoint_name(&catalog, "b.cup"), "Field");
    }

    #[test]
    fn source_names_remain_distinct_on_case_insensitive_filesystems() {
        let dir = assert_ok!(tempfile::tempdir());
        let storage = WaypointStorage::new(dir.path().to_owned());
        assert_ne!(
            storage
                .files
                .path("aaa.cup")
                .to_string_lossy()
                .to_lowercase(),
            storage
                .files
                .path("aaG.cup")
                .to_string_lossy()
                .to_lowercase(),
        );
    }

    #[test]
    fn imports_replaces_and_removes_long_source_names() {
        let dir = assert_ok!(tempfile::tempdir());
        let storage = WaypointStorage::new(dir.path().to_owned());
        for name in [
            format!("{}.cup", "a".repeat(122)),
            format!("{}.cup", "ä".repeat(100)),
        ] {
            assert_ok!(assert_ok!(storage.import(&name, CUP)));
            assert_ok!(assert_ok!(storage.import(&name, CUP)));
            let catalog = assert_ok!(storage.load());
            assert_eq!(first_waypoint_name(&catalog, &name), "Field");
            assert_eq!(assert_ok!(std::fs::read(storage.files.path(&name))), CUP);
            assert_ok!(storage.remove(&name));
            assert_eq!(assert_ok!(storage.load()).sources.len(), 0);
        }
    }

    #[cfg(unix)]
    #[test]
    fn removes_and_replaces_unreadable_sources() {
        use std::os::unix::fs::PermissionsExt;
        let dir = assert_ok!(tempfile::tempdir());
        let storage = WaypointStorage::new(dir.path().to_owned());
        let path = storage.files.path("a.cup");
        let replacement = String::from_utf8_lossy(CUP).replace("Field", "Replacement");
        for replace in [false, true] {
            assert_ok!(assert_ok!(storage.import("a.cup", CUP)));
            assert_ok!(std::fs::set_permissions(
                &path,
                std::fs::Permissions::from_mode(0o000)
            ));
            assert_err!(std::fs::read(&path));
            if replace {
                assert_ok!(assert_ok!(storage.import("a.cup", replacement.as_bytes())));
                assert_eq!(assert_ok!(std::fs::read(&path)), replacement.as_bytes());
            } else {
                assert_ok!(storage.remove("a.cup"));
                assert!(!path.exists());
            }
        }
    }

    #[test]
    #[tracing_test::traced_test]
    fn invalid_imports_retain_bytes_and_reload_errors() {
        let dir = assert_ok!(tempfile::tempdir());
        let storage = WaypointStorage::new(dir.path().to_owned());
        assert_ok!(assert_ok!(storage.import("../a.cup", CUP)));
        assert_ok!(assert_ok!(storage.import("other.cup", CUP)));
        for name in ["../a.cup", "new.cup"] {
            let source = assert_ok!(storage.import(name, b"invalid"));
            assert_eq!(source, Err(WaypointLoadError::ParseFailed));
            let stored = assert_ok!(std::fs::read(storage.files.path(name)));
            assert_eq!(stored, b"invalid");
            let catalog = assert_ok!(WaypointStorage::new(dir.path().to_owned()).load());
            assert_eq!(
                catalog.sources[name],
                WaypointSource::Unavailable(WaypointLoadError::ParseFailed)
            );
            std::assert_matches!(&catalog.sources["other.cup"], WaypointSource::Active(_));
        }
        assert!(logs_contain("Could not parse stored waypoint source"));
    }
    #[cfg(unix)]
    #[test]
    fn an_unreadable_source_subtree_fails_catalog_loading() {
        use std::os::unix::fs::PermissionsExt;
        let directory = assert_ok!(tempfile::tempdir());
        let storage = WaypointStorage::new(directory.path().to_owned());
        let name = format!("{}.cup", "a".repeat(150));
        assert_ok!(assert_ok!(storage.import(&name, CUP)));
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
        assert_eq!(assert_err!(loaded).kind(), io::ErrorKind::PermissionDenied);
        assert_eq!(assert_ok!(storage.load()).sources.len(), 1);
    }

    #[test]
    fn malformed_stored_filenames_fail_catalog_loading() {
        let directory = assert_ok!(tempfile::tempdir());
        let storage = WaypointStorage::new(directory.path().to_owned());
        assert_ok!(assert_ok!(storage.import("valid.cup", CUP)));
        assert_ok!(std::fs::write(
            directory.path().join("waypoints/zz.cup"),
            CUP
        ));
        assert_eq!(
            assert_err!(storage.load()).kind(),
            io::ErrorKind::InvalidData
        );
    }
}
