use super::*;
use claims::{assert_err, assert_ok};
use serde_json::{Value, json};
use std::fs::{self, FileTimes, OpenOptions};
use tauri::Manager;

#[test]
fn initial_delivery_failure_does_not_register_a_channel() {
    let directory = tempfile::tempdir().unwrap();
    let service = CatalogService::load(directory.path().join("catalog.json"));
    let subscriptions = CatalogSubscriptions::new(&service);
    let channel = Channel::new(|_| Err(std::io::Error::other("closed").into()));
    assert_err!(subscriptions.subscribe(channel));
    assert!(subscriptions.channels.lock().unwrap().is_empty());
}

#[test]
fn catalog_subscription_serializes_metadata_through_ipc() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("catalog.json");
    let json = br#"{"maps":[{"path":"Europe/Germany.mbtiles","size":10,"time":"20260908"}]}"#;
    assert_ok!(fs::write(&path, json));
    let file = assert_ok!(OpenOptions::new().write(true).open(&path));
    let timestamp = std::time::UNIX_EPOCH + std::time::Duration::from_secs(1234);
    assert_ok!(file.set_times(FileTimes::new().set_modified(timestamp)));
    let service = CatalogService::load(path);
    let subscriptions = CatalogSubscriptions::new(&service);
    let (sender, messages) = std::sync::mpsc::channel::<Value>();
    let app = tauri::test::mock_builder()
        .manage(subscriptions)
        .channel_interceptor(move |_, _, _, body| {
            sender.send(body.clone().deserialize().unwrap()).unwrap();
            true
        })
        .invoke_handler(tauri::generate_handler![
            subscribe_enroute_catalog,
            unsubscribe_enroute_catalog
        ])
        .build(tauri::test::mock_context(tauri::test::noop_assets()))
        .unwrap();
    let window = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
        .build()
        .unwrap();
    let invoke = |command: &str, body| {
        let request = tauri::webview::InvokeRequest {
            cmd: command.into(),
            callback: tauri::ipc::CallbackFn(0),
            error: tauri::ipc::CallbackFn(1),
            url: "tauri://localhost".parse().unwrap(),
            body: tauri::ipc::InvokeBody::Json(body),
            headers: Default::default(),
            invoke_key: tauri::test::INVOKE_KEY.into(),
        };
        tauri::test::get_ipc_response(&window, request).map(|r| r.deserialize::<Value>().unwrap())
    };
    assert_ok!(invoke(
        "subscribe_enroute_catalog",
        json!({"channel":"__CHANNEL__:42"})
    ));
    let mut initial = assert_ok!(messages.try_recv());
    insta::assert_json_snapshot!(initial);
    service.state.lock().unwrap().error = true;
    service.publish();
    initial["error"] = json!(true);
    let update = messages.recv_timeout(std::time::Duration::from_secs(2));
    assert_eq!(assert_ok!(update), initial);
    for _ in 0..2 {
        assert_ok!(invoke(
            "unsubscribe_enroute_catalog",
            json!({"channelId":42})
        ));
    }
    assert!(
        app.state::<CatalogSubscriptions>()
            .channels
            .lock()
            .unwrap()
            .is_empty()
    );
}

#[test]
#[tracing_test::traced_test]
fn available_updates_use_cached_catalog_and_report_read_failures_through_ipc() {
    for (extension, command, kind) in [
        ("mbtiles", "get_enroute_basemap_updates", "Basemap"),
        ("terrain", "get_enroute_terrain_updates", "Terrain"),
    ] {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("catalog.json");
        assert_ok!(fs::write(
            &path,
            br#"{"maps":[{"path":"Europe/Germany.mbtiles","size":10,"time":"20260908"}]}"#
        ));
        let relative = format!("Europe/Germany.{extension}");
        let installed = directory.path().join("enroute").join(&relative);
        assert_ok!(fs::create_dir_all(installed.parent().unwrap()));
        assert_ok!(fs::write(&installed, b"disabled"));
        assert_ok!(fs::write(
            installed.with_extension(format!("{extension}.disabled")),
            b""
        ));
        let file = assert_ok!(OpenOptions::new().write(true).open(&installed));
        assert_ok!(file.set_times(FileTimes::new().set_modified(std::time::UNIX_EPOCH)));
        let basemaps = assert_ok!(crate::basemap::Basemaps::load(directory.path()));
        let terrain = assert_ok!(crate::terrain::Terrain::load(directory.path()));
        let service = Arc::new(CatalogService::load(path));
        if extension == "terrain" {
            let cached = service.status().cached.unwrap();
            let mut entries = cached.entries.clone();
            entries[0].path = "Europe/Germany.terrain";
            service.state.lock().unwrap().cached = Some(Arc::new(super::super::CachedCatalog {
                entries,
                checked_at: cached.checked_at,
            }));
        }
        let app = tauri::test::mock_builder()
            .manage(service.clone())
            .manage(Arc::new(Mutex::new(basemaps)))
            .manage(Arc::new(Mutex::new(terrain)))
            .invoke_handler(tauri::generate_handler![
                get_enroute_basemap_updates,
                get_enroute_terrain_updates
            ])
            .build(tauri::test::mock_context(tauri::test::noop_assets()))
            .unwrap();
        let window = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
            .build()
            .unwrap();
        let invoke = || {
            let request = tauri::webview::InvokeRequest {
                cmd: command.into(),
                callback: tauri::ipc::CallbackFn(0),
                error: tauri::ipc::CallbackFn(1),
                url: "tauri://localhost".parse().unwrap(),
                body: tauri::ipc::InvokeBody::Json(json!({})),
                headers: Default::default(),
                invoke_key: tauri::test::INVOKE_KEY.into(),
            };
            tauri::test::get_ipc_response(&window, request)
                .map(|r| r.deserialize::<Value>().unwrap())
        };
        let updates = assert_ok!(invoke());
        assert_eq!(updates, json!([relative]));
        service.state.lock().unwrap().error = true;
        assert_eq!(assert_ok!(invoke()), updates);
        assert_ok!(fs::remove_file(installed));
        let error = format!("Could not check {} updates", kind.to_lowercase());
        assert_eq!(assert_err!(invoke()), json!(error));
        // Tauri dispatches IPC commands outside the test span.
        assert!(tracing_test::internal::logs_with_scope_contain(
            module_path!().trim_end_matches("::tests"),
            &error
        ));
        service.state.lock().unwrap().cached = None;
        assert_eq!(
            assert_err!(invoke()),
            json!(format!("{kind} catalog is unavailable"))
        );
    }
}
