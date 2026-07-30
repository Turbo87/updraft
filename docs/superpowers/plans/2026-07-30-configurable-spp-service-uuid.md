# Configurable SPP Service UUID Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let an Android Bluetooth configuration select an RFCOMM service UUID while standard SPP remains the stored default.

**Architecture:** Serde converts an absent JSON field to the standard SPP UUID. Rust always stores and sends one UUID string. Kotlin parses that string before it creates the secure RFCOMM socket.

**Tech Stack:** Rust 2024, Serde 1.0.229, ts-rs 12.0.1, Tauri 2.11.5, Kotlin, Android SDK 37, JUnit 4.13.2.

## Global Constraints

- Keep the Android single-attempt owner.
- Do not add a configuration user interface.
- Do not add Bluetooth discovery, pairing, BLE, or insecure RFCOMM.
- Keep `serviceUuid` optional in JSON and TypeScript.
- Store the selected UUID as a required Rust `String`.
- Use `00001101-0000-1000-8000-00805F9B34FB` when JSON omits `serviceUuid`.
- Do not fall back after an invalid configured UUID.
- Keep the existing retry, cancellation, and event behavior.
- Do not log NMEA payload data.
- Use secure `createRfcommSocketToServiceRecord()`.
- Use Insta for core snapshots.
- Do not add `serde_json` to `updraft_core`.

---

### Task 1: Normalize the stored service UUID

**Files:**

- Modify: `libs/updraft_core/src/connection.rs`
- Modify: `tauri/src/settings.rs`
- Modify: `tauri/src/transport/mod.rs`
- Modify: `frontend/src/lib/protocol/generated/ConnectionSpec.ts`
- Modify: `libs/updraft_core/tests/snapshots/scenario__add_external_device_after_loaded_devices_uses_a_fresh_id.snap`
- Modify: `libs/updraft_core/tests/snapshots/scenario__edit_external_device_tcp_to_bluetooth.snap`
- Modify: `libs/updraft_core/tests/snapshots/scenario__reorder_external_devices_publishes_and_persists_the_new_order.snap`

**Interfaces:**

- Produces: `pub const STANDARD_SPP_SERVICE_UUID: &str`.
- Produces: `ConnectionSpec::BluetoothSpp { address: String, service_uuid: String }`.
- Preserves: `ConnectionSpec::bluetooth_spp(address)` for the standard service.
- Produces: TypeScript field `serviceUuid?: string`.

- [ ] **Step 1: Add failing settings tests**

Replace the existing test import in `tauri/src/settings.rs`:

```rust
use updraft_core::{
    ConnectionSpec, ExternalDeviceConfig, Locale, Settings, SettingsSnapshot,
    STANDARD_SPP_SERVICE_UUID,
};
```

Add this test:

```rust
#[test]
fn bluetooth_without_service_uuid_loads_the_standard_uuid() {
    let directory = assert_ok!(tempdir());
    assert_ok!(std::fs::write(
        directory.path().join("settings.json"),
        concat!(
            "{\"externalDevices\":[{",
            "\"enabled\":true,",
            "\"type\":\"bluetooth\",",
            "\"address\":\"00:11:22:33:44:55\"",
            "}]}"
        ),
    ));
    let file = SettingsFile::new(directory.path());

    assert_eq!(
        file.load(),
        SettingsSnapshot {
            settings: Settings::default(),
            external_devices: vec![ExternalDeviceConfig {
                enabled: true,
                spec: ConnectionSpec::BluetoothSpp {
                    address: "00:11:22:33:44:55".to_owned(),
                    service_uuid: STANDARD_SPP_SERVICE_UUID.to_owned(),
                },
            }],
        }
    );
}
```

Extend `writing_creates_the_directory_and_settings_file()` with this device:

```rust
ExternalDeviceConfig {
    enabled: true,
    spec: ConnectionSpec::BluetoothSpp {
        address: "00:11:22:33:44:66".to_owned(),
        service_uuid: "e56617bf-f548-4f7c-9cef-4a26eec19b04".to_owned(),
    },
},
```

Extend the expected JSON with this row:

```json
{
  "enabled": true,
  "type": "bluetooth",
  "address": "00:11:22:33:44:66",
  "serviceUuid": "e56617bf-f548-4f7c-9cef-4a26eec19b04"
}
```

- [ ] **Step 2: Run the settings tests**

Run:

```bash
cargo test -p updraft_tauri settings::
```

Expected: compilation fails because `BluetoothSpp` has no `service_uuid` field.

- [ ] **Step 3: Add the normalized Rust field**

Change `ConnectionSpec` in `libs/updraft_core/src/connection.rs`:

