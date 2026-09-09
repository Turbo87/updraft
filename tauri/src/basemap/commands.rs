use super::{BasemapSource, Basemaps};
use crate::enroute::commands::DownloadCommands;
use crate::enroute::storage::ManagedFileDetails;
use anyhow::ensure;
use serde::Serialize;
use std::sync::{Arc, Mutex};
use tauri::ipc::Channel;

#[derive(Clone, Serialize)]
pub struct BasemapStatus {
    generation: u64,
    sources: Vec<BasemapSourceStatus>,
}

#[tauri::command]
pub async fn get_basemap_file_details(
    source_name: String,
    state: tauri::State<'_, Arc<Mutex<Basemaps>>>,
) -> Result<ManagedFileDetails, &'static str> {
    let basemaps = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || -> anyhow::Result<_> {
        let basemaps = basemaps.lock().unwrap();
        ensure!(
            basemaps.files.contains_key(&source_name),
            "Unknown basemap file"
        );
        ManagedFileDetails::read(&basemaps.directory.join(&source_name))
    })
    .await
    .unwrap_or_else(|error| Err(error.into()))
    .map_err(|error| {
        tracing::warn!(%error, "Could not read basemap file details");
        "Could not read basemap file details"
    })
}

#[derive(Clone, Serialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
enum BasemapSourceStatus {
    Active { source_name: String },
    Disabled { source_name: String },
    Unavailable { source_name: String },
}

impl Basemaps {
    pub fn publish(&mut self) {
        let status = self.status();
        self.subscribers
            .retain(|_, channel| channel.send(status.clone()).is_ok());
    }

    fn status(&self) -> BasemapStatus {
        let sources = self
            .files
            .iter()
            .map(|(id, source)| {
                let source_name = id.clone();
                match source {
                    BasemapSource::Active(_) => BasemapSourceStatus::Active { source_name },
                    BasemapSource::Disabled => BasemapSourceStatus::Disabled { source_name },
                    BasemapSource::Unavailable(_) => {
                        BasemapSourceStatus::Unavailable { source_name }
                    }
                }
            })
            .collect();
        BasemapStatus {
            generation: self.generation,
            sources,
        }
    }

    fn subscribe(&mut self, channel: Channel<BasemapStatus>) -> tauri::Result<()> {
        channel.send(self.status())?;
        self.subscribers.insert(channel.id(), channel);
        Ok(())
    }
}

#[tauri::command]
pub async fn set_basemap_enabled(
    source_name: String,
    enabled: bool,
    state: tauri::State<'_, Arc<Mutex<Basemaps>>>,
) -> Result<(), &'static str> {
    let basemaps = state.inner().clone();
    let name = source_name.clone();
    tauri::async_runtime::spawn_blocking(move || {
        basemaps
            .lock()
            .expect("Basemap access should not panic")
            .set_enabled(&name, enabled)
    })
    .await
    .unwrap_or_else(|error| Err(error.into()))
    .map_err(|error| {
        tracing::warn!(%error, source_name, "Could not change basemap activation");
        "Could not change basemap activation"
    })
}

#[tauri::command(async)]
pub fn subscribe_basemaps(
    channel: Channel<BasemapStatus>,
    state: tauri::State<'_, Arc<Mutex<Basemaps>>>,
) -> Result<(), &'static str> {
    state
        .lock()
        .expect("Basemap access should not panic")
        .subscribe(channel)
        .map_err(|_| "Could not subscribe to basemap status")
}

#[tauri::command]
pub async fn remove_basemap(
    source_name: String,
    downloads: tauri::State<'_, DownloadCommands>,
    state: tauri::State<'_, Arc<Mutex<Basemaps>>>,
) -> Result<(), &'static str> {
    let basemaps = state.inner().clone();
    let queue = downloads.queue.clone();
    let name = source_name.clone();
    tauri::async_runtime::spawn_blocking(move || {
        // Keep the queue locked through deletion to exclude installation.
        let mut queue = queue.lock().unwrap();
        if let Some(path) = name.strip_prefix("enroute/") {
            queue.cancel(path);
        }
        basemaps
            .lock()
            .expect("Basemap access should not panic")
            .remove(&name)
    })
    .await
    .unwrap_or_else(|error| Err(error.into()))
    .map_err(|error| {
        tracing::warn!(%error, source_name, "Could not remove basemap file");
        "Could not remove basemap file"
    })
}

