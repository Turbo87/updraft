use super::*;
use crate::{
    driver::Driver,
    file_picker::{FileBytesPicker, FileBytesPickerFuture, PickedFileBytes},
};
use claims::{assert_err, assert_ok};
use serde_json::{Value, json};
use std::{sync::Mutex, time::Duration};
use tauri::{Manager, test::MockRuntime};
use updraft_core::{AirspaceState, SettingsSnapshot, WaypointSource};

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
        .invoke_handler(tauri::generate_handler![
            import_waypoints,
            remove_waypoints,
            set_waypoints_enabled
        ])
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
    let WaypointSource::Active(dataset) = &catalog.sources["local.cup"] else {
        panic!("an active source")
    };
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
#[tracing_test::traced_test]
async fn invalid_replacement_publishes_an_unavailable_source() {
    use updraft_core::WaypointLoadError;

    let dir = assert_ok!(tempfile::tempdir());
    let storage = WaypointStorage::new(dir.path().to_owned());
    assert_ok!(assert_ok!(storage.import("local.cup", CUP)));
    assert_ok!(assert_ok!(storage.import("other.cup", CUP)));
    let original = Arc::new(assert_ok!(storage.load()));
    let app = app(storage.clone(), Some(b"invalid"), driver());
    let handle = app.state::<DriverHandle>();
    assert_ok!(handle.send(ReplaceWaypointCatalog(original.clone())).await);
    assert_eq!(
        assert_ok!(invoke(&app, "import_waypoints", json!({}))),
        json!({"type": "imported", "sourceName": "local.cup"})
    );
    let catalog = assert_ok!(handle.send(GetWaypointCatalog).await);
    assert_eq!(
        catalog.sources["local.cup"],
        WaypointSource::Unavailable(WaypointLoadError::ParseFailed)
    );
    assert_eq!(catalog.sources["other.cup"], original.sources["other.cup"]);
    assert_eq!(assert_ok!(storage.load()), *catalog);
    let logs = tracing_test::internal::global_buf().lock().unwrap().clone();
    assert!(String::from_utf8_lossy(&logs).contains("Could not parse stored waypoint source"));
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
    assert_ok!(assert_ok!(storage.import("a.cup", CUP)));
    assert_ok!(assert_ok!(storage.import("b.cup", CUP)));
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
    assert_ok!(assert_ok!(storage.import("a.cup", CUP)));
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
        ("set_waypoints_enabled", true),
    ] {
        let dir = assert_ok!(tempfile::tempdir());
        let storage = WaypointStorage::new(dir.path().to_owned());
        assert_ok!(assert_ok!(storage.import("other.cup", CUP)));
        if installed {
            assert_ok!(assert_ok!(storage.import("local.cup", CUP)));
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
        let args = json!({"sourceName": "local.cup", "enabled": false});
        let error = assert_err!(invoke(&app, command, args));
        assert_eq!(error, json!("driverStopped"));
        let catalog = assert_ok!(storage.load());
        assert_eq!(catalog.sources["other.cup"], original.sources["other.cup"]);
        if command == "remove_waypoints" {
            assert_eq!(catalog.sources.keys().collect::<Vec<_>>(), ["other.cup"]);
        } else if command == "set_waypoints_enabled" {
            assert_eq!(catalog.sources["local.cup"], WaypointSource::Disabled);
        } else {
            assert_eq!(catalog.sources.len(), 2);
            let WaypointSource::Active(dataset) = &catalog.sources["local.cup"] else {
                panic!("an active source")
            };
            assert_eq!(dataset.waypoints()[0].name, "Replacement");
        }
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn activation_persists_and_refreshes_waypoint_resources() {
    use crate::waypoints::{arrival_resource::ArrivalResource, resource::waypoint_geojson};
    use updraft_core::{GetGlideSnapshot, WaypointSourceStatus};
    use updraft_geo::BoundingBox;
    use updraft_units::Angle;

    let dir = assert_ok!(tempfile::tempdir());
    let storage = WaypointStorage::new(dir.path().to_owned());
    for name in ["local.cup", "other.cup"] {
        assert_ok!(assert_ok!(storage.import(name, CUP)));
    }
    let handle = driver();
    let catalog = Arc::new(assert_ok!(storage.load()));
    assert_ok!(handle.send(ReplaceWaypointCatalog(catalog)).await);
    let bounds = BoundingBox::new(
        Angle::from_degrees(49.),
        Angle::from_degrees(51.),
        Angle::from_degrees(5.),
        Angle::from_degrees(7.),
    );
    for (generation, enabled) in [(2, false), (3, true)] {
        let app = app(storage.clone(), None, handle.clone());
        let args = json!({"sourceName": "local.cup", "enabled": enabled});
        assert_eq!(
            assert_ok!(invoke(&app, "set_waypoints_enabled", args)),
            Value::Null
        );
        let snapshot = assert_ok!(handle.send(GetGlideSnapshot).await);
        assert_eq!(snapshot.waypoints.generation, generation);
        assert_eq!(
            *snapshot.waypoints.catalog,
            assert_ok!(WaypointStorage::new(dir.path().to_owned()).load())
        );
        let status = &snapshot.waypoints.catalog.status(generation).sources[0];
        if enabled {
            std::assert_matches!(status, WaypointSourceStatus::Active { .. });
        } else {
            assert_eq!(
                status,
                &WaypointSourceStatus::Disabled {
                    source_name: "local.cup".into()
                }
            );
        }
        let resource = assert_ok!(ArrivalResource::calculate(&snapshot, bounds));
        let arrivals: Value = assert_ok!(serde_json::from_slice(&resource.body));
        let waypoints = waypoint_geojson(generation, &snapshot.waypoints.catalog);
        let expected = if enabled {
            vec![format!("{generation}:0:0"), format!("{generation}:1:0")]
        } else {
            vec![format!("{generation}:1:0")]
        };
        for resource in [waypoints, arrivals] {
            let ids: Vec<_> = resource["features"]
                .as_array()
                .unwrap()
                .iter()
                .map(|feature| feature["id"].as_str().unwrap())
                .collect();
            assert_eq!(ids, expected);
        }
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn activation_rejects_unknown_sources_and_concurrent_mutations() {
    let dir = assert_ok!(tempfile::tempdir());
    let storage = WaypointStorage::new(dir.path().to_owned());
    let args = json!({"sourceName": "missing.cup", "enabled": false});
    let test_app = app(storage.clone(), None, driver());
    assert_eq!(
        assert_err!(invoke(&test_app, "set_waypoints_enabled", args.clone())),
        json!("notFound")
    );
    assert!(assert_ok!(storage.load()).sources.is_empty());
    for command in [
        "import_waypoints",
        "remove_waypoints",
        "set_waypoints_enabled",
    ] {
        let test_app = app(storage.clone(), None, driver());
        let state = test_app.state::<WaypointCommandState>();
        let _guard = assert_ok!(state.mutation.try_lock());
        assert_eq!(
            assert_err!(invoke(&test_app, command, args.clone())),
            json!("busy")
        );
    }
}

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread")]
#[tracing_test::traced_test]
async fn failed_activation_save_keeps_confirmed_state() {
    use std::fs::{Permissions, set_permissions};
    use std::os::unix::fs::PermissionsExt;
    use updraft_core::GetWaypointSnapshot;

    let dir = assert_ok!(tempfile::tempdir());
    let storage = WaypointStorage::new(dir.path().to_owned());
    assert_ok!(assert_ok!(storage.import("local.cup", CUP)));
    for enabled in [false, true] {
        assert_ok!(storage.set_enabled("local.cup", !enabled));
        let original = Arc::new(assert_ok!(storage.load()));
        let handle = driver();
        assert_ok!(handle.send(ReplaceWaypointCatalog(original.clone())).await);
        let app = app(storage.clone(), None, handle.clone());
        let path = dir.path().join("waypoints");
        assert_ok!(set_permissions(&path, Permissions::from_mode(0o500)));
        let args = json!({"sourceName": "local.cup", "enabled": enabled});
        let result = invoke(&app, "set_waypoints_enabled", args);
        assert_ok!(set_permissions(&path, Permissions::from_mode(0o700)));
        assert_eq!(assert_err!(result), json!("storageFailed"));
        let snapshot = assert_ok!(handle.send(GetWaypointSnapshot).await);
        assert_eq!(snapshot.generation, 1);
        assert_eq!(snapshot.catalog, original);
        assert_eq!(assert_ok!(storage.load()), *original);
    }
    let logs = tracing_test::internal::global_buf().lock().unwrap().clone();
    assert!(String::from_utf8_lossy(&logs).contains("Could not persist waypoint activation"));
}