```rust
pub const STANDARD_SPP_SERVICE_UUID: &str =
    "00001101-0000-1000-8000-00805F9B34FB";

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "type", rename_all_fields = "camelCase")]
pub enum ConnectionSpec {
    #[serde(rename = "tcp")]
    Tcp { host: String, port: u16 },

    #[serde(rename = "bluetooth")]
    BluetoothSpp {
        address: String,

        #[serde(
            default = "default_spp_service_uuid",
            skip_serializing_if = "is_standard_spp_service_uuid"
        )]
        service_uuid: String,
    },
}

fn default_spp_service_uuid() -> String {
    STANDARD_SPP_SERVICE_UUID.to_owned()
}

fn is_standard_spp_service_uuid(service_uuid: &str) -> bool {
    service_uuid == STANDARD_SPP_SERVICE_UUID
}
```

Keep the existing constructor name:

```rust
pub fn bluetooth_spp(address: impl Into<String>) -> Self {
    Self::BluetoothSpp {
        address: address.into(),
        service_uuid: default_spp_service_uuid(),
    }
}
```

Do not add a constructor for a custom UUID.

- [ ] **Step 4: Keep transport dispatch buildable**

Change the existing Bluetooth match pattern in `tauri/src/transport/mod.rs`:

```rust
ConnectionSpec::BluetoothSpp {
    address,
    service_uuid: _,
} => {
```

Task 2 will replace this temporary pattern with explicit UUID forwarding.

- [ ] **Step 5: Regenerate TypeScript and update snapshots**

Run:

```bash
cargo run -p updraft_core --features ts --example generate_protocol_bindings
cargo test -p updraft_core
cargo insta accept
```

Inspect the three changed snapshots before acceptance.
Accept only changes that add the normalized standard UUID.

Confirm that `ConnectionSpec.ts` contains:

```typescript
export type ConnectionSpec =
  | { "type": "tcp"; host: string; port: number }
  | { "type": "bluetooth"; address: string; serviceUuid?: string };
```

- [ ] **Step 6: Run focused verification**

Run:

```bash
cargo fmt --all --check
cargo test -p updraft_core --all-features
cargo test -p updraft_tauri settings::
cargo clippy -p updraft_core -p updraft_tauri --all-targets --all-features -- -D warnings
git diff --check
```

Expected: all commands pass.

- [ ] **Step 7: Commit the normalized configuration**

Review and stage only the listed files.

```bash
git add libs/updraft_core/src/connection.rs
git add tauri/src/settings.rs
git add tauri/src/transport/mod.rs
git add frontend/src/lib/protocol/generated/ConnectionSpec.ts
git add libs/updraft_core/tests/snapshots/scenario__add_external_device_after_loaded_devices_uses_a_fresh_id.snap
git add libs/updraft_core/tests/snapshots/scenario__edit_external_device_tcp_to_bluetooth.snap
git add libs/updraft_core/tests/snapshots/scenario__reorder_external_devices_publishes_and_persists_the_new_order.snap
git commit -S -m "core: Normalize Bluetooth service UUID"
```

### Task 2: Carry the UUID through the Rust bridge

**Files:**

- Modify: `tauri/src/transport/mod.rs`
- Modify: `tauri/src/transport/spp.rs`
- Modify: `libs/tauri_plugin_updraft/src/mobile.rs`
- Modify: `libs/tauri_plugin_updraft/src/desktop.rs`

**Interfaces:**

- Consumes: `ConnectionSpec::BluetoothSpp { address, service_uuid }`.
- Produces: `start_spp_attempt(address, service_uuid, events)`.
- Produces: mobile argument `serviceUuid: String`.
- Preserves: one maintained Tauri channel for all retries.

- [ ] **Step 1: Add a failing transport test**

Add UUID capture to `FakePlatform` in `tauri/src/transport/spp.rs`:

```rust
service_uuids: Mutex<Vec<String>>,
```

Initialize it with `Mutex::new(Vec::new())`.

Add this accessor:

```rust
fn service_uuids(&self) -> Vec<String> {
    self.service_uuids
        .lock()
        .expect("service UUIDs lock")
        .clone()
}
```

Add this test:

```rust
#[tokio::test]
async fn attempt_passes_the_service_uuid_to_the_platform() {
    const CUSTOM_UUID: &str = "e56617bf-f548-4f7c-9cef-4a26eec19b04";
    let platform = FakePlatform::with_events(vec![r#"{"type":"disconnected"}"#]);
    let (events, mut receiver) = event_stream();
    let (_stop_sender, stop_receiver) = oneshot::channel();
    tokio::pin!(stop_receiver);

    run_attempt(
        DEVICE_ID,
        ADDRESS,
        CUSTOM_UUID,
        &driver(),
        &platform,
        &events,
        &mut receiver,
        stop_receiver.as_mut(),
    )
    .await;

    assert_eq!(platform.service_uuids(), vec![CUSTOM_UUID.to_owned()]);
}
```

