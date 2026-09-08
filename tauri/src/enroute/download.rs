use super::BasemapEntry;
use anyhow::{Context, Result};
use std::fs::{self, File, FileTimes};
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use tempfile::NamedTempFile;

/// Owns a partial basemap beside its destination. Dropping it discards the partial file.
#[derive(Debug)]
pub struct BasemapDownload {
    temporary: NamedTempFile,
    destination: PathBuf,
}

impl BasemapDownload {
    pub fn new(directory: &Path, entry: &BasemapEntry) -> Result<Self> {
        let destination = directory.join("enroute").join(entry.path);
        let parent = destination.parent().context("Basemap path has no parent")?;
        fs::create_dir_all(parent)?;
        let temporary = tempfile::Builder::new()
            .prefix(".updraft-download-")
            .suffix(".part")
            .tempfile_in(parent)?;
        Ok(Self {
            temporary,
            destination,
        })
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

impl BasemapEntry {
    /// Compares the publication date at midnight UTC with the installed file's timestamp.
    pub fn update_available(&self, modified: SystemTime) -> bool {
        let publication: SystemTime = self.publication_date.midnight().assume_utc().into();
        publication > modified
    }
}

#[cfg(test)]
mod tests;
