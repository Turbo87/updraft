use crate::driver::DriverHandle;
use serde::Serialize;
use tauri::ipc::Channel;
use tauri_plugin_updraft::{BondedBluetoothDevices, UpdraftMobileExt};
use updraft_core::{
    AddExternalDevice, ConnectionSpec, DeleteExternalDevice, EditExternalDevice, ExternalDeviceId,
    InvalidExternalDeviceOrder, ReorderExternalDevices, SetExternalDeviceEnabled, SetLocale,
    SetUnits, Topic, UnitSettings, UnknownExternalDevice,
};

#[derive(Debug, Serialize, thiserror::Error)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum DriverCommandError {
    #[error("driver stopped")]
    DriverStopped,
}

#[derive(Debug, Serialize, thiserror::Error)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum ExistingExternalDeviceCommandError {
    #[error("driver stopped")]
    DriverStopped,
    #[error("unknown external device: {device_id:?}")]
    UnknownExternalDevice { device_id: ExternalDeviceId },
}

#[derive(Debug, Serialize, thiserror::Error)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum ReorderExternalDevicesCommandError {
    #[error("driver stopped")]
    DriverStopped,
    #[error("invalid external device order")]
    InvalidExternalDeviceOrder,
}

#[derive(Debug, Serialize, thiserror::Error)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum BondedBluetoothDevicesCommandError {
    #[error("bonded Bluetooth device query failed")]
    QueryFailed,
}

#[tauri::command]
pub fn bonded_bluetooth_devices<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
) -> Result<BondedBluetoothDevices, BondedBluetoothDevicesCommandError> {
    app.updraft_mobile()
        .bonded_bluetooth_devices()
        .map_err(|_| BondedBluetoothDevicesCommandError::QueryFailed)
}

#[tauri::command]
pub async fn set_locale(
    locale: updraft_core::Locale,
    handle: tauri::State<'_, DriverHandle>,
) -> Result<(), DriverCommandError> {
    let input = SetLocale::new(locale);
    handle
        .send(input)
        .await
        .map_err(|_| DriverCommandError::DriverStopped)
}

#[tauri::command]
pub async fn set_units(
    units: UnitSettings,
    handle: tauri::State<'_, DriverHandle>,
) -> Result<(), DriverCommandError> {
    let input = SetUnits::new(units);
    handle
        .send(input)
        .await
        .map_err(|_| DriverCommandError::DriverStopped)
}

#[tauri::command]
pub async fn add_external_device(
    spec: ConnectionSpec,
    handle: tauri::State<'_, DriverHandle>,
) -> Result<ExternalDeviceId, DriverCommandError> {
    let input = AddExternalDevice::new(spec);
    handle
        .send(input)
        .await
        .map_err(|_| DriverCommandError::DriverStopped)
}

#[tauri::command]
pub async fn delete_external_device(
    device_id: ExternalDeviceId,
    handle: tauri::State<'_, DriverHandle>,
) -> Result<(), ExistingExternalDeviceCommandError> {
    let input = DeleteExternalDevice::new(device_id);
    handle
        .send(input)
        .await
        .map_err(|_| ExistingExternalDeviceCommandError::DriverStopped)?
        .map_err(|UnknownExternalDevice { device_id }| {
            ExistingExternalDeviceCommandError::UnknownExternalDevice { device_id }
        })
}

#[tauri::command]
pub async fn reorder_external_devices(
    order: Vec<ExternalDeviceId>,
    handle: tauri::State<'_, DriverHandle>,
) -> Result<(), ReorderExternalDevicesCommandError> {
    let input = ReorderExternalDevices::new(order);
    handle
        .send(input)
        .await
        .map_err(|_| ReorderExternalDevicesCommandError::DriverStopped)?
        .map_err(|InvalidExternalDeviceOrder| {
            ReorderExternalDevicesCommandError::InvalidExternalDeviceOrder
        })
}

#[tauri::command]
pub async fn edit_external_device(
    device_id: ExternalDeviceId,
    spec: ConnectionSpec,
    handle: tauri::State<'_, DriverHandle>,
) -> Result<(), ExistingExternalDeviceCommandError> {
    let input = EditExternalDevice::new(device_id, spec);
    handle
        .send(input)
        .await
        .map_err(|_| ExistingExternalDeviceCommandError::DriverStopped)?
        .map_err(|UnknownExternalDevice { device_id }| {
            ExistingExternalDeviceCommandError::UnknownExternalDevice { device_id }
        })
}

