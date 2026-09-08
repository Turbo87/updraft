use super::*;
use crate::enroute::queue::DownloadOutcome;
use claims::{assert_err, assert_ok, assert_some};
use serde_json::{Value, json};
use tauri::Manager;

#[test]
fn failed_initial_delivery_does_not_register_a_subscriber() {
    let state = DownloadCommands::default();
    let channel = Channel::new(|_| Err(std::io::Error::other("closed channel").into()));
    assert_err!(state.subscribe(channel));
    assert!(state.subscribers.lock().unwrap().is_empty());
}

#[tokio::test]
async fn subscription_delivers_queue_changes_through_ipc_and_can_be_closed() {
    let state = DownloadCommands::default();
    let queue = state.queue.clone();
    let (sender, mut messages) = tokio::sync::mpsc::unbounded_channel::<Value>();
    let app = tauri::test::mock_builder()
        .manage(state)
        .channel_interceptor(move |_, _, _, body| {
            sender.send(body.clone().deserialize().unwrap()).unwrap();
            true
        })
        .invoke_handler(tauri::generate_handler![
            subscribe_enroute_downloads,
            unsubscribe_enroute_downloads
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
        tauri::test::get_ipc_response(&window, request)
            .map(|response| response.deserialize::<Value>().unwrap())
    };
    let subscribe = json!({"channel":"__CHANNEL__:42"});
    assert_eq!(
        assert_ok!(invoke("subscribe_enroute_downloads", subscribe)),
        Value::Null
    );
    assert_eq!(assert_ok!(messages.try_recv()), json!([]));
    let json = br#"{"maps":[{"path":"Europe/Germany.mbtiles","size":10,"time":"20260908"}]}"#;
    let entry = assert_ok!(crate::enroute::parse_catalog(json)).remove(0);
    let attempt = {
        let mut queue = queue.lock().unwrap();
        queue.enqueue(entry);
        let attempt = assert_some!(queue.start_next());
        queue.report_progress(&attempt, 3);
        attempt
    };
    let progress = tokio::time::timeout(std::time::Duration::from_secs(2), async {
        loop {
            let message = assert_some!(messages.recv().await);
            if message[0]["downloaded"] == 3 {
                break message;
            }
        }
    })
    .await;
    insta::assert_json_snapshot!(assert_ok!(progress));
    for _ in 0..2 {
        let body = json!({"channelId":42});
        assert_eq!(
            assert_ok!(invoke("unsubscribe_enroute_downloads", body)),
            Value::Null
        );
    }
    queue
        .lock()
        .unwrap()
        .finish(&attempt, DownloadOutcome::Installed);
    let state = app.state::<DownloadCommands>();
    assert!(state.subscribers.lock().unwrap().is_empty());
    assert_err!(messages.try_recv());
}
