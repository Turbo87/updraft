use std::{
    io::{self, Write},
    path::PathBuf,
    sync::Arc,
};
use tempfile::{NamedTempFile, TempPath};
use updraft_core::{WaypointCatalog, WaypointLoadError};
use updraft_waypoint::WaypointDataset;

/// Original CUP bytes are stored separately under encoded source names.
#[derive(Clone, Debug)]
pub struct WaypointStorage {
    directory: PathBuf,
}

/// Restores the previous source if core activation fails.
#[derive(Debug)]
pub struct WaypointSourceChange {
    path: PathBuf,
    previous: Option<TempPath>,
}

impl WaypointSourceChange {
    pub fn rollback(self) -> io::Result<()> {
        match self.previous {
            Some(previous) => previous.persist(self.path).map_err(|e| e.error),
            None => std::fs::remove_file(self.path),
        }
    }
}

impl WaypointStorage {
    pub fn new(data_directory: PathBuf) -> Self {
        Self {
            directory: data_directory.join("waypoints"),
        }
    }

    fn source_path(&self, name: &str) -> PathBuf {
        let encoded = hex::encode(name);
        let mut path = self.directory.clone();
        for chunk in encoded.as_bytes().chunks(250) {
            path.push(std::str::from_utf8(chunk).expect("Hexadecimal names are ASCII"));
        }
        path.set_extension("cup");
        path
    }

    pub fn load(&self) -> io::Result<WaypointCatalog> {
        let entries = match std::fs::read_dir(&self.directory) {
            Ok(entries) => entries,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                return Ok(WaypointCatalog::default());
            }
            Err(error) => return Err(error),
        };
        let mut catalog = WaypointCatalog::default();
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
                        directories
                            .push((std::fs::read_dir(path)?, format!("{prefix}{component}")));
                    }
                    continue;
                }
                if path.extension().and_then(|s| s.to_str()) != Some("cup") {
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
                            "Invalid stored waypoint filename",
                        )
                    })?;
                let dataset = match std::fs::read(&path) {
                    Ok(bytes) => WaypointDataset::from_cup(&bytes)
                        .map(Arc::new)
                        .map_err(|error| {
                            tracing::warn!(%error, "Could not parse stored waypoint source");
                            WaypointLoadError::ParseFailed
                        }),
                    Err(error) => {
                        tracing::warn!(%error, "Could not read stored waypoint source");
                        Err(WaypointLoadError::ReadFailed)
                    }
                };
                catalog.sources.insert(name, dataset);
            }
        }
        Ok(catalog)
    }

    pub fn import(
        &self,
        name: &str,
        bytes: &[u8],
    ) -> anyhow::Result<(Arc<WaypointDataset>, WaypointSourceChange)> {
        let dataset = Arc::new(WaypointDataset::from_cup(bytes)?);
        let path = self.source_path(name);
        std::fs::create_dir_all(
            path.parent()
                .expect("Waypoint sources have a parent directory"),
        )?;
        let previous = self.backup(&path)?;
        self.prepare(bytes)?.persist(&path)?;
        Ok((dataset, WaypointSourceChange { path, previous }))
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

#[cfg(test)]
mod tests {
    use super::*;
    use claims::{assert_err, assert_ok};

    const CUP: &[u8] =
        b"name,code,country,lat,lon,elev,style\nField,,,5000.000N,00600.000E,100m,2\n";

    #[test]
    fn restores_sources_and_replaces_only_the_matching_name() {
        let dir = assert_ok!(tempfile::tempdir());
        let storage = WaypointStorage::new(dir.path().to_owned());
        assert_ok!(storage.import("a.cup", CUP));
        assert_ok!(storage.import("b.cup", CUP));
        let replacement = String::from_utf8_lossy(CUP).replace("Field", "Replacement");
        assert_ok!(storage.import("a.cup", replacement.as_bytes()));
        let catalog = assert_ok!(storage.load());
        assert_eq!(catalog.sources.len(), 2);
        assert_eq!(
            assert_ok!(catalog.sources["a.cup"].as_ref()).waypoints()[0].name,
            "Replacement"
        );
        assert_eq!(
            assert_ok!(catalog.sources["b.cup"].as_ref()).waypoints()[0].name,
            "Field"
        );
    }

    #[test]
    fn source_names_remain_distinct_on_case_insensitive_filesystems() {
        let dir = assert_ok!(tempfile::tempdir());
        let storage = WaypointStorage::new(dir.path().to_owned());
        assert_ne!(
            storage
                .source_path("aaa.cup")
                .to_string_lossy()
                .to_lowercase(),
            storage
                .source_path("aaG.cup")
                .to_string_lossy()
                .to_lowercase(),
        );
    }

    #[test]
    fn imports_and_replaces_long_source_names() {
        let dir = assert_ok!(tempfile::tempdir());
        let storage = WaypointStorage::new(dir.path().to_owned());
        for name in [
            format!("{}.cup", "a".repeat(122)),
            format!("{}.cup", "ä".repeat(100)),
        ] {
            assert_ok!(storage.import(&name, CUP));
            assert_ok!(storage.import(&name, CUP));
            let catalog = assert_ok!(storage.load());
            assert_eq!(
                assert_ok!(catalog.sources[&name].as_ref()).waypoints()[0].name,
                "Field"
            );
            assert_eq!(assert_ok!(std::fs::read(storage.source_path(&name))), CUP);
        }
    }

    #[cfg(unix)]
    #[test]
    fn replaces_unreadable_sources_with_rollback() {
        use std::os::unix::fs::PermissionsExt;
        let dir = assert_ok!(tempfile::tempdir());
        let storage = WaypointStorage::new(dir.path().to_owned());
        let path = storage.source_path("a.cup");
        let replacement = String::from_utf8_lossy(CUP).replace("Field", "Replacement");
        assert_ok!(storage.import("a.cup", CUP));
        assert_ok!(std::fs::set_permissions(
            &path,
            std::fs::Permissions::from_mode(0o000)
        ));
        assert_err!(std::fs::read(&path));
        let (_, change) = assert_ok!(storage.import("a.cup", replacement.as_bytes()));
        assert_eq!(assert_ok!(std::fs::read(&path)), replacement.as_bytes());
        assert_ok!(change.rollback());
        assert_eq!(
            assert_ok!(std::fs::metadata(&path)).permissions().mode() & 0o777,
            0
        );
        assert_ok!(std::fs::set_permissions(
            &path,
            std::fs::Permissions::from_mode(0o600)
        ));
        assert_eq!(assert_ok!(std::fs::read(&path)), CUP);
    }

    #[test]
    fn failed_import_and_rollback_preserve_original_bytes() {
        let dir = assert_ok!(tempfile::tempdir());
        let storage = WaypointStorage::new(dir.path().to_owned());
        assert_ok!(storage.import("../a.cup", CUP));
        assert_err!(storage.import("../a.cup", b"invalid"));
        let replacement = String::from_utf8_lossy(CUP).replace("Field", "Replacement");
        let (_, change) = assert_ok!(storage.import("../a.cup", replacement.as_bytes()));
        assert_ok!(change.rollback());
        assert_eq!(
            assert_ok!(std::fs::read(storage.source_path("../a.cup"))),
            CUP
        );
        assert_eq!(assert_ok!(storage.load()).sources.len(), 1);
    }
}
