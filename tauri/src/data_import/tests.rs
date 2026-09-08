use super::*;
use crate::{
    airspace_storage::AirspaceStorage,
    driver::Driver,
    file_picker::{FileBytesPicker, FileBytesPickerError, FileBytesPickerFuture, PickedFileBytes},
    ipc::AirspaceCommandState,
    waypoints::{commands::WaypointCommandState, storage::WaypointStorage},
};
use claims::{assert_none, assert_ok, assert_some};
use serde_json::{Value, json};
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};
use tauri::{Manager, test::MockRuntime};
use updraft_core::{AirspaceState, GetWaypointCatalog, SettingsSnapshot};

#[derive(Clone)]
struct Picker(Arc<Mutex<Result<Option<PickedFileBytes>, FileBytesPickerError>>>);
impl FileBytesPicker for Picker {
    fn pick_file_bytes(&self) -> FileBytesPickerFuture {
        let selected = std::mem::replace(&mut *self.0.lock().unwrap(), Ok(None));
        Box::pin(async move { selected })
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

fn app(directory: &std::path::Path, name: &str, handle: DriverHandle) -> tauri::App<MockRuntime> {
    let picker = Picker(Arc::new(Mutex::new(Ok(Some(PickedFileBytes {
        display_name: Some(name.into()),
        bytes: b"invalid file".to_vec(),
    })))));
    tauri::test::mock_builder()
        .manage(DataImportState::default())
        .manage(AirspaceCommandState::new(AirspaceStorage::new(directory)))
        .manage(WaypointCommandState::new(WaypointStorage::new(
            directory.into(),
        )))
        .manage(handle)
        .manage(picker.clone())
        .manage(Box::new(picker) as FileBytesPickerState)
        .invoke_handler(tauri::generate_handler![
            select_data_file,
            import_data_file,
            discard_data_file
        ])
        .build(tauri::test::mock_context(tauri::test::noop_assets()))
        .unwrap()
}

fn invoke(app: &tauri::App<MockRuntime>, command: &str, body: Value) -> Result<Value, Value> {
    let window = app.get_webview_window("main").unwrap_or_else(|| {
        tauri::WebviewWindowBuilder::new(app, "main", Default::default())
            .build()
            .unwrap()
    });
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
async fn selection_waits_for_import_and_discard_releases_it() {
    let dir = assert_ok!(tempfile::tempdir());
    let app = app(dir.path(), "local.CUP", driver());
    let selected = assert_ok!(invoke(&app, "select_data_file", json!({})));
    assert_eq!(selected["sourceName"], "local.CUP");
    assert_eq!(selected["dataType"], "waypoints");
    let catalog = assert_ok!(app.state::<DriverHandle>().send(GetWaypointCatalog).await);
    assert!(catalog.sources.is_empty());
    assert_eq!(assert_ok!(std::fs::read_dir(dir.path())).count(), 0);
    let body = json!({"selectionId": selected["selectionId"]});
    assert_eq!(
        assert_ok!(invoke(&app, "discard_data_file", body.clone())),
        Value::Null
    );
    assert_eq!(
        invoke(&app, "import_data_file", body),
        Err(json!({"kind":"noSelection"}))
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn stale_requests_cannot_import_or_discard_a_new_selection() {
    let dir = assert_ok!(tempfile::tempdir());
    let app = app(dir.path(), "old.cup", driver());
    let old = assert_ok!(invoke(&app, "select_data_file", json!({})));
    *app.state::<Picker>().0.lock().unwrap() = Ok(Some(PickedFileBytes {
        display_name: Some("new.txt".into()),
        bytes: vec![],
    }));
    let new = assert_ok!(invoke(&app, "select_data_file", json!({})));
    assert_ne!(old["selectionId"], new["selectionId"]);
    let old = json!({"selectionId":old["selectionId"]});
    assert_ok!(invoke(&app, "discard_data_file", old.clone()));
    assert_eq!(
        invoke(&app, "import_data_file", old),
        Err(json!({"kind":"noSelection"}))
    );
    let state = app.state::<DataImportState>();
    assert_some!(&*state.selection.lock().await);
    assert_eq!(
        assert_ok!(invoke(&app, "select_data_file", json!({}))),
        Value::Null
    );
    let new = json!({"selectionId":new["selectionId"]});
    assert_eq!(
        invoke(&app, "import_data_file", new),
        Err(json!({"kind":"noSelection"}))
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn import_routes_the_selected_bytes_and_consumes_the_selection() {
    use updraft_core::GetAirspaceSnapshot;
    let cases = [
        (
            "local.TXT",
            "airspace",
            include_bytes!("../../../testdata/airspace/circle.txt").as_slice(),
        ),
        (
            "local.cup",
            "waypoints",
            b"name,code,country,lat,lon,elev,style\nField,,,5000.000N,00600.000E,100m,2\n"
                .as_slice(),
        ),
    ];
    for (name, data_type, bytes) in cases {
        let dir = assert_ok!(tempfile::tempdir());
        let app = app(dir.path(), name, driver());
        *app.state::<Picker>().0.lock().unwrap() = Ok(Some(PickedFileBytes {
            display_name: Some(name.into()),
            bytes: bytes.to_vec(),
        }));
        let selected = assert_ok!(invoke(&app, "select_data_file", json!({})));
        assert_eq!(selected["dataType"], data_type);
        let body = json!({"selectionId":selected["selectionId"]});
        assert_eq!(
            assert_ok!(invoke(&app, "import_data_file", body.clone())),
            selected
        );
        assert_eq!(
            invoke(&app, "import_data_file", body),
            Err(json!({"kind":"noSelection"}))
        );
        let handle = app.state::<DriverHandle>();
        let airspace = assert_ok!(handle.send(GetAirspaceSnapshot).await);
        let waypoints = assert_ok!(handle.send(GetWaypointCatalog).await);
        assert_eq!(
            airspace.catalog.sources.contains_key(name),
            data_type == "airspace"
        );
        assert_eq!(
            waypoints.sources.contains_key(name),
            data_type == "waypoints"
        );
        assert_eq!(
            *airspace.catalog,
            assert_ok!(AirspaceStorage::new(dir.path()).load())
        );
        assert_eq!(
            *waypoints,
            assert_ok!(WaypointStorage::new(dir.path().into()).load())
        );
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn selection_rejects_missing_names_and_unsupported_extensions() {
    for name in ["", "data", "data.mbtiles"] {
        let dir = assert_ok!(tempfile::tempdir());
        let app = app(dir.path(), name, driver());
        let kind = if name.is_empty() {
            "missingName"
        } else {
            "unsupportedType"
        };
        assert_eq!(
            invoke(&app, "select_data_file", json!({})),
            Err(json!({"kind":kind}))
        );
        let state = app.state::<DataImportState>();
        let selection = state.selection.lock().await;
        assert_none!(selection.as_ref().map(|s| &s.info.selection_id));
        assert_eq!(assert_ok!(std::fs::read_dir(dir.path())).count(), 0);
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn concurrent_requests_return_busy_without_consuming_the_selection() {
    let dir = assert_ok!(tempfile::tempdir());
    let app = app(dir.path(), "local.cup", driver());
    let selected = assert_ok!(invoke(&app, "select_data_file", json!({})));
    let state = app.state::<DataImportState>();
    let selection = state.selection.lock().await;
    for command in ["select_data_file", "import_data_file", "discard_data_file"] {
        let body = json!({"selectionId": selected["selectionId"]});
        assert_eq!(invoke(&app, command, body), Err(json!({"kind":"busy"})));
    }
    assert_eq!(
        selection.as_ref().unwrap().info.selection_id,
        selected["selectionId"]
    );
}

#[tokio::test(flavor = "multi_thread")]
#[tracing_test::traced_test]
async fn read_failure_discards_the_selection_and_preserves_installed_data() {
    let dir = assert_ok!(tempfile::tempdir());
    let storage = AirspaceStorage::new(dir.path());
    let bytes = include_bytes!("../../../testdata/airspace/circle.txt");
    assert_ok!(assert_ok!(storage.import_airspace(bytes, "local.txt")));
    let original = assert_ok!(storage.load());
    let app = app(dir.path(), "local.txt", driver());
    let selected = assert_ok!(invoke(&app, "select_data_file", json!({})));
    *app.state::<Picker>().0.lock().unwrap() = Err(FileBytesPickerError::Read {
        display_name: Some("local.txt".into()),
        source: anyhow::anyhow!("test read failure"),
    });
    assert_eq!(
        invoke(&app, "select_data_file", json!({})),
        Err(json!({"kind":"readFailed"}))
    );
    let body = json!({"selectionId":selected["selectionId"]});
    assert_eq!(
        invoke(&app, "import_data_file", body),
        Err(json!({"kind":"noSelection"}))
    );
    assert_eq!(assert_ok!(storage.load()), original);
    let logs = tracing_test::internal::global_buf().lock().unwrap().clone();
    assert!(String::from_utf8_lossy(&logs).contains("Could not select data file"));
}

#[tokio::test(flavor = "multi_thread")]
async fn import_errors_preserve_the_dataset_error_contract() {
    for (name, error) in [
        (
            "local.cup",
            json!({"kind":"waypoints", "error":"driverStopped"}),
        ),
        (
            "local.txt",
            json!({"kind":"airspace", "error":{"kind":"driverStopped", "sourceName":"local.txt"}}),
        ),
    ] {
        let dir = assert_ok!(tempfile::tempdir());
        let app = app(dir.path(), name, DriverHandle::stopped());
        let selected = assert_ok!(invoke(&app, "select_data_file", json!({})));
        let body = json!({"selectionId":selected["selectionId"]});
        assert_eq!(invoke(&app, "import_data_file", body), Err(error));
        assert_eq!(assert_ok!(std::fs::read_dir(dir.path())).count(), 0);
    }
}
