use crate::driver::{Driver, DriverHandle};
use serde_json::Value;
use std::time::Duration;
use updraft_core::{AirspaceState, SettingsSnapshot};

pub fn request(command: &str, body: Value) -> tauri::webview::InvokeRequest {
    tauri::webview::InvokeRequest {
        cmd: command.to_owned(),
        callback: tauri::ipc::CallbackFn(0),
        error: tauri::ipc::CallbackFn(1),
        url: "tauri://localhost".parse().expect("valid test URL"),
        body: tauri::ipc::InvokeBody::Json(body),
        headers: Default::default(),
        invoke_key: tauri::test::INVOKE_KEY.to_owned(),
    }
}

pub fn invoke(
    app: &tauri::App<tauri::test::MockRuntime>,
    command: &str,
    body: Value,
) -> Result<Value, Value> {
    use tauri::Manager;
    let window = app.get_webview_window("main").unwrap_or_else(|| {
        tauri::WebviewWindowBuilder::new(app, "main", Default::default())
            .build()
            .expect("the test webview should build")
    });
    tauri::test::get_ipc_response(&window, request(command, body)).map(|response| {
        response
            .deserialize()
            .expect("the IPC response should deserialize")
    })
}

pub fn capture_channels(
    receive: impl Fn(Value) + Send + Sync + 'static,
) -> tauri::Builder<tauri::test::MockRuntime> {
    tauri::test::mock_builder().channel_interceptor(move |_, _, _, body| {
        receive(
            body.clone()
                .deserialize()
                .expect("the channel message should deserialize"),
        );
        true
    })
}

pub fn spawn_driver(snapshot: SettingsSnapshot, airspace: AirspaceState) -> DriverHandle {
    Driver::spawn(
        snapshot,
        airspace,
        Box::new(|_, _, _| Box::new(|| {})),
        Box::new(|_| {}),
        Duration::from_millis(100),
    )
}
