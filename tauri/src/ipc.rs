use crate::airspace_storage::{AirspaceSourceChange, AirspaceStorage, AirspaceStorageError};
use crate::driver::DriverHandle;
use crate::file_picker::{FileBytesPickerError, FileBytesPickerState};
use serde::Serialize;
use std::sync::Arc;
use tauri::ipc::Channel;
use tauri_plugin_updraft::{BondedBluetoothDevices, UpdraftMobileExt};
use tokio::sync::Mutex;
use updraft_airspace::AirspaceImportError;
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
}

#[derive(Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ImportAirspaceResult {
    Imported,
    Cancelled,
}

#[derive(Debug, Serialize, thiserror::Error)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum AirspaceCommandError {
    #[error("airspace picker failed")]
    PickerFailed,
    #[error("could not read selected airspace")]
    ReadFailed {
        #[serde(skip_serializing_if = "Option::is_none")]
        source_name: Option<String>,
    },
    #[error("could not parse selected airspace")]
    ParseFailed {
        #[serde(skip_serializing_if = "Option::is_none")]
        source_name: Option<String>,
    },
    #[error("could not normalize selected airspace")]
    GeometryFailed {
        #[serde(skip_serializing_if = "Option::is_none")]
        source_name: Option<String>,
    },
    #[error("could not persist selected airspace")]
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
pub async fn import_airspace(
    state: tauri::State<'_, AirspaceCommandState>,
    picker: tauri::State<'_, FileBytesPickerState>,
    handle: tauri::State<'_, DriverHandle>,
) -> Result<ImportAirspaceResult, AirspaceCommandError> {
    let _mutation = state
        .mutation
        .try_lock()
        .map_err(|_| AirspaceCommandError::Busy)?;
    let selected = picker
        .pick_file_bytes()
        .await
        .map_err(map_file_picker_error)?;
    let Some(selected) = selected else {
        return Ok(ImportAirspaceResult::Cancelled);
    };
    let source_name = selected
        .display_name
        .filter(|name| !name.is_empty())
        .ok_or(AirspaceCommandError::MissingName)?;
    let snapshot = handle.send(GetAirspaceSnapshot).await.map_err(|_| {
        AirspaceCommandError::DriverStopped {
            source_name: Some(source_name.clone()),
        }
    })?;
    let storage = state.storage.clone();
    let name = source_name.clone();
    let (dataset, change) =
        tokio::task::spawn_blocking(move || storage.import_airspace(&selected.bytes, &name))
            .await
            .map_err(|error| {
                tracing::warn!(%error, "Airspace import worker failed");
                AirspaceCommandError::WorkerFailed
            })?
            .map_err(|error| map_airspace_storage_error(error, Some(source_name.clone())))?;
    let mut catalog = (*snapshot.catalog).clone();
    catalog.sources.insert(source_name.clone(), Ok(dataset));
    activate_airspace_catalog(&handle, catalog, change, source_name).await?;

    Ok(ImportAirspaceResult::Imported)
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
    let change = tokio::task::spawn_blocking(move || storage.remove(&name))
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
    activate_airspace_catalog(&handle, catalog, change, source_name).await
}

async fn activate_airspace_catalog(
    handle: &DriverHandle,
    catalog: AirspaceCatalog,
    change: AirspaceSourceChange,
    source_name: String,
) -> Result<(), AirspaceCommandError> {
    if let Err(error) = handle.send(ReplaceAirspaceCatalog(Arc::new(catalog))).await {
        tracing::warn!(%error, "Could not activate airspace catalog");
        tokio::task::spawn_blocking(move || change.rollback()).await.map_err(|error| {
            tracing::error!(%error, "Airspace rollback worker failed");
            AirspaceCommandError::WorkerFailed
        })?.map_err(|error| {
            tracing::error!(%error, "Could not restore airspace source after activation failed");
            AirspaceCommandError::StorageFailed { source_name: Some(source_name.clone()) }
        })?;
        return Err(AirspaceCommandError::DriverStopped {
            source_name: Some(source_name),
        });
    }
    Ok(())
}

fn map_file_picker_error(error: FileBytesPickerError) -> AirspaceCommandError {
    match error {
        FileBytesPickerError::Picker { source } => {
            tracing::warn!(%source, "Could not open the file picker");
            AirspaceCommandError::PickerFailed
        }
        FileBytesPickerError::Read {
            display_name,
            source,
        } => {
            tracing::warn!(%source, "Could not read the selected file");
            AirspaceCommandError::ReadFailed {
                source_name: display_name,
            }
        }
    }
}

