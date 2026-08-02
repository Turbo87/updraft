use serde::Deserialize;
use std::path::PathBuf;
use std::sync::Arc;
use updraft_core::{AirspaceDataset, AirspaceImportError, AirspaceLoadError, AirspaceState};

const SOURCE_FILE_NAME: &str = "airspace.txt";
const METADATA_FILE_NAME: &str = "airspace.json";

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AirspaceMetadata {
    source_name: Option<String>,
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use claims::{assert_none, assert_ok, assert_some};
    use tempfile::tempdir;
    use tracing_test::traced_test;
    use updraft_core::{AirspaceLoadError, AirspaceStatus};

    const POLYGON: &[u8] =
        include_bytes!("../../libs/updraft_core/tests/fixtures/airspace/polygon.txt");
    const PARSER_ERROR: &[u8] =
        include_bytes!("../../libs/updraft_core/tests/fixtures/airspace/parser_error.txt");
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
}
