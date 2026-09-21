use super::*;
use crate::test_support::{invoke, spawn_driver};
use claims::{assert_err, assert_ok};
use serde_json::json;
use tauri::Manager;
use updraft_core::{AirspaceState, SettingsSnapshot};

fn app() -> tauri::App<tauri::test::MockRuntime> {
    let directory = tempfile::tempdir().unwrap();
    tauri::test::mock_builder()
        .manage(PinnedTargetsFile::new(directory.path().to_owned()))
        .manage(crate::navigation::NavigationFile::new(
            directory.path().to_owned(),
        ))
        .manage(directory)
        .manage(spawn_driver(
            SettingsSnapshot::default(),
            AirspaceState::none_at_startup(),
        ))
        .invoke_handler(tauri::generate_handler![pin_target, unpin_target])
        .build(tauri::test::mock_context(tauri::test::noop_assets()))
        .unwrap()
}

#[tokio::test(flavor = "multi_thread")]
async fn ipc_persists_order_and_only_target_identity_then_removes_pins() {
    let app = app();
    let file = app.state::<PinnedTargetsFile>();
    assert_eq!(assert_ok!(file.load()), vec![]);
    for target in [
        json!({"type":"waypoint","name":"Home","latitudeDegrees":50.,"longitudeDegrees":6.,"elevationMeters":100.}),
        json!({"type":"traffic","id":"icao:ABC123"}),
        json!({"type":"mapPosition","latitudeDegrees":50.,"longitudeDegrees":6.}),
    ] {
        assert_eq!(
            assert_ok!(invoke(&app, "pin_target", json!({"target":target}))),
            json!(true)
        );
    }
    let saved = assert_ok!(file.load());
    assert_eq!(saved.len(), 3);
    assert_eq!(
        assert_ok!(serde_json::to_value(&saved[1])),
        json!({"id":1,"target":{"type":"traffic","id":"icao:ABC123"}})
    );
    let mut restored = updraft_core::Core::new(SettingsSnapshot::default());
    assert_ok!(
        restored
            .apply(
                updraft_core::RestorePinnedTargets(saved),
                Default::default()
            )
            .response
    );
    assert_eq!(
        assert_ok!(invoke(&app, "unpin_target", json!({"id":1}))),
        json!(true)
    );
    assert_eq!(
        assert_ok!(file.load())
            .iter()
            .map(|pin| pin.id)
            .collect::<Vec<_>>(),
        vec![0, 2]
    );
    assert_eq!(
        assert_ok!(serde_json::to_value(assert_ok!(
            app.state::<crate::navigation::NavigationFile>()
                .load_recents()
        ))),
        json!([{"type":"traffic","id":"icao:ABC123"}])
    );
    assert_err!(invoke(
        &app,
        "pin_target",
        json!({"target":{"type":"mapPosition","latitudeDegrees":91.,"longitudeDegrees":0.}})
    ));
    assert_eq!(assert_ok!(file.load()).len(), 2);
}

#[tokio::test(flavor = "multi_thread")]
#[tracing_test::traced_test]
async fn failed_save_keeps_live_pins_and_retry_saves_the_current_list() {
    let app = app();
    let file = app.state::<PinnedTargetsFile>();
    assert_ok!(std::fs::create_dir(&file.path));
    let target = json!({"target":{"type":"traffic","id":"icao:ABC123"}});
    assert_eq!(
        assert_ok!(invoke(&app, "pin_target", target.clone())),
        json!(false)
    );
    assert_ok!(std::fs::remove_dir(&file.path));
    assert_eq!(assert_ok!(invoke(&app, "pin_target", target)), json!(true));
    assert_eq!(assert_ok!(file.load()).len(), 1);
    let logs = tracing_test::internal::global_buf().lock().unwrap().clone();
    assert!(String::from_utf8_lossy(&logs).contains("Could not save pinned targets"));
}