#[tauri::command(async)]
pub fn unsubscribe_basemaps(channel_id: u32, state: tauri::State<'_, Arc<Mutex<Basemaps>>>) {
    state
        .lock()
        .expect("Basemap access should not panic")
        .subscribers
        .remove(&channel_id);
}

#[cfg(test)]
mod tests {
    use super::*;
    use claims::assert_err;

    #[test]
    fn failed_initial_delivery_does_not_register_a_subscriber() {
        let mut basemaps = Basemaps::default();
        let channel = Channel::new(|_| Err(std::io::Error::other("closed channel").into()));
        assert_err!(basemaps.subscribe(channel));
        assert!(basemaps.subscribers.is_empty());
    }

    #[test]
    #[tracing_test::traced_test]
    fn file_details_read_disabled_files_and_reject_unknown_paths_through_ipc() {
        use claims::assert_ok;
        use serde_json::{Value, json};
        use std::fs::{self, FileTimes, OpenOptions};
        use std::time::{Duration, UNIX_EPOCH};

        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("enroute/Europe/France.mbtiles");
        assert_ok!(fs::create_dir_all(path.parent().unwrap()));
        assert_ok!(fs::write(&path, b"not a database"));
        assert_ok!(fs::write(path.with_extension("mbtiles.disabled"), b""));
        let file = assert_ok!(OpenOptions::new().write(true).open(&path));
        let modified = UNIX_EPOCH + Duration::from_secs(1234);
        assert_ok!(file.set_times(FileTimes::new().set_modified(modified)));
        let basemaps = assert_ok!(Basemaps::load(directory.path()));
        let app = tauri::test::mock_builder()
            .manage(Arc::new(Mutex::new(basemaps)))
            .invoke_handler(tauri::generate_handler![get_basemap_file_details])
            .build(tauri::test::mock_context(tauri::test::noop_assets()))
            .unwrap();
        let window = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
            .build()
            .unwrap();
        let invoke = |name: &str| {
            let request = tauri::webview::InvokeRequest {
                cmd: "get_basemap_file_details".into(),
                callback: tauri::ipc::CallbackFn(0),
                error: tauri::ipc::CallbackFn(1),
                url: "tauri://localhost".parse().unwrap(),
                body: tauri::ipc::InvokeBody::Json(json!({"sourceName": name})),
                headers: Default::default(),
                invoke_key: tauri::test::INVOKE_KEY.into(),
            };
            tauri::test::get_ipc_response(&window, request)
                .map(|r| r.deserialize::<Value>().unwrap())
        };
        let name = "enroute/Europe/France.mbtiles";
        insta::assert_json_snapshot!(assert_ok!(invoke(name)), @r#"
        {
          "modifiedAt": 1234000.0,
          "size": 14
        }
        "#);
        let modified = UNIX_EPOCH - Duration::from_secs(1);
        assert_ok!(file.set_times(FileTimes::new().set_modified(modified)));
        assert_eq!(assert_ok!(invoke(name))["modifiedAt"], json!(-1000.0));
        let error = json!("Could not read basemap file details");
        assert_eq!(assert_err!(invoke("../outside.mbtiles")), error);
        assert_ok!(fs::remove_file(path));
        assert_eq!(assert_err!(invoke(name)), error);
        // Tauri dispatches IPC commands outside the test span.
        assert!(tracing_test::internal::logs_with_scope_contain(
            module_path!().trim_end_matches("::tests"),
            "Could not read basemap file details"
        ));
        assert!(!logs_contain("Could not open offline basemap"));
    }
}
