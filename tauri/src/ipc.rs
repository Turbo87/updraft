use crate::airspace_storage::AirspaceStorage;
use crate::driver::DriverHandle;
use crate::file_picker::PickedFileBytes;
use serde::Serialize;
use std::sync::Arc;
use tauri::ipc::Channel;
use tauri_plugin_updraft::{BondedBluetoothDevices, UpdraftMobileExt};
use tokio::sync::Mutex;
use updraft_core::{
    AddExternalDevice, AirspaceCatalog, ConnectionSpec, DeleteExternalDevice, EditExternalDevice,
    ExternalDeviceId, GetAirspaceSnapshot, InvalidExternalDeviceOrder, ReorderExternalDevices,
    ReplaceAirspaceCatalog, SetExternalDeviceEnabled, SetLocale, SetPolar, SetUnits, Topic,
    UnitSettings, UnknownExternalDevice,
};

pub struct AirspaceCommandState {
    storage: AirspaceStorage,
    mutation: Arc<Mutex<()>>,
}

impl AirspaceCommandState {
    pub fn new(storage: AirspaceStorage) -> Self {
        Self {
            storage,
            mutation: Arc::new(Mutex::new(())),
        }
    }

    pub async fn import_selected(
        &self,
        selected: PickedFileBytes,
        handle: &DriverHandle,
    ) -> Result<(), AirspaceCommandError> {
        let _guard = self
            .mutation
            .try_lock()
            .map_err(|_| AirspaceCommandError::Busy)?;
        let storage = self.storage.clone();
        let source_name = selected
            .display_name
            .filter(|name| !name.is_empty())
            .ok_or(AirspaceCommandError::MissingName)?;
        let snapshot = handle.send(GetAirspaceSnapshot).await.map_err(|_| {
            AirspaceCommandError::DriverStopped {
                source_name: Some(source_name.clone()),
            }
        })?;
        let name = source_name.clone();
        let dataset =
            tokio::task::spawn_blocking(move || storage.import_airspace(&selected.bytes, &name))
                .await
                .map_err(|error| {
                    tracing::warn!(%error, "Airspace import worker failed");
                    AirspaceCommandError::WorkerFailed
                })?
                .map_err(|error| {
                    tracing::warn!(%error, "Could not store airspace source");
                    AirspaceCommandError::StorageFailed {
                        source_name: Some(source_name.clone()),
                    }
                })?;
        let mut catalog = (*snapshot.catalog).clone();
        catalog.sources.insert(source_name.clone(), dataset.into());
        activate_airspace_catalog(handle, catalog, source_name).await
    }
}

#[derive(Debug, Serialize, thiserror::Error)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum AirspaceCommandError {
    #[error("airspace source does not exist")]
    NotFound { source_name: String },
    #[error("could not persist airspace source")]
    StorageFailed {
        #[serde(skip_serializing_if = "Option::is_none")]
        source_name: Option<String>,
    },
    #[error("driver stopped")]
    DriverStopped {
        #[serde(skip_serializing_if = "Option::is_none")]
        source_name: Option<String>,
    },
    #[error("selected airspace file has no display name")]
    MissingName,
    #[error("airspace worker failed")]
    WorkerFailed,
    #[error("another airspace mutation is active")]
    Busy,
}

#[tauri::command]
pub async fn remove_airspace(
    source_name: String,
    state: tauri::State<'_, AirspaceCommandState>,
    handle: tauri::State<'_, DriverHandle>,
) -> Result<(), AirspaceCommandError> {
    let _mutation = state
        .mutation
        .try_lock()
        .map_err(|_| AirspaceCommandError::Busy)?;
    let snapshot = handle.send(GetAirspaceSnapshot).await.map_err(|_| {
        AirspaceCommandError::DriverStopped {
            source_name: Some(source_name.clone()),
        }
    })?;
    let mut catalog = (*snapshot.catalog).clone();
    if catalog.sources.remove(&source_name).is_none() {
        return Ok(());
    }
    let storage = state.storage.clone();
    let name = source_name.clone();
    tokio::task::spawn_blocking(move || storage.remove(&name))
        .await
        .map_err(|error| {
            tracing::warn!(%error, "Airspace removal worker failed");
            AirspaceCommandError::WorkerFailed
        })?
        .map_err(|error| {
            tracing::warn!(%error, "Could not remove stored airspace source");
            AirspaceCommandError::StorageFailed {
                source_name: Some(source_name.clone()),
            }
        })?;
    activate_airspace_catalog(&handle, catalog, source_name).await
}

