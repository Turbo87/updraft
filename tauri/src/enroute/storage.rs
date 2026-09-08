use anyhow::{Context, Result, ensure};
use std::collections::BTreeMap;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

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
