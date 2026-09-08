use std::{
    io::{self, Write},
    path::{Path, PathBuf},
};
use tempfile::NamedTempFile;

/// Original source bytes stored under encoded names.
#[derive(Clone, Debug)]
pub struct SourceFiles {
    directory: PathBuf,
    extension: &'static str,
}

impl SourceFiles {
    pub fn new(directory: PathBuf, extension: &'static str) -> Self {
        Self {
            directory,
            extension,
        }
    }

    pub fn path(&self, name: &str) -> PathBuf {
        let encoded = hex::encode(name);
        let mut path = self.directory.clone();
        for chunk in encoded.as_bytes().chunks(250) {
            path.push(std::str::from_utf8(chunk).expect("Hexadecimal names are ASCII"));
        }
        path.set_extension(self.extension);
        path
    }

    /// Lists decoded source names and paths without parsing their contents.
    ///
    /// The caller decides whether an unreadable subtree can be skipped.
    pub fn entries(
        &self,
        mut directory_error: impl FnMut(&Path, io::Error) -> io::Result<()>,
    ) -> io::Result<Vec<(String, PathBuf)>> {
        let entries = match std::fs::read_dir(&self.directory) {
            Ok(entries) => entries,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(error) => return Err(error),
        };
        let mut sources = Vec::new();
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
                        match std::fs::read_dir(&path) {
                            Ok(entries) => {
                                directories.push((entries, format!("{prefix}{component}")))
                            }
                            Err(error) => directory_error(&path, error)?,
                        }
                    }
                    continue;
                }
                let extension = path.extension().and_then(|extension| extension.to_str());
                if extension != Some(self.extension) {
                    continue;
                }
                let stem = path
                    .file_stem()
                    .and_then(|stem| stem.to_str())
                    .unwrap_or_default();
                let name = hex::decode(format!("{prefix}{stem}"))
                    .ok()
                    .and_then(|name| String::from_utf8(name).ok())
                    .ok_or_else(|| {
                        io::Error::new(io::ErrorKind::InvalidData, "Invalid stored source filename")
                    })?;
                sources.push((name, path));
            }
        }
        Ok(sources)
    }

    pub fn replace(&self, name: &str, bytes: &[u8]) -> io::Result<()> {
        let path = self.path(name);
        std::fs::create_dir_all(path.parent().expect("Source files have a parent directory"))?;
        let mut file = NamedTempFile::new_in(&self.directory)?;
        file.write_all(bytes)?;
        file.as_file().sync_all()?;
        file.persist(&path).map_err(|error| error.error)?;
        Ok(())
    }

    pub fn remove(&self, name: &str) -> io::Result<()> {
        std::fs::remove_file(self.path(name))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use claims::{assert_err, assert_ok};
    use std::fs;

    #[test]
    fn failed_replacement_cleans_temporary_files() {
        let directory = assert_ok!(tempfile::tempdir());
        let files = SourceFiles::new(directory.path().into(), "txt");
        let path = files.path("blocked.txt");
        assert_ok!(fs::create_dir(&path));
        let original = path.join("original");
        assert_ok!(fs::write(&original, b"original"));

        assert_err!(files.replace("blocked.txt", b"replacement"));

        assert_eq!(assert_ok!(fs::read(&original)), b"original");
        assert_eq!(assert_ok!(fs::read_dir(directory.path())).count(), 1);
    }

    #[cfg(unix)]
    #[test]
    fn write_failure_preserves_installed_bytes() {
        use std::os::unix::fs::PermissionsExt;

        let directory = assert_ok!(tempfile::tempdir());
        let path = directory.path();
        let files = SourceFiles::new(path.into(), "txt");
        assert_ok!(files.replace("local.txt", b"original"));
        assert_ok!(fs::set_permissions(path, fs::Permissions::from_mode(0o500)));

        let result = files.replace("local.txt", b"replacement");

        assert_ok!(fs::set_permissions(path, fs::Permissions::from_mode(0o700)));
        assert_eq!(assert_err!(result).kind(), io::ErrorKind::PermissionDenied);
        assert_eq!(assert_ok!(fs::read(files.path("local.txt"))), b"original");
        assert_eq!(assert_ok!(fs::read_dir(path)).count(), 1);
    }
}
