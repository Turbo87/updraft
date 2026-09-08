use super::*;
use crate::{
    driver::Driver,
    file_picker::{FileBytesPicker, FileBytesPickerFuture, PickedFileBytes},
};
use claims::{assert_err, assert_ok};
use serde_json::{Value, json};
use std::{sync::Mutex, time::Duration};
use tauri::{Manager, test::MockRuntime};
use updraft_core::{AirspaceState, SettingsSnapshot};

const CUP: &[u8] = b"name,code,country,lat,lon,elev,style\nField,,,5000.000N,00600.000E,100m,2\n";

struct Picker(Mutex<Option<PickedFileBytes>>);
impl FileBytesPicker for Picker {
    fn pick_file_bytes(&self) -> FileBytesPickerFuture {
        let selected = self.0.lock().unwrap().take();
        Box::pin(async move { Ok(selected) })
    }
}

fn driver() -> DriverHandle {
    Driver::spawn(
        SettingsSnapshot::default(),
        AirspaceState::none_at_startup(),
        Box::new(|_, _, _| Box::new(|| {})),
        Box::new(|_| {}),
        Duration::from_millis(100),
    )
}

fn app(
    storage: WaypointStorage,
    selected: Option<&[u8]>,
    handle: DriverHandle,
) -> tauri::App<MockRuntime> {
    let picker: FileBytesPickerState =
        Box::new(Picker(Mutex::new(selected.map(|bytes| PickedFileBytes {
            display_name: Some("local.cup".into()),
            bytes: bytes.to_vec(),
        }))));
    tauri::test::mock_builder()
        .manage(WaypointCommandState::new(storage))
        .manage(handle)
        .manage(picker)
        .invoke_handler(tauri::generate_handler![import_waypoints, remove_waypoints])
        .build(tauri::test::mock_context(tauri::test::noop_assets()))
        .unwrap()
}

fn invoke(app: &tauri::App<MockRuntime>, command: &str, body: Value) -> Result<Value, Value> {
    let window = tauri::WebviewWindowBuilder::new(app, "main", Default::default())
        .build()
        .unwrap();
    let request = tauri::webview::InvokeRequest {
        cmd: command.into(),
        callback: tauri::ipc::CallbackFn(0),
        error: tauri::ipc::CallbackFn(1),
        url: "tauri://localhost".parse().unwrap(),
        body: tauri::ipc::InvokeBody::Json(body),
        headers: Default::default(),
        invoke_key: tauri::test::INVOKE_KEY.into(),
    };
    tauri::test::get_ipc_response(&window, request).map(|response| response.deserialize().unwrap())
}

#[tokio::test(flavor = "multi_thread")]
async fn import_persists_and_activates_valid_rows_with_diagnostics() {
    let dir = assert_ok!(tempfile::tempdir());
    let storage = WaypointStorage::new(dir.path().to_owned());
    let source = format!(
        "{}Bad,,,bad,00600.000E,0m,1\n",
        String::from_utf8_lossy(CUP)
    );
    let app = app(storage.clone(), Some(source.as_bytes()), driver());
    let response = assert_ok!(invoke(&app, "import_waypoints", json!({})));
    assert_eq!(
        response,
        json!({"type": "imported", "sourceName": "local.cup"})
    );
    let catalog = assert_ok!(app.state::<DriverHandle>().send(GetWaypointCatalog).await);
    let dataset = assert_ok!(catalog.sources["local.cup"].as_ref());
    assert_eq!(dataset.warnings().len(), 1);
    assert_eq!(assert_ok!(storage.load()), *catalog);
}

