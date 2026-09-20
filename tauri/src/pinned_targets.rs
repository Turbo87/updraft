use crate::driver::DriverHandle;
use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;
use updraft_core::{Input, NavigationTarget, PinTarget, SavedPinnedTarget, UnpinTarget};

pub struct PinnedTargetsFile {
    path: PathBuf,
    changes: tokio::sync::Mutex<()>,
}

impl PinnedTargetsFile {
    pub fn new(directory: PathBuf) -> Self {
        Self {
            path: directory.join("pinned-targets.json"),
            changes: Default::default(),
        }
    }

    pub fn load(&self) -> std::io::Result<Vec<SavedPinnedTarget>> {
        match File::open(&self.path) {
            Ok(file) => Ok(serde_json::from_reader(BufReader::new(file))?),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
            Err(error) => Err(error),
        }
    }

    async fn change<I>(&self, handle: &DriverHandle, input: I) -> Result<bool, String>
    where
        I: Input<Response = Result<Vec<SavedPinnedTarget>, &'static str>> + Send + 'static,
    {
        let _guard = self.changes.lock().await;
        let pins = handle
            .send(input)
            .await
            .map_err(|error| error.to_string())?
            .map_err(str::to_owned)?;
        let path = self.path.clone();
        let result = tauri::async_runtime::spawn_blocking(move || {
            crate::navigation::save_target_file(path, &pins)
        })
        .await;
        match result {
            Ok(Ok(())) => Ok(true),
            error => {
                tracing::warn!(?error, "Could not save pinned targets");
                Ok(false)
            }
        }
    }
}

#[tauri::command]
pub async fn pin_target(
    target: NavigationTarget,
    handle: tauri::State<'_, DriverHandle>,
    file: tauri::State<'_, PinnedTargetsFile>,
) -> Result<bool, String> {
    file.change(&handle, PinTarget(target)).await
}

#[tauri::command]
pub async fn unpin_target(
    id: u32,
    handle: tauri::State<'_, DriverHandle>,
    file: tauri::State<'_, PinnedTargetsFile>,
) -> Result<bool, String> {
    file.change(&handle, UnpinTarget(id)).await
}

#[cfg(test)]
mod tests;
