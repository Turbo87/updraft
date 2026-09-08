use super::CatalogEntry;
use super::storage::{DOWNLOAD_PREFIX, DOWNLOAD_SUFFIX};
use anyhow::{Context, Result, ensure};
use std::fs::{self, File, FileTimes};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};
use tempfile::NamedTempFile;

/// Owns a partial basemap beside its destination. Dropping it discards the partial file.
#[derive(Debug)]
pub struct DownloadFile {
    temporary: NamedTempFile,
    destination: PathBuf,
}

impl DownloadFile {
    /// Downloads the complete response into a temporary file without installing it.
    /// Reports cumulative bytes after each successful write. Progress does not imply installation.
    pub async fn fetch(
        directory: &Path,
        entry: &CatalogEntry,
        progress: impl FnMut(u64),
    ) -> Result<Self> {
        let download = Self::new(directory, entry)?;
        let client = super::http_client()
            .connect_timeout(Duration::from_secs(30))
            .read_timeout(Duration::from_secs(30))
            .build()?;
        let base_url = "https://enroute-data.akaflieg-freiburg.de/enroute-GeoJSONv003";
        let url = format!("{base_url}/{}", entry.path);
        let response = client.get(url).send().await?;
        download.receive(response, progress).await
    }

    async fn receive(
        mut self,
        response: reqwest::Response,
        mut progress: impl FnMut(u64),
    ) -> Result<Self> {
        let mut response = response.error_for_status()?;
        ensure!(
            response.status() == reqwest::StatusCode::OK,
            "Expected a complete basemap response"
        );
        let mut written = 0;
        while let Some(chunk) = response.chunk().await? {
            let length = chunk.len() as u64;
            self = tokio::task::spawn_blocking(move || -> Result<Self> {
                self.file_mut().write_all(&chunk)?;
                Ok(self)
            })
            .await??;
            written += length;
            progress(written);
        }
        Ok(self)
    }

    pub fn new(directory: &Path, entry: &CatalogEntry) -> Result<Self> {
        let destination = directory.join("enroute").join(entry.path);
        let parent = destination.parent().context("Basemap path has no parent")?;
        fs::create_dir_all(parent)?;
        let temporary = tempfile::Builder::new()
            .prefix(DOWNLOAD_PREFIX)
            .suffix(DOWNLOAD_SUFFIX)
            .tempfile_in(parent)?;
        Ok(Self {
            temporary,
            destination,
        })
    }

    pub fn destination(&self) -> &Path {
        &self.destination
    }

    pub fn file_mut(&mut self) -> &mut File {
        self.temporary.as_file_mut()
    }

    /// Atomically replaces the installed bytes without validating them or changing activation.
    ///
    /// Callers must finish the transfer and release installed database handles first.
    pub fn install(self) -> Result<()> {
        let times = FileTimes::new().set_modified(SystemTime::now());
        self.temporary.as_file().set_times(times)?;
        self.temporary.as_file().sync_all()?;
        self.temporary
            .persist(&self.destination)
            .map_err(|error| error.error)?;
        Ok(())
    }
}

impl CatalogEntry {
    /// Compares the publication date at midnight UTC with the installed file's timestamp.
    pub fn update_available(&self, modified: SystemTime) -> bool {
        let publication: SystemTime = self.publication_date.midnight().assume_utc().into();
        publication > modified
    }
}

#[cfg(test)]
mod tests;