#[tauri::command]
pub async fn set_external_device_enabled(
    device_id: ExternalDeviceId,
    enabled: bool,
    handle: tauri::State<'_, DriverHandle>,
) -> Result<(), ExistingExternalDeviceCommandError> {
    let input = if enabled {
        SetExternalDeviceEnabled::enabled(device_id)
    } else {
        SetExternalDeviceEnabled::disabled(device_id)
    };
    handle
        .send(input)
        .await
        .map_err(|_| ExistingExternalDeviceCommandError::DriverStopped)?
        .map_err(|UnknownExternalDevice { device_id }| {
            ExistingExternalDeviceCommandError::UnknownExternalDevice { device_id }
        })
}

/// Registers the webview's channel as a subscriber.
///
/// `Channel::send` fails once the webview is gone, which is exactly the
/// signal the driver uses to prune the sink.
#[tauri::command]
pub fn subscribe(channel: Channel<Topic>, handle: tauri::State<'_, DriverHandle>) {
    handle.subscribe(Box::new(move |topic: &Topic| {
        channel.send(topic.clone()).is_ok()
    }));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::driver::Driver;
    use serde_json::{Value, json};
    use std::time::Duration;
    use updraft_core::{AirspaceState, SettingsSnapshot};

    fn app() -> tauri::App<tauri::test::MockRuntime> {
        let handle = Driver::spawn(
            SettingsSnapshot::default(),
            AirspaceState::none_at_startup(),
            Box::new(|_, _, _| Box::new(|| {})),
            Box::new(|_| {}),
            Duration::from_millis(100),
        );

        tauri::test::mock_builder()
            .manage(handle)
            .plugin(tauri_plugin_updraft::init())
            .invoke_handler(tauri::generate_handler![
                bonded_bluetooth_devices,
                set_locale,
                set_units,
                add_external_device,
                delete_external_device,
                reorder_external_devices,
                edit_external_device,
                set_external_device_enabled,
                subscribe,
            ])
            .build(tauri::test::mock_context(tauri::test::noop_assets()))
            .expect("the IPC test app should build")
    }

    fn request(command: &str, body: Value) -> tauri::webview::InvokeRequest {
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

    #[tokio::test(flavor = "multi_thread")]
    async fn bonded_bluetooth_devices_reports_unsupported_on_desktop() {
        let app = app();
        let webview = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
            .build()
            .expect("the IPC test webview should build");

        let request = request("bonded_bluetooth_devices", json!({}));
        let response = tauri::test::get_ipc_response(&webview, request)
            .expect("the bonded-device query should succeed")
            .deserialize::<Value>()
            .expect("the bonded-device result should deserialize");

        assert_eq!(response, json!({ "status": "unsupported" }));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn set_units_deserializes_complete_unit_settings() {
        let app = app();
        let webview = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
            .build()
            .expect("the IPC test webview should build");

        let body = json!({
            "units": {
                "altitude": "ft",
                "distance": "nm",
                "speed": "kt",
                "verticalSpeed": "ft/min"
            }
        });
        let request = request("set_units", body);
        let response = tauri::test::get_ipc_response(&webview, request)
            .expect("the unit selections should be accepted")
            .deserialize::<Value>()
            .expect("the empty command response should deserialize");

        assert_eq!(response, Value::Null);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn add_external_device_returns_the_allocated_tcp_device_id() {
        let app = app();
        let webview = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
            .build()
            .expect("the IPC test webview should build");

        let body = json!({
            "spec": { "type": "tcp", "host": "127.0.0.1", "port": 4353 }
        });
        let request = request("add_external_device", body);
        let response = tauri::test::get_ipc_response(&webview, request)
            .expect("the external device should be added")
            .deserialize::<Value>()
            .expect("the allocated device ID should deserialize");

        assert_eq!(response, json!(1));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn delete_external_device_serializes_an_unknown_device_id() {
        let app = app();
        let webview = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
            .build()
            .expect("the IPC test webview should build");

        let body = json!({ "deviceId": 99 });
        let request = request("delete_external_device", body);
        let response = tauri::test::get_ipc_response(&webview, request)
            .expect_err("an unknown device ID should be rejected");

        assert_eq!(
            response,
            json!({ "kind": "unknownExternalDevice", "deviceId": 99 })
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn reorder_external_devices_serializes_an_invalid_order() {
        let app = app();
        let webview = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
            .build()
            .expect("the IPC test webview should build");

        let body = json!({ "order": [99] });
        let request = request("reorder_external_devices", body);
        let response = tauri::test::get_ipc_response(&webview, request)
            .expect_err("an invalid order should be rejected");

        assert_eq!(response, json!({ "kind": "invalidExternalDeviceOrder" }));
    }
}