fn map_airspace_storage_error(
    error: AirspaceStorageError,
    source_name: Option<String>,
) -> AirspaceCommandError {
    tracing::warn!(%error, "Could not import selected airspace");
    match error {
        AirspaceStorageError::Import(AirspaceImportError::Parse { .. }) => {
            AirspaceCommandError::ParseFailed { source_name }
        }
        AirspaceStorageError::Import(AirspaceImportError::Geometry { .. }) => {
            AirspaceCommandError::GeometryFailed { source_name }
        }
        AirspaceStorageError::Io(_) => AirspaceCommandError::StorageFailed { source_name },
    }
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
    use crate::driver::Driver;
    use crate::file_picker::{FileBytesPicker, FileBytesPickerFuture, PickedFileBytes};
    use serde_json::{Value, json};
    use std::time::Duration;
    use tempfile::tempdir;
    use updraft_core::{AirspaceState, GetAirspaceSnapshot, SettingsSnapshot};

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
        tauri::test::mock_builder()
            .manage(state)
            .manage(handle)
            .manage(picker)
            .invoke_handler(tauri::generate_handler![import_airspace, remove_airspace])
            .build(tauri::test::mock_context(tauri::test::noop_assets()))
            .expect("the airspace IPC test app should build")
    }

    fn invoke_airspace(
        app: &tauri::App<tauri::test::MockRuntime>,
        command: &str,
    ) -> Result<Value, Value> {
        let webview = tauri::WebviewWindowBuilder::new(app, "main", Default::default())
            .build()
            .expect("the airspace IPC test webview should build");
        tauri::test::get_ipc_response(
            &webview,
            request(command, json!({"sourceName": "Local airspace.txt"})),
        )
        .map(|response| {
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
    async fn import_airspace_serializes_cancelled_as_a_normal_result() {
        let directory = tempdir().expect("a temporary airspace directory");
        let state = command_state(AirspaceStorage::new(directory.path()));
        let app = airspace_app(state, driver(AirspaceState::none_at_startup()), Ok(None));

        let response = invoke_airspace(&app, "import_airspace")
            .expect("picker cancellation should be a successful command");

        insta::assert_json_snapshot!(response, @r#"
        {
          "type": "cancelled"
        }
        "#);
        assert!(!directory.path().join("airspace.txt").exists());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn import_airspace_serializes_imported_and_activates_the_dataset() {
        let directory = tempdir().expect("a temporary airspace directory");
        let state = command_state(AirspaceStorage::new(directory.path()));
        let handle = driver(AirspaceState::none_at_startup());
        let app = airspace_app(
            state,
            handle.clone(),
            Ok(selected_file("Local airspace.txt", POLYGON)),
        );

        let response =
            invoke_airspace(&app, "import_airspace").expect("a valid OpenAir source should import");

        insta::assert_json_snapshot!(response, @r#"
        {
          "type": "imported"
        }
        "#);
        let snapshot = handle
            .send(GetAirspaceSnapshot)
            .await
            .expect("active driver");
        let dataset = snapshot.catalog.sources["Local airspace.txt"]
            .as_ref()
            .unwrap();
        assert_eq!(dataset.airspaces().len(), 1);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn import_airspace_serializes_safe_structured_errors() {
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

        let parse_directory = tempdir().expect("a temporary parse directory");
        let parse_error = (
            command_state(AirspaceStorage::new(parse_directory.path())),
            Ok(selected_file("Parser source.txt", PARSER_ERROR)),
        );

        let geometry_directory = tempdir().expect("a temporary geometry directory");
        let geometry_error = (
            command_state(AirspaceStorage::new(geometry_directory.path())),
            Ok(selected_file("Geometry source.txt", GEOMETRY_ERROR)),
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
            ("parse", parse_error),
            ("geometry", geometry_error),
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
                "error": invoke_airspace(&app, "import_airspace")
                    .expect_err("the import should fail")
            })
        })
        .collect::<Vec<_>>();

        insta::assert_json_snapshot!(errors, @r#"
        [
          {
            "case": "picker",
            "error": {
              "kind": "pickerFailed"
            }
          },
          {
            "case": "read",
            "error": {
              "kind": "readFailed"
            }
          },
          {
            "case": "parse",
            "error": {
              "kind": "parseFailed",
              "sourceName": "Parser source.txt"
            }
          },
          {
            "case": "geometry",
            "error": {
              "kind": "geometryFailed",
              "sourceName": "Geometry source.txt"
            }
          },
          {
            "case": "storage",
            "error": {
              "kind": "storageFailed",
              "sourceName": "Replacement source.txt"
            }
          }
        ]
        "#);
        let serialized = serde_json::to_string(&errors).expect("serializable command errors");
        assert!(!serialized.contains("/private/"));
        assert!(!serialized.contains("content://"));
        assert!(!serialized.contains("permission denied"));
        assert!(!serialized.contains("SourceParser"));
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

        let error = invoke_airspace(&app, "import_airspace")
            .expect_err("the stopped driver should reject activation");

        insta::assert_json_snapshot!(error, @r#"
        {
          "kind": "driverStopped",
          "sourceName": "Local airspace.txt"
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
        let directory = tempdir().expect("a temporary airspace directory");
        let state = command_state(AirspaceStorage::new(directory.path()));
        let mutation = state.mutation.clone();
        let _guard = mutation
            .try_lock()
            .expect("the test should own the mutation lock");
        let app = airspace_app(state, driver(AirspaceState::none_at_startup()), Ok(None));

        let error = invoke_airspace(&app, "import_airspace")
            .expect_err("a concurrent mutation should be rejected");

        insta::assert_json_snapshot!(error, @r#"
        {
          "kind": "busy"
        }
        "#);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn remove_airspace_deletes_the_source_before_clearing_the_driver() {
        let directory = tempdir().expect("a temporary airspace directory");
        let storage = AirspaceStorage::new(directory.path());
        storage
            .import_airspace(POLYGON, "Local airspace.txt")
            .expect("the test source should import");
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
            assert_ok!(invoke_airspace(&app, "import_airspace"));
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
    async fn failed_airspace_replacement_keeps_core_and_storage() {
        use claims::{assert_err, assert_ok};
        let directory = assert_ok!(tempdir());
        let storage = AirspaceStorage::new(directory.path());
        assert_ok!(storage.import_airspace(POLYGON, "Local airspace.txt"));
        assert_ok!(storage.import_airspace(POLYGON, "Other.txt"));
        let original = assert_ok!(storage.load());
        let handle = driver(AirspaceState::at_startup(original.clone()));
        let app = airspace_app(
            command_state(storage.clone()),
            handle.clone(),
            Ok(selected_file("Local airspace.txt", PARSER_ERROR)),
        );
        assert_eq!(
            assert_err!(invoke_airspace(&app, "import_airspace")),
            json!({"kind":"parseFailed", "sourceName":"Local airspace.txt"})
        );
        let snapshot = assert_ok!(handle.send(GetAirspaceSnapshot).await);
        assert_eq!(snapshot.generation, 0);
        assert_eq!(*snapshot.catalog, original);
        assert_eq!(assert_ok!(storage.load()), original);
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
                assert_err!(invoke_airspace(&app, "import_airspace")),
                json!({"kind":"missingName"})
            );
        }
        assert_eq!(assert_ok!(storage.load()).sources.len(), 0);
    }
    #[tokio::test(flavor = "multi_thread")]
    #[tracing_test::traced_test]
    async fn failed_catalog_activation_restores_each_durable_mutation() {
        use claims::{assert_err, assert_ok};
        let directory = assert_ok!(tempdir());
        let storage = AirspaceStorage::new(directory.path());
        assert_ok!(storage.import_airspace(POLYGON, "local.txt"));
        assert_ok!(storage.import_airspace(POLYGON, "other.txt"));
        let original = assert_ok!(storage.load());
        let circle = include_bytes!("../../testdata/airspace/circle.txt");
        for (name, remove) in [
            ("local.txt", false),
            ("new.txt", false),
            ("local.txt", true),
        ] {
            let change = if remove {
                assert_ok!(storage.remove(name))
            } else {
                assert_ok!(storage.import_airspace(circle, name)).1
            };
            let replacement = assert_ok!(storage.load());
            let error = assert_err!(
                activate_airspace_catalog(
                    &DriverHandle::stopped(),
                    replacement,
                    change,
                    name.into()
                )
                .await
            );
            std::assert_matches!(error, AirspaceCommandError::DriverStopped { .. });
            assert_eq!(assert_ok!(storage.load()), original);
        }
        assert!(logs_contain("Could not activate airspace catalog"));
    }
}