- [ ] **Step 2: Run the transport test**

Run:

```bash
cargo test -p updraft_tauri attempt_passes_the_service_uuid_to_the_platform
```

Expected: compilation fails because the platform does not accept a UUID.

- [ ] **Step 3: Update the Rust transport signatures**

Change the platform interface:

```rust
trait SppPlatform: Send + Sync + 'static {
    fn start_attempt(
        &self,
        address: &str,
        service_uuid: &str,
        events: Channel,
    ) -> Result<(), String>;

    fn cancel_attempt(&self) -> Result<(), String>;
}
```

Change `AndroidSppPlatform::start_attempt()`:

```rust
fn start_attempt(
    &self,
    address: &str,
    service_uuid: &str,
    events: Channel,
) -> Result<(), String> {
    self.0
        .updraft_mobile()
        .start_spp_attempt(address, service_uuid, events)
        .map_err(|error| error.to_string())
}
```

Add `service_uuid` to these functions:

```rust
pub fn run<R: Runtime>(
    device_id: ExternalDeviceId,
    address: String,
    service_uuid: String,
    handle: DriverHandle,
    app: AppHandle<R>,
) -> StopFn;

async fn run_attempt(
    device_id: ExternalDeviceId,
    address: &str,
    service_uuid: &str,
    handle: &DriverHandle,
    platform: &dyn SppPlatform,
    events: &Channel,
    receiver: &mut mpsc::UnboundedReceiver<InvokeResponseBody>,
    stop_receiver: Pin<&mut oneshot::Receiver<()>>,
) -> AttemptResult;
```

Carry the owned string through `spawn_maintained()`, `maintain()`, and
`maintain_on_channel()`.

Pass the same string to every retry.
Do not add a service UUID to cancellation.

Import `STANDARD_SPP_SERVICE_UUID` in the test module.
Pass that constant to each existing test helper and function call.
Pass the custom UUID only in the new forwarding test.

Update `FakePlatform::start_attempt()`:

```rust
fn start_attempt(
    &self,
    address: &str,
    service_uuid: &str,
    events: Channel,
) -> Result<(), String> {
    assert_eq!(address, ADDRESS);
    self.service_uuids
        .lock()
        .expect("service UUIDs lock")
        .push(service_uuid.to_owned());
    self.attempts.fetch_add(1, Ordering::SeqCst);
    self.channel_ids
        .lock()
        .expect("channel IDs lock")
        .push(events.id());
    self.channels
        .lock()
        .expect("channels lock")
        .push(events.clone());

    if let Some(reason) = self.start_error {
        return Err(reason.to_owned());
    }

    for payload in &self.events {
        events
            .send(InvokeResponseBody::Json((*payload).to_owned()))
            .expect("fake event reaches the channel");
    }
    Ok(())
}
```

- [ ] **Step 4: Update transport dispatch**

Change the Bluetooth match in `tauri/src/transport/mod.rs`:

```rust
ConnectionSpec::BluetoothSpp {
    address,
    service_uuid,
} => {
    #[cfg(target_os = "android")]
    {
        spp::run(device_id, address, service_uuid, handle, app)
    }
    #[cfg(not(target_os = "android"))]
    {
        let _ = (app, service_uuid);
        tracing::warn!(?device_id, %address, "Bluetooth SPP transport is unsupported");
        handle.send(Input::connection_changed(
            device_id,
            ConnectionState::Disconnected,
        ));
        Box::new(|| {})
    }
}
```

- [ ] **Step 5: Update the mobile plugin API**

Change the Rust mobile arguments in
`libs/tauri_plugin_updraft/src/mobile.rs`:

```rust
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct StartSppAttemptArgs<'a> {
    address: &'a str,
    service_uuid: &'a str,
    events: Channel,
}
```

Change the mobile method:

```rust
pub fn start_spp_attempt(
    &self,
    address: &str,
    service_uuid: &str,
    events: Channel,
) -> crate::Result<()> {
    let args = StartSppAttemptArgs {
        address,
        service_uuid,
        events,
    };

    self.0
        .run_mobile_plugin("startSppAttempt", args)
        .map_err(Into::into)
}
```

Give the desktop method the same arguments.
Keep the desktop method as a no-op.

- [ ] **Step 6: Run focused verification**

Run:

