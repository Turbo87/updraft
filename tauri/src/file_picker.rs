use std::future::Future;
use std::pin::Pin;
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_fs::{FilePath, FsExt};

/// The complete contents and display name of one selected file.
///
/// The picker reads the complete file into memory.
pub struct PickedFileBytes {
    pub display_name: Option<String>,
    pub bytes: Vec<u8>,
}

/// A failure while selecting or reading one complete file.
#[derive(Debug, thiserror::Error)]
pub enum FileBytesPickerError {
    #[error("file picker failed")]
    Picker {
        #[source]
        source: anyhow::Error,
    },
    #[error("selected file could not be read")]
    Read {
        display_name: Option<String>,
        #[source]
        source: anyhow::Error,
    },
}

/// The result future returned by a file bytes picker.
pub type FileBytesPickerFuture = Pin<
    Box<
        dyn Future<Output = Result<Option<PickedFileBytes>, FileBytesPickerError>> + Send + 'static,
    >,
>;

/// Selects and reads files whose complete contents fit in memory.
pub trait FileBytesPicker: Send + Sync {
    fn pick_file_bytes(&self) -> FileBytesPickerFuture;
}

/// A file bytes picker stored as Tauri managed state.
pub type FileBytesPickerState = Box<dyn FileBytesPicker>;

/// Selects files with Tauri and reads paths and platform content URIs.
pub struct TauriFileBytesPicker<R: tauri::Runtime> {
    app: tauri::AppHandle<R>,
}

impl<R: tauri::Runtime> TauriFileBytesPicker<R> {
    pub fn new(app: tauri::AppHandle<R>) -> Self {
        Self { app }
    }
}

impl<R: tauri::Runtime> FileBytesPicker for TauriFileBytesPicker<R> {
    fn pick_file_bytes(&self) -> FileBytesPickerFuture {
        let app = self.app.clone();
        Box::pin(async move { pick_tauri_file_bytes(app).await })
    }
}

async fn pick_tauri_file_bytes<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
) -> Result<Option<PickedFileBytes>, FileBytesPickerError> {
    let (sender, receiver) = tokio::sync::oneshot::channel();
    app.dialog().file().pick_file(move |selection| {
        let _ = sender.send(selection);
    });
    let Some(locator) = receiver
        .await
        .map_err(|source| FileBytesPickerError::Picker {
            source: source.into(),
        })?
    else {
        return Ok(None);
    };

    let display_name = file_display_name(&locator, |uri| {
        #[cfg(target_os = "android")]
        {
            use tauri_plugin_updraft::UpdraftMobileExt;
            Ok(app.updraft_mobile().document_display_name(uri)?)
        }
        #[cfg(not(target_os = "android"))]
        {
            let _ = uri;
            Ok(None)
        }
    })
    .map_err(|source| FileBytesPickerError::Read {
        display_name: None,
        source,
    })?;
    let bytes = app
        .fs()
        .read(locator)
        .map_err(|source| FileBytesPickerError::Read {
            display_name: display_name.clone(),
            source: source.into(),
        })?;

    Ok(Some(PickedFileBytes {
        display_name,
        bytes,
    }))
}

fn file_display_name(
    locator: &FilePath,
    document_name: impl FnOnce(&str) -> anyhow::Result<Option<String>>,
) -> anyhow::Result<Option<String>> {
    match locator {
        FilePath::Url(uri) if uri.scheme() == "content" => document_name(uri.as_str()),
        _ => Ok(locator
            .as_path()
            .and_then(std::path::Path::file_name)
            .and_then(std::ffi::OsStr::to_str)
            .map(str::to_owned)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use claims::{assert_ok, assert_some_eq};
    use tauri_plugin_fs::FilePath;

    #[test]
    fn path_file_name_becomes_the_display_name() {
        let path = FilePath::Path("/selected/Local airspace.txt".into());

        let name = assert_ok!(file_display_name(&path, |_| panic!(
            "path needs no metadata"
        )));
        assert_some_eq!(name, "Local airspace.txt");
    }

    #[test]
    fn content_uri_uses_document_display_name() {
        let uri = "content://provider/document/123".parse().unwrap();
        let name = file_display_name(&FilePath::Url(uri), |uri| {
            assert_eq!(uri, "content://provider/document/123");
            Ok(Some("Local waypoints.cup".to_owned()))
        });
        assert_some_eq!(assert_ok!(name), "Local waypoints.cup");
    }
}
