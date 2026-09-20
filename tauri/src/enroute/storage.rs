use super::CatalogEntry;
use anyhow::{Context, Result, ensure};
use std::collections::BTreeMap;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagedFileDetails {
    size: u64,
    /// File modification time, serialized as milliseconds since the Unix epoch.
    #[serde(serialize_with = "super::serialize_timestamp")]
    modified_at: SystemTime,
}

impl ManagedFileDetails {
    pub fn read(path: &Path) -> Result<Self> {
        let metadata = fs::metadata(path)?;
        Ok(Self {
            size: metadata.len(),
            modified_at: metadata.modified()?,
        })
    }
}

pub const DOWNLOAD_PREFIX: &str = ".updraft-download-";
pub const DOWNLOAD_SUFFIX: &str = ".part";

/// Lists installed files by provider and catalog-relative path, without opening them.
///
/// Flat development files and symlinks are excluded. A missing provider directory
/// gives an empty inventory. Other filesystem errors fail the scan.
pub fn installed_files(directory: &Path) -> Result<BTreeMap<String, PathBuf>> {
    Ok(stored_files(directory)?
        .into_iter()
        .filter(|(_, path)| {
            path.extension()
                .is_some_and(|ext| ext == "mbtiles" || ext == "terrain")
        })
        .collect())
}

pub fn persist_enabled(marker: &Path, enabled: bool) -> std::io::Result<()> {
    if enabled {
        match fs::remove_file(marker) {
            Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
            result => result,
        }
    } else {
        match fs::File::create_new(marker) {
            Ok(_) => Ok(()),
            Err(error)
                if error.kind() == ErrorKind::AlreadyExists
                    && fs::symlink_metadata(marker)?.is_file() =>
            {
                Ok(())
            }
            Err(error) => Err(error),
        }
    }
}

pub fn available_updates<T>(
    directory: &Path,
    files: &BTreeMap<String, T>,
    entries: &[CatalogEntry],
    file_kind: &str,
) -> Result<Vec<&'static str>> {
    let mut updates = Vec::new();
    for entry in entries {
        let name = format!("enroute/{}", entry.path);
        if !files.contains_key(&name) {
            continue;
        }
        let modified = fs::metadata(directory.join(&name))
            .and_then(|metadata| metadata.modified())
            .with_context(|| format!("Could not read {file_kind} timestamp for {name}"))?;
        if entry.update_available(modified) {
            updates.push(entry.path);
        }
    }
    Ok(updates)
}

/// Removes abandoned partial files. Call only at startup, before downloads begin.
pub fn remove_partial_downloads(directory: &Path) -> Result<()> {
    for (id, path) in stored_files(directory)? {
        let name = id.rsplit('/').next().expect("Stored files have names");
        if name.starts_with(DOWNLOAD_PREFIX) && name.ends_with(DOWNLOAD_SUFFIX) {
            fs::remove_file(&path).with_context(|| {
                format!("Could not remove partial download: {}", path.display())
            })?;
        }
    }
    Ok(())
}

fn stored_files(directory: &Path) -> Result<BTreeMap<String, PathBuf>> {
    let provider = directory.join("enroute");
    let metadata = match fs::symlink_metadata(&provider) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(BTreeMap::new()),
        Err(error) => return Err(error.into()),
    };
    ensure!(metadata.is_dir(), "Enroute storage must be a directory");
    let mut pending = vec![("enroute".to_owned(), provider)];
    let mut files = BTreeMap::new();
    while let Some((parent, directory)) = pending.pop() {
        for entry in fs::read_dir(&directory)? {
            let entry = entry?;
            let file_type = entry.file_type()?;
            if file_type.is_symlink() {
                continue;
            }
            let name = entry.file_name();
            let name = name.to_str().context("Enroute filename must be UTF-8")?;
            let id = format!("{parent}/{name}");
            let path = entry.path();
            if file_type.is_dir() {
                pending.push((id, path));
            } else if file_type.is_file() && parent != "enroute" {
                files.insert(id, path);
            }
        }
    }
    Ok(files)
}

#[cfg(test)]
mod tests;
