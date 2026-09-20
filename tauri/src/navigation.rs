use crate::driver::DriverHandle;
use std::fs::File;
use std::io::{BufReader, Write};
use std::path::PathBuf;
use tempfile::NamedTempFile;
use updraft_core::{NavigationTarget, SetNavigationTarget};

pub struct NavigationFile {
    path: PathBuf,
    changes: tokio::sync::Mutex<()>,
}

impl NavigationFile {
    pub fn new(directory: PathBuf) -> Self {
        Self {
            path: directory.join("navigation.json"),
            changes: tokio::sync::Mutex::new(()),
        }
    }

    pub fn load(&self) -> std::io::Result<Option<NavigationTarget>> {
        let file = match File::open(&self.path) {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error),
        };
        let target: Option<NavigationTarget> = serde_json::from_reader(BufReader::new(file))?;
        if let Some(target) = &target {
            target.validate().map_err(std::io::Error::other)?;
        }
        Ok(target)
    }

    pub async fn select(
        &self,
        handle: &DriverHandle,
        target: Option<NavigationTarget>,
    ) -> Result<bool, String> {
        let _guard = self.changes.lock().await;
        handle
            .send(SetNavigationTarget(target.clone()))
            .await
            .map_err(|error| error.to_string())?
            .map_err(str::to_owned)?;
        let path = self.path.clone();
        let result =
            tauri::async_runtime::spawn_blocking(move || save_target_file(path, &target)).await;
        match result {
            Ok(Ok(())) => Ok(true),
            error => {
                tracing::warn!(?error, "Could not save navigation target");
                Ok(false)
            }
        }
    }
}

pub fn save_target_file(path: PathBuf, value: &impl serde::Serialize) -> std::io::Result<()> {
    let directory = path.parent().expect("target file has a directory");
    std::fs::create_dir_all(directory)?;
    let mut temporary = NamedTempFile::new_in(directory)?;
    serde_json::to_writer(&mut temporary, value)?;
    writeln!(temporary)?;
    temporary.persist(path).map_err(|error| error.error)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use claims::{assert_err, assert_none, assert_ok};

    #[test]
    fn missing_target_is_empty_and_invalid_saved_coordinates_are_rejected() {
        let directory = assert_ok!(tempfile::tempdir());
        let file = NavigationFile::new(directory.path().to_owned());
        assert_none!(assert_ok!(file.load()));
        assert_ok!(std::fs::write(
            &file.path,
            r#"{"type":"waypoint","name":"Invalid","latitudeDegrees":91,"longitudeDegrees":0,"elevationMeters":0}"#
        ));
        assert_err!(file.load());
    }
}