```bash
cargo fmt --all --check
cargo test -p updraft_tauri transport::spp::
cargo test -p tauri-plugin-updraft --all-features
cargo clippy -p updraft_tauri -p tauri-plugin-updraft --all-targets --all-features -- -D warnings
git diff --check
```

Expected: all commands pass.

- [ ] **Step 7: Commit the Rust bridge**

Review and stage only the listed files.

```bash
git add tauri/src/transport/mod.rs
git add tauri/src/transport/spp.rs
git add libs/tauri_plugin_updraft/src/mobile.rs
git add libs/tauri_plugin_updraft/src/desktop.rs
git commit -S -m "bluetooth: Carry configured service UUID"
```

### Task 3: Use the UUID in the Android socket

**Files:**

- Modify: `libs/tauri_plugin_updraft/android/src/main/java/SppAttemptOwner.kt`
- Modify: `libs/tauri_plugin_updraft/android/src/main/java/SppSource.kt`
- Modify: `libs/tauri_plugin_updraft/android/src/main/java/SessionService.kt`
- Modify: `libs/tauri_plugin_updraft/android/src/main/java/UpdraftMobilePlugin.kt`
- Modify: `libs/tauri_plugin_updraft/android/src/test/java/SppAttemptOwnerTest.kt`
- Modify: `libs/tauri_plugin_updraft/android/src/test/java/SppSourceTest.kt`

**Interfaces:**

- Consumes: required plugin argument `serviceUuid: String`.
- Produces: `SppSource` reader factory input `serviceUuid: String`.
- Preserves: `SppAttemptOwner` singleton behavior.
- Preserves: secure socket creation.

- [ ] **Step 1: Add failing UUID forwarding tests**

Change the `source()` helper in `SppSourceTest.kt`:

```kotlin
private fun source(
    events: EventCollector,
    serviceUuid: String = "00001101-0000-1000-8000-00805F9B34FB",
    readerFactory: (
        serviceUuid: String,
        onConnected: () -> Unit,
        onBytes: (ByteArray) -> Unit
    ) -> SppReader
): SppSource = SppSource(
    serviceUuid,
    events.channel,
    readerFactory,
    encoder = Base64.getEncoder()::encodeToString
)
```

Add one ignored UUID argument to each existing reader factory.
For example:

```kotlin
val source = source(events) { _, connected, bytes ->
    SppReader(
        FakeSocket(ChunkedInputStream(byteArrayOf(1, 2), byteArrayOf(3))),
        connected,
        bytes
    )
}
```

Add these tests to `SppSourceTest.kt`:

```kotlin
@Test
fun `custom service UUID reaches the reader factory`() {
    val events = EventCollector()
    val customUuid = "e56617bf-f548-4f7c-9cef-4a26eec19b04"
    var receivedUuid: String? = null
    val source = source(events, customUuid) { serviceUuid, connected, bytes ->
        receivedUuid = serviceUuid
        SppReader(
            FakeSocket(ByteArrayInputStream(byteArrayOf())),
            connected,
            bytes
        )
    }

    source.run()

    assertEquals(customUuid, receivedUuid)
    assertEquals(listOf("connected", "disconnected"), events.types())
}

@Test
fun `reader creation failure emits one terminal event`() {
    val events = EventCollector()
    val source = source(events) { _, _, _ ->
        throw IllegalArgumentException("invalid service UUID")
    }

    source.run()

    assertEquals(listOf("disconnected"), events.types())
    assertTrue(events.last()["error"].asText().contains("invalid service UUID"))
}
```

- [ ] **Step 2: Run the Android forwarding tests**

Run from `tauri/gen/android`:

```bash
./gradlew :tauri-plugin-updraft:testDebugUnitTest --tests aero.updraft.mobile.SppSourceTest
```

Expected: compilation fails because the internal `SppSource` constructor does
not accept a service UUID.

- [ ] **Step 3: Carry the UUID through Kotlin**

Change `StartSppAttemptArgs`:

```kotlin
@InvokeArg
class StartSppAttemptArgs {
    lateinit var address: String
    lateinit var serviceUuid: String
    lateinit var events: Channel
}
```

Change `SppRequest`:

```kotlin
internal data class SppRequest(
    val address: String,
    val serviceUuid: String,
    val events: Channel,
    val onStarted: (Exception?) -> Unit
)
```

Pass `args.serviceUuid` when `UpdraftMobilePlugin` creates the request.

Pass `it.serviceUuid` when `SessionService` creates `SppSource`.

Add the standard UUID to the `request()` helper in `SppAttemptOwnerTest`.
Do not change the owner maps, locks, or cancellation behavior.