#[tauri::command]
pub async fn set_airspace_enabled(
    source_name: String,
    enabled: bool,
    state: tauri::State<'_, AirspaceCommandState>,
    handle: tauri::State<'_, DriverHandle>,
) -> Result<(), AirspaceCommandError> {
    let _mutation = state
        .mutation
        .try_lock()
        .map_err(|_| AirspaceCommandError::Busy)?;
    let snapshot = handle.send(GetAirspaceSnapshot).await.map_err(|_| {
        AirspaceCommandError::DriverStopped {
            source_name: Some(source_name.clone()),
        }
    })?;
    let mut catalog = (*snapshot.catalog).clone();
    let Some(source) = catalog.sources.get_mut(&source_name) else {
        return Err(AirspaceCommandError::NotFound { source_name });
    };
    let storage = state.storage.clone();
    let name = source_name.clone();
    *source = tokio::task::spawn_blocking(move || storage.set_enabled(&name, enabled))
        .await
        .map_err(|error| {
            tracing::warn!(%error, "Airspace activation worker failed");
            AirspaceCommandError::WorkerFailed
        })?
        .map_err(|error| {
            tracing::warn!(%error, "Could not persist airspace activation");
            AirspaceCommandError::StorageFailed {
                source_name: Some(source_name.clone()),
            }
        })?;
    activate_airspace_catalog(&handle, catalog, source_name).await
}

async fn activate_airspace_catalog(
    handle: &DriverHandle,
    catalog: AirspaceCatalog,
    source_name: String,
) -> Result<(), AirspaceCommandError> {
    handle
        .send(ReplaceAirspaceCatalog(Arc::new(catalog)))
        .await
        .map_err(|error| {
            tracing::warn!(%error, "Could not activate airspace catalog");
            AirspaceCommandError::DriverStopped {
                source_name: Some(source_name),
            }
        })
}

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

