use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tempfile::NamedTempFile;
use thiserror::Error;
use updraft_airspace::{AirspaceDataset, AirspaceImportError, AirspaceLoadError, AirspaceState};

const SOURCE_FILE_NAME: &str = "airspace.txt";
const METADATA_FILE_NAME: &str = "airspace.json";

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct AirspaceMetadata {
    source_name: Option<String>,
}

/// One validated dataset after its source and metadata are committed.
pub struct StoredAirspace {
    pub dataset: Arc<AirspaceDataset>,
    pub source_name: Option<String>,
}

/// A validation or storage failure during an airspace mutation.
#[derive(Debug, Error)]
pub enum AirspaceStorageError {
    #[error(transparent)]
    Import(#[from] AirspaceImportError),
    #[error("could not {operation}: {source}")]
    Io {
        operation: &'static str,
        #[source]
        source: std::io::Error,
    },
    #[error("could not encode airspace metadata: {0}")]
    Metadata(#[from] serde_json::Error),
}

impl AirspaceStorageError {
    fn io(operation: &'static str, source: std::io::Error) -> Self {
        Self::Io { operation, source }
    }
}

/// Loads the app-owned OpenAir source and its optional display name.
pub struct AirspaceStorage {
    source_path: PathBuf,
    metadata_path: PathBuf,
}

impl AirspaceStorage {
    /// Resolves the fixed airspace paths under the app data directory.
    pub fn new(data_dir: impl Into<PathBuf>) -> Self {
        let data_dir = data_dir.into();
        Self {
            source_path: data_dir.join(SOURCE_FILE_NAME),
            metadata_path: data_dir.join(METADATA_FILE_NAME),
        }
    }

    /// Validates and commits one complete OpenAir source replacement.
    ///
    /// # Errors
    ///
    /// Returns an error if validation or a required storage operation fails.
    pub fn import_airspace(
        &self,
        bytes: &[u8],
        source_name: Option<String>,
    ) -> Result<StoredAirspace, AirspaceStorageError> {
        let dataset = Arc::new(AirspaceDataset::from_openair(bytes)?);
        let directory = self.storage_directory();

        std::fs::create_dir_all(directory)
            .map_err(|source| AirspaceStorageError::io("create the airspace directory", source))?;

        let previous_metadata = read_optional(&self.metadata_path).map_err(|source| {
            AirspaceStorageError::io("read the previous airspace metadata", source)
        })?;

        let source_file = prepare_file(directory, bytes)
            .map_err(|source| AirspaceStorageError::io("prepare the airspace source", source))?;

        let metadata = serde_json::to_vec(&AirspaceMetadata {
            source_name: source_name.clone(),
        })?;
        let metadata_file = prepare_file(directory, &metadata)
            .map_err(|source| AirspaceStorageError::io("prepare the airspace metadata", source))?;

        persist_file(metadata_file, &self.metadata_path)
            .map_err(|source| AirspaceStorageError::io("replace the airspace metadata", source))?;

        let source_result = persist_file(source_file, &self.source_path)
            .map_err(|source| AirspaceStorageError::io("replace the airspace source", source));
        if let Err(error) = source_result {
            self.restore_metadata(directory, previous_metadata.as_deref())?;
            return Err(error);
        }

        Ok(StoredAirspace {
            dataset,
            source_name,
        })
    }

    /// Commits removal by deleting the authoritative source first.
    ///
    /// Metadata cleanup is best effort after the source is absent.
    ///
    /// # Errors
    ///
    /// Returns an error if the source cannot be removed.
    pub fn remove_airspace(&self) -> Result<(), AirspaceStorageError> {
        remove_optional(&self.source_path)
            .map_err(|source| AirspaceStorageError::io("remove the airspace source", source))?;

        let metadata_result = remove_optional(&self.metadata_path)
            .map_err(|source| AirspaceStorageError::io("remove the airspace metadata", source));
        if let Err(error) = metadata_result {
            tracing::warn!(%error, "Could not remove airspace metadata");
        }

        Ok(())
    }

    /// Loads one complete initial airspace state without changing stored files.
    pub fn load(&self) -> AirspaceState {
        let bytes = match std::fs::read(&self.source_path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return AirspaceState::none_at_startup();
            }
            Err(error) => {
                tracing::warn!(
                    path = %self.source_path.display(),
                    %error,
                    "Could not read stored airspace"
                );
                return AirspaceState::unavailable_at_startup(
                    self.load_source_name(),
                    AirspaceLoadError::ReadFailed,
                );
            }
        };
        let source_name = self.load_source_name();

        match AirspaceDataset::from_openair(&bytes) {
            Ok(dataset) => AirspaceState::active_at_startup(Arc::new(dataset), source_name),
            Err(error @ AirspaceImportError::Parse { .. }) => {
                tracing::warn!(
                    path = %self.source_path.display(),
                    %error,
                    "Could not parse stored airspace"
                );
                AirspaceState::unavailable_at_startup(source_name, AirspaceLoadError::ParseFailed)
            }
            Err(error @ AirspaceImportError::Geometry { .. }) => {
                tracing::warn!(
                    path = %self.source_path.display(),
                    %error,
                    "Could not normalize stored airspace"
                );
                AirspaceState::unavailable_at_startup(
                    source_name,
                    AirspaceLoadError::GeometryFailed,
                )
            }
        }
    }

    fn load_source_name(&self) -> Option<String> {
        let bytes = match std::fs::read(&self.metadata_path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return None,
            Err(error) => {
                tracing::warn!(
                    path = %self.metadata_path.display(),
                    %error,
                    "Could not load airspace metadata"
                );
                return None;
            }
        };

        match serde_json::from_slice::<AirspaceMetadata>(&bytes) {
            Ok(metadata) => metadata.source_name,
            Err(error) => {
                tracing::warn!(
                    path = %self.metadata_path.display(),
                    %error,
                    "Could not load airspace metadata"
                );
                None
            }
        }
    }

    fn storage_directory(&self) -> &Path {
        self.source_path
            .parent()
            .expect("the fixed source file must have a parent directory")
    }

    fn restore_metadata(
        &self,
        directory: &Path,
        previous: Option<&[u8]>,
    ) -> Result<(), AirspaceStorageError> {
        match previous {
            Some(bytes) => {
                let file = prepare_file(directory, bytes).map_err(|source| {
                    AirspaceStorageError::io("prepare the previous airspace metadata", source)
                })?;
                persist_file(file, &self.metadata_path).map_err(|source| {
                    AirspaceStorageError::io("restore the previous airspace metadata", source)
                })
            }
            None => remove_optional(&self.metadata_path).map_err(|source| {
                AirspaceStorageError::io("remove the uncommitted airspace metadata", source)
            }),
        }
    }
}

fn prepare_file(directory: &Path, bytes: &[u8]) -> std::io::Result<NamedTempFile> {
    let mut file = NamedTempFile::new_in(directory)?;
    file.write_all(bytes)?;
    file.flush()?;
    Ok(file)
}

fn persist_file(file: NamedTempFile, path: &Path) -> std::io::Result<()> {
    file.persist(path).map(|_| ()).map_err(|error| error.error)
}

fn read_optional(path: &Path) -> std::io::Result<Option<Vec<u8>>> {
    match std::fs::read(path) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

fn remove_optional(path: &Path) -> std::io::Result<()> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use claims::{assert_none, assert_ok, assert_some};
    use tempfile::tempdir;
    use tracing_test::traced_test;
    use updraft_airspace::{AirspaceLoadError, AirspaceStatus};

    const POLYGON: &[u8] =
        include_bytes!("../../libs/updraft_airspace/tests/fixtures/airspace/polygon.txt");
    const PARSER_ERROR: &[u8] =
        include_bytes!("../../libs/updraft_airspace/tests/fixtures/airspace/parser_error.txt");
    const CIRCLE: &[u8] =
        include_bytes!("../../libs/updraft_airspace/tests/fixtures/airspace/circle.txt");
    const GEOMETRY_ERROR: &[u8] = b"AC D\nAL GND\nAH FL100\nDP 50:00:00 N 010:00:00 E\nDP 50:00:00 N 010:01:00 E\nDP 50:00:00 N 010:00:00 E\n";

    #[test]
    fn missing_source_loads_none() {
        let directory = assert_ok!(tempdir());

        let state = AirspaceStorage::new(directory.path()).load();

        assert_eq!(state.status(), AirspaceStatus::None);
        assert_none!(state.snapshot());
    }

    #[test]
    fn valid_source_with_metadata_loads_active() {
        let directory = assert_ok!(tempdir());
        assert_ok!(std::fs::write(
            directory.path().join("airspace.txt"),
            POLYGON
        ));
        assert_ok!(std::fs::write(
            directory.path().join("airspace.json"),
            r#"{"sourceName":"Local airspace.txt"}"#,
        ));

        let state = AirspaceStorage::new(directory.path()).load();

        assert_eq!(
            state.status(),
            AirspaceStatus::Active {
                source_name: Some("Local airspace.txt".into()),
                airspace_count: 1,
                generation: 0,
            }
        );
        assert_eq!(assert_some!(state.snapshot()).airspaces().len(), 1);
    }

    #[test]
    fn valid_source_without_metadata_loads_active_without_a_name() {
        let directory = assert_ok!(tempdir());
        assert_ok!(std::fs::write(
            directory.path().join("airspace.txt"),
            POLYGON
        ));

        let state = AirspaceStorage::new(directory.path()).load();

        assert_eq!(
            state.status(),
            AirspaceStatus::Active {
                source_name: None,
                airspace_count: 1,
                generation: 0,
            }
        );
    }

    #[test]
    #[traced_test]
    fn invalid_metadata_warns_and_loads_active_without_a_name() {
        let directory = assert_ok!(tempdir());
        assert_ok!(std::fs::write(
            directory.path().join("airspace.txt"),
            POLYGON
        ));
        assert_ok!(std::fs::write(
            directory.path().join("airspace.json"),
            b"not JSON",
        ));

        let state = AirspaceStorage::new(directory.path()).load();

        assert_eq!(
            state.status(),
            AirspaceStatus::Active {
                source_name: None,
                airspace_count: 1,
                generation: 0,
            }
        );
        assert!(logs_contain("Could not load airspace metadata"));
        assert!(directory.path().join("airspace.json").exists());
    }

    #[test]
    #[traced_test]
    fn unreadable_source_loads_unavailable_and_preserves_files() {
        let directory = assert_ok!(tempdir());
        assert_ok!(std::fs::create_dir(directory.path().join("airspace.txt")));
        assert_ok!(std::fs::write(
            directory.path().join("airspace.json"),
            r#"{"sourceName":"Unreadable airspace.txt"}"#,
        ));

        let state = AirspaceStorage::new(directory.path()).load();

        assert_eq!(
            state.status(),
            AirspaceStatus::Unavailable {
                source_name: Some("Unreadable airspace.txt".into()),
                error: AirspaceLoadError::ReadFailed,
            }
        );
        assert!(logs_contain("Could not read stored airspace"));
        assert!(directory.path().join("airspace.txt").exists());
        assert!(directory.path().join("airspace.json").exists());
    }

    #[test]
    #[traced_test]
    fn parser_failure_loads_unavailable_and_preserves_the_source() {
        let directory = assert_ok!(tempdir());
        assert_ok!(std::fs::write(
            directory.path().join("airspace.txt"),
            PARSER_ERROR,
        ));

        let state = AirspaceStorage::new(directory.path()).load();

        assert_eq!(
            state.status(),
            AirspaceStatus::Unavailable {
                source_name: None,
                error: AirspaceLoadError::ParseFailed,
            }
        );
        assert!(logs_contain("Could not parse stored airspace"));
        assert_eq!(
            assert_ok!(std::fs::read(directory.path().join("airspace.txt"))),
            PARSER_ERROR
        );
    }

    #[test]
    #[traced_test]
    fn geometry_failure_loads_unavailable_and_preserves_the_source() {
        let directory = assert_ok!(tempdir());
        assert_ok!(std::fs::write(
            directory.path().join("airspace.txt"),
            GEOMETRY_ERROR,
        ));

        let state = AirspaceStorage::new(directory.path()).load();

        assert_eq!(
            state.status(),
            AirspaceStatus::Unavailable {
                source_name: None,
                error: AirspaceLoadError::GeometryFailed,
            }
        );
        assert!(logs_contain("Could not normalize stored airspace"));
        assert_eq!(
            assert_ok!(std::fs::read(directory.path().join("airspace.txt"))),
            GEOMETRY_ERROR
        );
    }

    #[test]
    fn leftover_metadata_without_a_source_loads_none() {
        let directory = assert_ok!(tempdir());
        assert_ok!(std::fs::write(
            directory.path().join("airspace.json"),
            b"not JSON",
        ));

        let state = AirspaceStorage::new(directory.path()).load();

        assert_eq!(state.status(), AirspaceStatus::None);
        assert!(directory.path().join("airspace.json").exists());
    }

    fn write_stored_airspace(directory: &std::path::Path, source: &[u8], source_name: &str) {
        assert_ok!(std::fs::write(directory.join("airspace.txt"), source));
        assert_ok!(std::fs::write(
            directory.join("airspace.json"),
            format!(r#"{{"sourceName":"{source_name}"}}"#),
        ));
    }

    fn assert_stored_airspace(directory: &std::path::Path, source: &[u8], source_name: &str) {
        assert_eq!(
            assert_ok!(std::fs::read(directory.join("airspace.txt"))),
            source
        );
        let metadata: serde_json::Value = assert_ok!(serde_json::from_slice(&assert_ok!(
            std::fs::read(directory.join("airspace.json"))
        )));
        assert_eq!(metadata, serde_json::json!({ "sourceName": source_name }));
    }

    #[test]
    fn first_import_commits_exact_source_and_metadata() {
        let directory = assert_ok!(tempdir());
        let storage = AirspaceStorage::new(directory.path());

        let stored =
            assert_ok!(storage.import_airspace(POLYGON, Some("Local airspace.txt".into())));

        assert_eq!(stored.dataset.airspaces().len(), 1);
        assert_eq!(stored.source_name.as_deref(), Some("Local airspace.txt"));
        assert_stored_airspace(directory.path(), POLYGON, "Local airspace.txt");
    }

    #[test]
    fn replacement_commits_new_source_and_metadata() {
        let directory = assert_ok!(tempdir());
        write_stored_airspace(directory.path(), POLYGON, "Old airspace.txt");
        let storage = AirspaceStorage::new(directory.path());

        let stored =
            assert_ok!(storage.import_airspace(CIRCLE, Some("Replacement airspace.txt".into())));

        assert_eq!(stored.dataset.airspaces().len(), 1);
        assert_stored_airspace(directory.path(), CIRCLE, "Replacement airspace.txt");
    }

    #[test]
    fn validation_failure_keeps_previous_files() {
        let directory = assert_ok!(tempdir());
        write_stored_airspace(directory.path(), POLYGON, "Old airspace.txt");
        let storage = AirspaceStorage::new(directory.path());

        assert!(
            storage
                .import_airspace(PARSER_ERROR, Some("Broken airspace.txt".into()))
                .is_err()
        );

        assert_stored_airspace(directory.path(), POLYGON, "Old airspace.txt");
    }

    #[test]
    fn metadata_read_failure_keeps_the_previous_source() {
        let directory = assert_ok!(tempdir());
        assert_ok!(std::fs::write(
            directory.path().join("airspace.txt"),
            POLYGON,
        ));
        assert_ok!(std::fs::create_dir(directory.path().join("airspace.json")));
        let storage = AirspaceStorage::new(directory.path());

        assert!(
            storage
                .import_airspace(CIRCLE, Some("Replacement airspace.txt".into()))
                .is_err()
        );

        assert_eq!(
            assert_ok!(std::fs::read(directory.path().join("airspace.txt"))),
            POLYGON
        );
        assert!(directory.path().join("airspace.json").is_dir());
    }

    #[test]
    fn source_commit_failure_restores_previous_metadata() {
        let directory = assert_ok!(tempdir());
        let metadata = br#"{ "sourceName": "Old airspace.txt" }"#;
        assert_ok!(std::fs::create_dir(directory.path().join("airspace.txt")));
        assert_ok!(std::fs::write(
            directory.path().join("airspace.json"),
            metadata,
        ));
        let storage = AirspaceStorage::new(directory.path());

        assert!(
            storage
                .import_airspace(CIRCLE, Some("Replacement airspace.txt".into()))
                .is_err()
        );

        assert!(directory.path().join("airspace.txt").is_dir());
        assert_eq!(
            assert_ok!(std::fs::read(directory.path().join("airspace.json"))),
            metadata
        );
    }

    #[test]
    fn removal_deletes_source_and_metadata() {
        let directory = assert_ok!(tempdir());
        write_stored_airspace(directory.path(), POLYGON, "Local airspace.txt");
        let storage = AirspaceStorage::new(directory.path());

        assert_ok!(storage.remove_airspace());

        assert!(!directory.path().join("airspace.txt").exists());
        assert!(!directory.path().join("airspace.json").exists());
    }

    #[test]
    fn removal_failure_preserves_source_and_metadata() {
        let directory = assert_ok!(tempdir());
        let metadata = br#"{ "sourceName": "Local airspace.txt" }"#;
        assert_ok!(std::fs::create_dir(directory.path().join("airspace.txt")));
        assert_ok!(std::fs::write(
            directory.path().join("airspace.json"),
            metadata,
        ));
        let storage = AirspaceStorage::new(directory.path());

        assert!(storage.remove_airspace().is_err());

        assert!(directory.path().join("airspace.txt").is_dir());
        assert_eq!(
            assert_ok!(std::fs::read(directory.path().join("airspace.json"))),
            metadata
        );
    }

    #[test]
    #[traced_test]
    fn metadata_removal_failure_does_not_fail_committed_removal() {
        let directory = assert_ok!(tempdir());
        assert_ok!(std::fs::write(
            directory.path().join("airspace.txt"),
            POLYGON,
        ));
        assert_ok!(std::fs::create_dir(directory.path().join("airspace.json")));
        let storage = AirspaceStorage::new(directory.path());

        assert_ok!(storage.remove_airspace());

        assert!(!directory.path().join("airspace.txt").exists());
        assert!(directory.path().join("airspace.json").exists());
        assert!(logs_contain("Could not remove airspace metadata"));
    }
}