- [ ] **Step 4: Parse the UUID at the socket boundary**

Change the internal `SppSource` constructor and reader factory:

```kotlin
internal class SppSource(
    private val serviceUuid: String,
    private val events: Channel,
    private val readerFactory: (
        serviceUuid: String,
        onConnected: () -> Unit,
        onBytes: (ByteArray) -> Unit
    ) -> SppReader,
    private val encoder: (ByteArray) -> String
) : SppAttempt
```

Change the public constructor:

```kotlin
constructor(
    context: Context,
    address: String,
    serviceUuid: String,
    events: Channel
) : this(
    serviceUuid,
    events,
    { selectedServiceUuid, onConnected, onBytes ->
        SppReader(
            createSocket(context, address, selectedServiceUuid),
            onConnected,
            onBytes
        )
    },
    { bytes -> Base64.encodeToString(bytes, Base64.NO_WRAP) }
)
```

Pass `serviceUuid` as the first argument when `run()` calls `readerFactory`.

Change socket creation:

```kotlin
return AndroidSppSocket(
    device.createRfcommSocketToServiceRecord(
        UUID.fromString(serviceUuid)
    )
)
```

Remove the hardcoded `SPP_UUID` property.
Do not call `createInsecureRfcommSocketToServiceRecord()`.
Keep `createSocket()` inside the reader factory that `run()` calls.
The existing `try` block must convert a socket creation failure to the terminal
event.

- [ ] **Step 5: Run Android verification**

Run from `tauri/gen/android`:

```bash
./gradlew :tauri-plugin-updraft:testDebugUnitTest
```

Run from the repository root:

```bash
NDK_HOME="$ANDROID_HOME/ndk/28.1.13356709" pnpm tauri android build --target aarch64 --apk --debug
rg -n "createInsecureRfcommSocket|SPP_UUID" libs/tauri_plugin_updraft/android/src
git diff --check
```

Expected: tests and the Android build pass.
Expected: the search returns no insecure socket call or obsolete constant.

- [ ] **Step 6: Commit the Android socket change**

Review and stage only the listed files.

```bash
git add libs/tauri_plugin_updraft/android/src/main/java
git add libs/tauri_plugin_updraft/android/src/test/java
git commit -S -m "android: Use configured RFCOMM service UUID"
```

### Task 4: Run final and physical verification

**Files:**

- No repository file changes.

**Interfaces:**

- Consumes: a paired ESP32 with the standard SPP service.
- Consumes: the macOS simulator with its custom service UUID.
- Verifies: one SPP connection at a time.

- [ ] **Step 1: Run repository verification**

Run:

```bash
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test -p updraft_core --features ts bindings::tests::committed_bindings_are_up_to_date
pnpm lint
pnpm check
pnpm test
NDK_HOME="$ANDROID_HOME/ndk/28.1.13356709" pnpm tauri android build --target aarch64 --apk --debug
```

Run from `tauri/gen/android`:

```bash
./gradlew :tauri-plugin-updraft:testDebugUnitTest
```

Classify each failure.
Fix only failures caused by this change.

- [ ] **Step 2: Check standard SPP**

Stop Updraft.
Write one enabled Bluetooth row without `serviceUuid`.
Use the paired ESP32 address.
Start Updraft.

Check these results:

- Updraft reports `Connected`.
- Updraft reports first bytes.
- The settings readback does not add `serviceUuid`.
- Logs do not contain NMEA payload data.

- [ ] **Step 3: Check the custom Mac service**

Stop Updraft.
Write one enabled Bluetooth row with these fields:

```json
{
  "enabled": true,
  "type": "bluetooth",
  "address": "<MAC_BLUETOOTH_ADDRESS>",
  "serviceUuid": "e56617bf-f548-4f7c-9cef-4a26eec19b04"
}
```

Start the macOS simulator.
Start Updraft.

Check these results:

- The simulator reports channel 2.
- The simulator reports a client connection.
- Updraft reports `Connected`.
- Updraft reports first bytes.
- Logs do not contain NMEA payload data.

- [ ] **Step 4: Check the loop and reconnect**

Keep the Mac connection active for more than 301 seconds.
Check that the simulator starts batch one without a disconnect.

Force-stop Updraft.
Start Updraft again.
Check that the new connection starts with batch one.

- [ ] **Step 5: Review the complete change**

Run:

```bash
git status --short
git diff --check HEAD~3
git log --oneline -3
```

Confirm these limits:

- The diff does not change SPP attempt ownership.
- The diff does not add a configuration user interface.
- The diff does not add insecure RFCOMM.
- The diff does not log payload data.
- Both physical sources work separately.