/// Ends the app at the pilot's request.
#[tauri::command]
pub fn quit<R: tauri::Runtime>(app: tauri::AppHandle<R>) {
    tracing::info!("Quitting at the pilot's request");
    if let Err(error) = app.updraft_mobile().quit() {
        tracing::error!(%error, "Could not quit");
    }
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
pub fn get_polars() -> Vec<updraft_core::PolarId> {
    updraft_core::PolarId::all().collect()
}

#[tauri::command]
pub async fn set_bugs(
    bugs: updraft_core::Bugs,
    handle: tauri::State<'_, DriverHandle>,
) -> Result<(), DriverCommandError> {
    handle
        .send(updraft_core::SetBugs { bugs })
        .await
        .map_err(|_| DriverCommandError::DriverStopped)
}

#[tauri::command]
pub async fn set_ballast(
    ballast: updraft_core::Ballast,
    handle: tauri::State<'_, DriverHandle>,
) -> Result<(), DriverCommandError> {
    handle
        .send(updraft_core::SetBallast { ballast })
        .await
        .map_err(|_| DriverCommandError::DriverStopped)
}

#[tauri::command]
pub async fn set_mac_cready(
    mac_cready: updraft_core::MacCready,
    handle: tauri::State<'_, DriverHandle>,
) -> Result<(), DriverCommandError> {
    handle
        .send(updraft_core::SetMacCready { mac_cready })
        .await
        .map_err(|_| DriverCommandError::DriverStopped)
}

#[tauri::command]
pub async fn set_climb_average_method(
    method: updraft_core::ClimbAverageMethod,
    handle: tauri::State<'_, DriverHandle>,
) -> Result<(), DriverCommandError> {
    handle
        .send(updraft_core::SetClimbAverageMethod { method })
        .await
        .map_err(|_| DriverCommandError::DriverStopped)
}

#[tauri::command]
pub async fn set_arrival_reserve(
    reserve: updraft_core::ArrivalReserve,
    handle: tauri::State<'_, DriverHandle>,
) -> Result<(), DriverCommandError> {
    handle
        .send(updraft_core::SetArrivalReserve { reserve })
        .await
        .map_err(|_| DriverCommandError::DriverStopped)
}

#[tauri::command]
pub async fn set_polar(
    polar: updraft_core::PolarId,
    handle: tauri::State<'_, DriverHandle>,
) -> Result<(), DriverCommandError> {
    handle
        .send(SetPolar { polar })
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
    use crate::airspace_storage::AirspaceStorage;
    use crate::data_import::{self, DataImportState};
    use crate::driver::Driver;
    use crate::file_picker::{
        FileBytesPicker, FileBytesPickerError, FileBytesPickerFuture, FileBytesPickerState,
        PickedFileBytes,
    };
    use crate::waypoints::{commands::WaypointCommandState, storage::WaypointStorage};
    use serde_json::{Value, json};
    use std::time::Duration;
    use tauri::Manager;
    use tempfile::tempdir;
    use updraft_core::{AirspaceSource, AirspaceState, GetAirspaceSnapshot, SettingsSnapshot};

    const POLYGON: &[u8] = include_bytes!("../../testdata/airspace/polygon.txt");
    const PARSER_ERROR: &[u8] = include_bytes!("../../testdata/airspace/parser_error.txt");
    const GEOMETRY_ERROR: &[u8] = b"AC D\nAL GND\nAH FL100\nDP 50:00:00 N 010:00:00 E\nDP 50:00:00 N 010:01:00 E\nDP 50:00:00 N 010:00:00 E\n";

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
                get_polars,
                set_mac_cready,
                set_bugs,
                set_ballast,
                set_arrival_reserve,
                set_climb_average_method,
                set_polar,
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

    fn driver(airspace: AirspaceState) -> DriverHandle {
        Driver::spawn(
            SettingsSnapshot::default(),
            airspace,
            Box::new(|_, _, _| Box::new(|| {})),
            Box::new(|_| {}),
            Duration::from_millis(100),
        )
    }

    struct TestFileBytesPicker {
        result: std::sync::Mutex<Option<Result<Option<PickedFileBytes>, FileBytesPickerError>>>,
    }

    impl TestFileBytesPicker {
        fn new(result: Result<Option<PickedFileBytes>, FileBytesPickerError>) -> Self {
            Self {
                result: std::sync::Mutex::new(Some(result)),
            }
        }
    }

    impl FileBytesPicker for TestFileBytesPicker {
        fn pick_file_bytes(&self) -> FileBytesPickerFuture {
            let result = self
                .result
                .lock()
                .expect("the test file picker lock should be available")
                .take()
                .expect("the test file picker should only run once");
            Box::pin(async move { result })
        }
    }

    fn command_state(storage: AirspaceStorage) -> AirspaceCommandState {
        AirspaceCommandState::new(storage)
    }

    fn airspace_app(
        state: AirspaceCommandState,
        handle: DriverHandle,
        picker_result: Result<Option<PickedFileBytes>, FileBytesPickerError>,
    ) -> tauri::App<tauri::test::MockRuntime> {
        let picker: FileBytesPickerState = Box::new(TestFileBytesPicker::new(picker_result));
        let directory = tempdir().unwrap();
        let waypoints = WaypointCommandState::new(WaypointStorage::new(directory.path().into()));
        tauri::test::mock_builder()
            .manage(state)
            .manage(DataImportState::default())
            .manage(waypoints)
            .manage(directory)
            .manage(handle)
            .manage(picker)
            .invoke_handler(tauri::generate_handler![
                data_import::select_data_file,
                data_import::import_data_file,
                remove_airspace,
                set_airspace_enabled
            ])
            .build(tauri::test::mock_context(tauri::test::noop_assets()))
            .expect("the airspace IPC test app should build")
    }

    fn invoke_airspace(
        app: &tauri::App<tauri::test::MockRuntime>,
        command: &str,
    ) -> Result<Value, Value> {
        invoke_airspace_args(app, command, json!({"sourceName": "Local airspace.txt"}))
    }

    fn invoke_airspace_args(
        app: &tauri::App<tauri::test::MockRuntime>,
        command: &str,
        args: Value,
    ) -> Result<Value, Value> {
        let args = if command == "import_data_file" {
            let selected = invoke_airspace_args(app, "select_data_file", json!({}))?;
            json!({"selectionId": selected["selectionId"]})
        } else {
            args
        };
        let webview = app.get_webview_window("main").unwrap_or_else(|| {
            tauri::WebviewWindowBuilder::new(app, "main", Default::default())
                .build()
                .expect("the airspace IPC test webview should build")
        });
        tauri::test::get_ipc_response(&webview, request(command, args)).map(|response| {
            response
                .deserialize::<Value>()
                .expect("the airspace command response should deserialize")
        })
    }

    fn selected_file(display_name: &str, bytes: &[u8]) -> Option<PickedFileBytes> {
        Some(PickedFileBytes {
            display_name: Some(display_name.to_owned()),
            bytes: bytes.to_vec(),
        })
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
    async fn climb_average_method_command_accepts_only_known_methods() {
        let app = app();
        let webview = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
            .build()
            .expect("the IPC test webview should build");
        for method in ["normalizedEma", "average20s", "average30s"] {
            let input = request("set_climb_average_method", json!({ "method": method }));
            claims::assert_ok!(tauri::test::get_ipc_response(&webview, input));
        }
        let input = request("set_climb_average_method", json!({ "method": "unknown" }));
        claims::assert_err!(tauri::test::get_ipc_response(&webview, input));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn arrival_reserve_command_accepts_nonnegative_meters() {
        let app = app();
        let webview = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
            .build()
            .expect("the IPC test webview should build");
        for reserve in [0.0, 200.0, 304.8] {
            let input = request("set_arrival_reserve", json!({ "reserve": reserve }));
            claims::assert_ok!(tauri::test::get_ipc_response(&webview, input));
        }
        for reserve in [json!(-1), json!(null), json!("200")] {
            let input = request("set_arrival_reserve", json!({ "reserve": reserve }));
            claims::assert_err!(tauri::test::get_ipc_response(&webview, input));
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn maccready_command_validates_meters_per_second() {
        let app = app();
        let webview = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
            .build()
            .expect("the IPC test webview should build");
        for mac_cready in [0.0, 1.5] {
            let input = request("set_mac_cready", json!({ "macCready": mac_cready }));
            claims::assert_ok!(tauri::test::get_ipc_response(&webview, input));
        }
        for mac_cready in [json!(-1), json!(null), json!("1.5")] {
            let input = request("set_mac_cready", json!({ "macCready": mac_cready }));
            claims::assert_err!(tauri::test::get_ipc_response(&webview, input));
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn bugs_command_validates_performance_loss_percent() {
        let app = app();
        let webview = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
            .build()
            .expect("the IPC test webview should build");
        for bugs in [0.0, 10.5, 99.9] {
            let input = request("set_bugs", json!({ "bugs": bugs }));
            claims::assert_ok!(tauri::test::get_ipc_response(&webview, input));
        }
        for bugs in [json!(-1), json!(100), json!(null), json!("10")] {
            let input = request("set_bugs", json!({ "bugs": bugs }));
            claims::assert_err!(tauri::test::get_ipc_response(&webview, input));
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn ballast_command_validates_litres() {
        let app = app();
        let webview = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
            .build()
            .expect("the IPC test webview should build");
        for ballast in [0.0, 100.5] {
            let input = request("set_ballast", json!({ "ballast": ballast }));
            claims::assert_ok!(tauri::test::get_ipc_response(&webview, input));
        }
        for ballast in [json!(-1), json!(null), json!("100")] {
            let input = request("set_ballast", json!({ "ballast": ballast }));
            claims::assert_err!(tauri::test::get_ipc_response(&webview, input));
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn polar_commands_validate_catalog_names() {
        let app = app();
        let webview = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
            .build()
            .expect("the IPC test webview should build");
        let catalog = tauri::test::get_ipc_response(&webview, request("get_polars", json!({})))
            .expect("polar catalog should be available")
            .deserialize::<Vec<String>>()
            .unwrap();
        assert!(catalog.contains(&"LS 8".to_owned()));
        assert!(catalog.contains(&"LS 8-18".to_owned()));
        let valid = request("set_polar", json!({ "polar": "LS 8-18" }));
        claims::assert_ok!(tauri::test::get_ipc_response(&webview, valid));
        let invalid = request("set_polar", json!({ "polar": "Unknown glider" }));
        let error = claims::assert_err!(tauri::test::get_ipc_response(&webview, invalid));
        assert!(error.as_str().unwrap().contains("Unknown polar"));
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

    #[tokio::test(flavor = "multi_thread")]
    async fn airspace_activation_persists_and_refreshes_map_resources() {
        use claims::assert_ok;
        use updraft_core::AirspaceSourceStatus;

        let directory = assert_ok!(tempdir());
        let storage = AirspaceStorage::new(directory.path());
        let name = "Local airspace.txt";
        assert_ok!(assert_ok!(storage.import_airspace(POLYGON, name)));
        assert_ok!(assert_ok!(storage.import_airspace(POLYGON, "Other.txt")));
        let handle = driver(AirspaceState::at_startup(assert_ok!(storage.load())));
        for (generation, enabled) in [(1, false), (2, true)] {
            let app = airspace_app(command_state(storage.clone()), handle.clone(), Ok(None));
            let args = json!({"sourceName": name, "enabled": enabled});
            assert_eq!(
                assert_ok!(invoke_airspace_args(&app, "set_airspace_enabled", args)),
                Value::Null
            );
            let snapshot = assert_ok!(handle.send(GetAirspaceSnapshot).await);
            assert_eq!(snapshot.generation, generation);
            let status = if enabled {
                AirspaceSourceStatus::Active {
                    source_name: name.into(),
                    airspace_count: 1,
                }
            } else {
                AirspaceSourceStatus::Disabled {
                    source_name: name.into(),
                }
            };
            assert_eq!(snapshot.catalog.source_statuses()[0], status);
            assert_eq!(
                *snapshot.catalog,
                assert_ok!(AirspaceStorage::new(directory.path()).load())
            );
            std::assert_matches!(
                &snapshot.catalog.sources["Other.txt"],
                AirspaceSource::Active(_)
            );
            let response =
                crate::airspace_resource::airspace_resource_response(app.handle().clone()).await;
            let geojson: Value = assert_ok!(serde_json::from_slice(response.body()));
            let names: Vec<_> = geojson["features"]
                .as_array()
                .unwrap()
                .iter()
                .map(|feature| feature["properties"]["sourceName"].as_str().unwrap())
                .collect();
            assert_eq!(
                names,
                if enabled {
                    vec![name, "Other.txt"]
                } else {
                    vec!["Other.txt"]
                }
            );
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn airspace_activation_rejects_unknown_sources() {
        use claims::{assert_err, assert_ok};

        let directory = assert_ok!(tempdir());
        let storage = AirspaceStorage::new(directory.path());
        let handle = driver(AirspaceState::none_at_startup());
        let app = airspace_app(command_state(storage.clone()), handle.clone(), Ok(None));
        let args = json!({"sourceName": "missing.txt", "enabled": false});
        let error = assert_err!(invoke_airspace_args(&app, "set_airspace_enabled", args));
        assert_eq!(
            error,
            json!({"kind": "notFound", "sourceName": "missing.txt"})
        );
        assert_eq!(
            assert_ok!(handle.send(GetAirspaceSnapshot).await).generation,
            0
        );
        assert_eq!(assert_ok!(storage.load()), AirspaceCatalog::default());
    }

    #[cfg(unix)]
    #[tokio::test(flavor = "multi_thread")]
    #[tracing_test::traced_test]
    async fn failed_airspace_activation_save_keeps_confirmed_state() {
        use claims::{assert_err, assert_ok};
        use std::fs::{Permissions, set_permissions};
        use std::os::unix::fs::PermissionsExt;

        let directory = assert_ok!(tempdir());
        let storage = AirspaceStorage::new(directory.path());
        let name = "Local airspace.txt";
        assert_ok!(assert_ok!(storage.import_airspace(POLYGON, name)));
        for enabled in [false, true] {
            assert_ok!(storage.set_enabled(name, !enabled));
            let original = assert_ok!(storage.load());
            let handle = driver(AirspaceState::at_startup(original.clone()));
            let app = airspace_app(command_state(storage.clone()), handle.clone(), Ok(None));
            let path = directory.path().join("airspaces");
            assert_ok!(set_permissions(&path, Permissions::from_mode(0o500)));
            let args = json!({"sourceName": name, "enabled": enabled});
            let result = invoke_airspace_args(&app, "set_airspace_enabled", args);
            assert_ok!(set_permissions(&path, Permissions::from_mode(0o700)));
            assert_eq!(
                assert_err!(result),
                json!({"kind": "storageFailed", "sourceName": name})
            );
            let snapshot = assert_ok!(handle.send(GetAirspaceSnapshot).await);
            assert_eq!(snapshot.generation, 0);
            assert_eq!(*snapshot.catalog, original);
            assert_eq!(assert_ok!(storage.load()), original);
        }
        let logs = tracing_test::internal::global_buf().lock().unwrap().clone();
        assert!(String::from_utf8_lossy(&logs).contains("Could not persist airspace activation"));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn cancelled_selection_does_not_create_airspace() {
        let directory = tempdir().expect("a temporary airspace directory");
        let state = command_state(AirspaceStorage::new(directory.path()));
        let app = airspace_app(state, driver(AirspaceState::none_at_startup()), Ok(None));

        let response = invoke_airspace(&app, "select_data_file")
            .expect("picker cancellation should be a successful command");

        assert_eq!(response, Value::Null);
        assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 0);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn import_activates_the_selected_airspace() {
        let directory = tempdir().expect("a temporary airspace directory");
        let state = command_state(AirspaceStorage::new(directory.path()));
        let handle = driver(AirspaceState::none_at_startup());
        let app = airspace_app(
            state,
            handle.clone(),
            Ok(selected_file("Local airspace.txt", POLYGON)),
        );

        let response = invoke_airspace(&app, "import_data_file")
            .expect("a valid OpenAir source should import");

        insta::assert_json_snapshot!(response, @r#"
        {
          "dataType": "airspace",
          "selectionId": "0",
          "sourceName": "Local airspace.txt"
        }
        "#);
        let snapshot = handle
            .send(GetAirspaceSnapshot)
            .await
            .expect("active driver");
        let AirspaceSource::Active(dataset) = &snapshot.catalog.sources["Local airspace.txt"]
        else {
            panic!("an active source")
        };
        assert_eq!(dataset.airspaces().len(), 1);
    }

    #[tokio::test(flavor = "multi_thread")]
    #[tracing_test::traced_test]
    async fn import_serializes_safe_structured_errors() {
        let picker_directory = tempdir().expect("a temporary picker directory");
        let picker_error = (
            command_state(AirspaceStorage::new(picker_directory.path())),
            Err(FileBytesPickerError::Picker {
                source: anyhow::anyhow!("native dialog failed at /private/secret/picker"),
            }),
        );

        let read_directory = tempdir().expect("a temporary read directory");
        let read_error = (
            command_state(AirspaceStorage::new(read_directory.path())),
            Err(FileBytesPickerError::Read {
                display_name: None,
                source: anyhow::anyhow!(
                    "permission denied for content://provider/private/document/airspace-secret"
                ),
            }),
        );

        let storage_directory = tempdir().expect("a temporary storage directory");
        let invalid_directory = storage_directory.path().join("not-a-directory");
        std::fs::write(&invalid_directory, b"not a directory")
            .expect("the storage blocker should be written");
        let storage_error = (
            command_state(AirspaceStorage::new(invalid_directory)),
            Ok(selected_file("Replacement source.txt", POLYGON)),
        );

        let errors = [
            ("picker", picker_error),
            ("read", read_error),
            ("storage", storage_error),
        ]
        .into_iter()
        .map(|(name, (state, picker_result))| {
            let app = airspace_app(
                state,
                driver(AirspaceState::none_at_startup()),
                picker_result,
            );
            json!({
                "case": name,
                "error": invoke_airspace(&app, "import_data_file")
                    .expect_err("the import should fail")
            })
        })
        .collect::<Vec<_>>();

        insta::assert_json_snapshot!(errors, @r#"
        [
          {
            "case": "picker",
            "error": {
              "kind": "readFailed"
            }
          },
          {
            "case": "read",
            "error": {
              "kind": "readFailed"
            }
          },
          {
            "case": "storage",
            "error": {
              "error": {
                "kind": "storageFailed",
                "sourceName": "Replacement source.txt"
              },
              "kind": "airspace"
            }
          }
        ]
        "#);
        let serialized = serde_json::to_string(&errors).expect("serializable command errors");
        assert!(!serialized.contains("/private/"));
        assert!(!serialized.contains("content://"));
        assert!(!serialized.contains("permission denied"));
        assert!(!serialized.contains("SourceParser"));
        let logs = tracing_test::internal::global_buf().lock().unwrap().clone();
        let logs = String::from_utf8_lossy(&logs);
        assert!(logs.contains("Could not select data file"));
        assert!(logs.contains("Could not store airspace source"));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn stopped_driver_does_not_change_storage() {
        let directory = tempdir().expect("a temporary airspace directory");
        let state = command_state(AirspaceStorage::new(directory.path()));
        let app = airspace_app(
            state,
            DriverHandle::stopped(),
            Ok(selected_file("Local airspace.txt", POLYGON)),
        );

        let error = invoke_airspace(&app, "import_data_file")
            .expect_err("the stopped driver should reject activation");

        insta::assert_json_snapshot!(error, @r#"
        {
          "error": {
            "kind": "driverStopped",
            "sourceName": "Local airspace.txt"
          },
          "kind": "airspace"
        }
        "#);
        assert_eq!(
            AirspaceStorage::new(directory.path())
                .load()
                .unwrap()
                .sources
                .len(),
            0
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn concurrent_airspace_mutation_returns_busy() {
        for command in [
            "import_data_file",
            "remove_airspace",
            "set_airspace_enabled",
        ] {
            let directory = tempdir().expect("a temporary airspace directory");
            let state = command_state(AirspaceStorage::new(directory.path()));
            let mutation = state.mutation.clone();
            let _guard = mutation
                .try_lock()
                .expect("the test should own the mutation lock");
            let app = airspace_app(
                state,
                driver(AirspaceState::none_at_startup()),
                Ok(selected_file("Local airspace.txt", POLYGON)),
            );
            let args = json!({"sourceName": "Local airspace.txt", "enabled": false});
            let error = invoke_airspace_args(&app, command, args)
                .expect_err("a concurrent mutation should be rejected");
            let expected = json!({"kind":"busy"});
            let expected = if command == "import_data_file" {
                json!({"kind":"airspace", "error":expected})
            } else {
                expected
            };
            assert_eq!(error, expected);
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn remove_airspace_deletes_the_source_before_clearing_the_driver() {
        let directory = tempdir().expect("a temporary airspace directory");
        let storage = AirspaceStorage::new(directory.path());
        storage
            .import_airspace(POLYGON, "Local airspace.txt")
            .expect("the test source should be stored")
            .expect("the test source should parse");
        let initial_airspace = AirspaceState::at_startup(storage.load().unwrap());
        let state = command_state(storage);
        let handle = driver(initial_airspace);
        let app = airspace_app(state, handle.clone(), Ok(None));

        let response = invoke_airspace(&app, "remove_airspace")
            .expect("the stored airspace should be removed");

        assert_eq!(response, Value::Null);
        assert_eq!(
            handle
                .send(GetAirspaceSnapshot)
                .await
                .expect("active driver")
                .catalog
                .sources
                .len(),
            0
        );
        assert!(!directory.path().join("airspace.txt").exists());
        assert_eq!(
            AirspaceStorage::new(directory.path())
                .load()
                .unwrap()
                .sources
                .len(),
            0
        );
    }
    #[tokio::test(flavor = "multi_thread")]
    async fn airspace_import_replacement_and_removal_keep_other_sources() {
        use claims::assert_ok;
        let directory = assert_ok!(tempdir());
        let storage = AirspaceStorage::new(directory.path());
        let handle = driver(AirspaceState::default());
        let circle = include_bytes!("../../testdata/airspace/circle.txt");
        for (name, bytes) in [
            ("Local airspace.txt", POLYGON),
            ("Other.txt", POLYGON),
            ("Local airspace.txt", circle.as_slice()),
        ] {
            let app = airspace_app(
                command_state(storage.clone()),
                handle.clone(),
                Ok(selected_file(name, bytes)),
            );
            assert_ok!(invoke_airspace(&app, "import_data_file"));
        }
        let before = assert_ok!(handle.send(GetAirspaceSnapshot).await);
        assert_eq!(before.generation, 3);
        assert_eq!(before.catalog.sources.len(), 2);
        assert_eq!(assert_ok!(storage.load()), *before.catalog);
        assert_ne!(
            before.catalog.sources["Local airspace.txt"],
            before.catalog.sources["Other.txt"]
        );
        let app = airspace_app(command_state(storage.clone()), handle.clone(), Ok(None));
        assert_ok!(invoke_airspace(&app, "remove_airspace"));
        let after = assert_ok!(handle.send(GetAirspaceSnapshot).await);
        assert_eq!(after.generation, 4);
        assert_eq!(
            after.catalog.sources.keys().collect::<Vec<_>>(),
            vec!["Other.txt"]
        );
        assert_eq!(assert_ok!(storage.load()), *after.catalog);
        assert_eq!(before.catalog.sources.len(), 2);
    }

    #[tokio::test(flavor = "multi_thread")]
    #[tracing_test::traced_test]
    async fn invalid_airspace_replacement_publishes_an_unavailable_source() {
        use claims::assert_ok;
        use updraft_core::AirspaceLoadError;
        let name = "Local airspace.txt";
        for (bytes, error) in [
            (PARSER_ERROR, AirspaceLoadError::ParseFailed),
            (GEOMETRY_ERROR, AirspaceLoadError::GeometryFailed),
        ] {
            let directory = assert_ok!(tempdir());
            let storage = AirspaceStorage::new(directory.path());
            assert_ok!(assert_ok!(storage.import_airspace(POLYGON, name)));
            assert_ok!(assert_ok!(storage.import_airspace(POLYGON, "Other.txt")));
            let original = assert_ok!(storage.load());
            let handle = driver(AirspaceState::at_startup(original.clone()));
            let app = airspace_app(
                command_state(storage.clone()),
                handle.clone(),
                Ok(selected_file(name, bytes)),
            );
            assert_eq!(
                assert_ok!(invoke_airspace(&app, "import_data_file")),
                json!({"selectionId":"0", "sourceName":name, "dataType":"airspace"})
            );
            let snapshot = assert_ok!(handle.send(GetAirspaceSnapshot).await);
            assert_eq!(snapshot.generation, 1);
            assert_eq!(
                snapshot.catalog.sources[name],
                AirspaceSource::Unavailable(error)
            );
            assert_eq!(
                snapshot.catalog.sources["Other.txt"],
                original.sources["Other.txt"]
            );
            assert_eq!(assert_ok!(storage.load()), *snapshot.catalog);
        }
        let logs = tracing_test::internal::global_buf().lock().unwrap().clone();
        assert!(String::from_utf8_lossy(&logs).contains("Could not parse stored airspace source"));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn airspace_import_requires_a_display_filename() {
        use claims::{assert_err, assert_ok};
        let directory = assert_ok!(tempdir());
        let storage = AirspaceStorage::new(directory.path());
        for display_name in [None, Some(String::new())] {
            let app = airspace_app(
                command_state(storage.clone()),
                driver(AirspaceState::default()),
                Ok(Some(PickedFileBytes {
                    display_name,
                    bytes: POLYGON.to_vec(),
                })),
            );
            assert_eq!(
                assert_err!(invoke_airspace(&app, "import_data_file")),
                json!({"kind":"missingName"})
            );
        }
        assert_eq!(assert_ok!(storage.load()).sources.len(), 0);
    }
    #[tokio::test(flavor = "multi_thread")]
    #[tracing_test::traced_test]
    async fn failed_airspace_publication_keeps_stored_changes() {
        use crate::driver::tests::stop_after_next_input;
        use claims::{assert_err, assert_ok};
        use updraft_core::Core;

        let circle = include_bytes!("../../testdata/airspace/circle.txt");
        let name = "Local airspace.txt";
        for (command, installed) in [
            ("import_data_file", false),
            ("import_data_file", true),
            ("remove_airspace", true),
            ("set_airspace_enabled", true),
        ] {
            let directory = assert_ok!(tempdir());
            let storage = AirspaceStorage::new(directory.path());
            assert_ok!(assert_ok!(storage.import_airspace(POLYGON, "other.txt")));
            if installed {
                assert_ok!(assert_ok!(storage.import_airspace(POLYGON, name)));
            }
            let mut expected = assert_ok!(storage.load());
            let airspace = AirspaceState::at_startup(expected.clone());
            let core = Core::with_airspace(SettingsSnapshot::default(), airspace);
            let app = airspace_app(
                command_state(storage.clone()),
                stop_after_next_input(core),
                Ok(selected_file(name, circle)),
            );
            let args = json!({"sourceName": name, "enabled": false});
            let error = assert_err!(invoke_airspace_args(&app, command, args));
            let expected_error = json!({"kind": "driverStopped", "sourceName": name});
            let expected_error = if command == "import_data_file" {
                json!({"kind":"airspace", "error":expected_error})
            } else {
                expected_error
            };
            assert_eq!(error, expected_error);
            if command == "remove_airspace" {
                expected.sources.remove(name);
            } else if command == "set_airspace_enabled" {
                expected
                    .sources
                    .insert(name.into(), AirspaceSource::Disabled);
            } else {
                let dataset = assert_ok!(updraft_airspace::AirspaceDataset::from_openair(circle));
                expected
                    .sources
                    .insert(name.into(), AirspaceSource::Active(Arc::new(dataset)));
            }
            assert_eq!(assert_ok!(storage.load()), expected);
        }
        let logs = tracing_test::internal::global_buf().lock().unwrap().clone();
        assert!(String::from_utf8_lossy(&logs).contains("Could not activate airspace catalog"));
    }
}