#[tokio::test(flavor = "multi_thread")]
async fn cancellation_does_not_create_a_source() {
    let dir = assert_ok!(tempfile::tempdir());
    let storage = WaypointStorage::new(dir.path().to_owned());
    let app = app(storage.clone(), None, driver());
    assert_eq!(
        assert_ok!(invoke(&app, "import_waypoints", json!({}))),
        json!({"type":"cancelled"})
    );
    assert!(assert_ok!(storage.load()).sources.is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn invalid_replacement_preserves_the_stored_source() {
    let dir = assert_ok!(tempfile::tempdir());
    let storage = WaypointStorage::new(dir.path().to_owned());
    assert_ok!(storage.import("local.cup", CUP));
    let original = assert_ok!(storage.load());
    let app = app(storage.clone(), Some(b"invalid"), driver());
    assert_eq!(
        assert_err!(invoke(&app, "import_waypoints", json!({}))),
        json!("parseFailed")
    );
    assert_eq!(assert_ok!(storage.load()), original);
}

#[tokio::test(flavor = "multi_thread")]
async fn stopped_driver_does_not_change_storage() {
    let dir = assert_ok!(tempfile::tempdir());
    let storage = WaypointStorage::new(dir.path().to_owned());
    let app = app(storage.clone(), Some(CUP), DriverHandle::stopped());
    assert_eq!(
        assert_err!(invoke(&app, "import_waypoints", json!({}))),
        json!("driverStopped")
    );
    assert!(assert_ok!(storage.load()).sources.is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn removal_clears_only_the_selected_file() {
    let dir = assert_ok!(tempfile::tempdir());
    let storage = WaypointStorage::new(dir.path().to_owned());
    assert_ok!(storage.import("a.cup", CUP));
    assert_ok!(storage.import("b.cup", CUP));
    let catalog = Arc::new(assert_ok!(storage.load()));
    let app = app(storage.clone(), None, driver());
    assert_ok!(
        app.state::<DriverHandle>()
            .send(ReplaceWaypointCatalog(catalog))
            .await
    );
    assert_ok!(invoke(
        &app,
        "remove_waypoints",
        json!({"sourceName":"a.cup"})
    ));
    let catalog = assert_ok!(app.state::<DriverHandle>().send(GetWaypointCatalog).await);
    assert_eq!(catalog.sources.keys().collect::<Vec<_>>(), ["b.cup"]);
    assert_eq!(assert_ok!(storage.load()), *catalog);
}

#[tokio::test(flavor = "multi_thread")]
#[tracing_test::traced_test]
async fn failed_removal_keeps_the_active_catalog() {
    let dir = assert_ok!(tempfile::tempdir());
    let storage = WaypointStorage::new(dir.path().to_owned());
    assert_ok!(storage.import("a.cup", CUP));
    let catalog = Arc::new(assert_ok!(storage.load()));
    let app = app(storage.clone(), None, driver());
    assert_ok!(
        app.state::<DriverHandle>()
            .send(ReplaceWaypointCatalog(catalog.clone()))
            .await
    );
    assert_ok!(std::fs::remove_dir_all(dir.path().join("waypoints")));
    assert_eq!(
        assert_err!(invoke(
            &app,
            "remove_waypoints",
            json!({"sourceName":"a.cup"})
        )),
        json!("storageFailed")
    );
    assert_eq!(
        assert_ok!(app.state::<DriverHandle>().send(GetWaypointCatalog).await),
        catalog
    );
    // Tauri runs IPC futures outside the test span.
    let logs = tracing_test::internal::global_buf().lock().unwrap().clone();
    assert!(String::from_utf8_lossy(&logs).contains("Could not remove waypoint source"));
}

#[tokio::test(flavor = "multi_thread")]
async fn failed_waypoint_publication_keeps_stored_changes() {
    use crate::driver::tests::stop_after_next_input;
    use updraft_core::{Core, Timestamp};

    for (command, installed) in [
        ("import_waypoints", false),
        ("import_waypoints", true),
        ("remove_waypoints", true),
    ] {
        let dir = assert_ok!(tempfile::tempdir());
        let storage = WaypointStorage::new(dir.path().to_owned());
        assert_ok!(storage.import("other.cup", CUP));
        if installed {
            assert_ok!(storage.import("local.cup", CUP));
        }
        let original = assert_ok!(storage.load());
        let mut core = Core::new(SettingsSnapshot::default());
        core.apply(
            ReplaceWaypointCatalog(Arc::new(original.clone())),
            Timestamp::from_millis(0),
        );
        let replacement = String::from_utf8_lossy(CUP).replace("Field", "Replacement");
        let app = app(
            storage.clone(),
            Some(replacement.as_bytes()),
            stop_after_next_input(core),
        );
        let error = assert_err!(invoke(&app, command, json!({"sourceName": "local.cup"})));
        assert_eq!(error, json!("driverStopped"));
        let catalog = assert_ok!(storage.load());
        assert_eq!(catalog.sources["other.cup"], original.sources["other.cup"]);
        if command == "remove_waypoints" {
            assert_eq!(catalog.sources.keys().collect::<Vec<_>>(), ["other.cup"]);
        } else {
            assert_eq!(catalog.sources.len(), 2);
            let dataset = assert_ok!(catalog.sources["local.cup"].as_ref());
            assert_eq!(dataset.waypoints()[0].name, "Replacement");
        }
    }
}
