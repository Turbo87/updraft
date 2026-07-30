# Typed SPP Service UUID Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Keep each configured SPP service UUID as `uuid::Uuid` until Serde creates the Rust-to-Kotlin invocation payload.

**Architecture:** `updraft_core` parses and stores the UUID. The Tauri transport and the Rust mobile plugin keep the typed value. Serde writes the UUID as a string in `StartSppAttemptArgs`, so the Kotlin API and Android socket code do not change.

**Tech Stack:** Rust 2024, `uuid` 1.24.0, Serde 1.0.229, ts-rs 12.0.1, Tauri 2.11.5.

## Global Constraints

- Keep `serviceUuid` optional in JSON and TypeScript.
- Use the standard SPP UUID when JSON omits `serviceUuid`.
- Use `uuid!()` for UUID literals in Rust.
- Do not enable the `macro-diagnostics` feature.
- Reject an invalid UUID when `SettingsFile` deserializes the complete settings file.
- Keep `Uuid` through the core, Tauri transport, and Rust mobile plugin.
- Convert `Uuid` to a string only when Serde creates the mobile invocation payload.
- Do not change Kotlin code or the mobile invocation JSON shape.
- Keep the existing SPP retry, cancellation, event, and socket behavior.
- Do not add a configuration user interface.
- Do not add Bluetooth discovery, pairing, BLE, or insecure RFCOMM.
- Do not repeat physical SPP acceptance. The invocation payload and Kotlin behavior do not change.

---

### Task 1: Use `Uuid` through the Rust mobile boundary

**Files:**

- Modify: `Cargo.lock`
- Modify: `libs/updraft_core/Cargo.toml`
- Modify: `libs/updraft_core/src/connection.rs`
- Modify: `tauri/Cargo.toml`
- Modify: `tauri/src/settings.rs`
- Modify: `tauri/src/transport/spp.rs`
- Modify: `libs/tauri_plugin_updraft/Cargo.toml`
- Modify: `libs/tauri_plugin_updraft/src/desktop.rs`
- Modify: `libs/tauri_plugin_updraft/src/mobile.rs`
- Regenerate and verify: `frontend/src/lib/protocol/generated/ConnectionSpec.ts`
- Regenerate and verify: `frontend/src/lib/protocol/generated/PublishedExternalDevice.ts`
- Modify: `libs/updraft_core/tests/snapshots/scenario__add_external_device_after_loaded_devices_uses_a_fresh_id.snap`
- Modify: `libs/updraft_core/tests/snapshots/scenario__edit_external_device_tcp_to_bluetooth.snap`
- Modify: `libs/updraft_core/tests/snapshots/scenario__reorder_external_devices_publishes_and_persists_the_new_order.snap`

**Interfaces:**

- Produces: `pub const STANDARD_SPP_SERVICE_UUID: Uuid`.
- Produces: `ConnectionSpec::BluetoothSpp { address: String, service_uuid: Uuid }`.
- Preserves: `ConnectionSpec::bluetooth_spp(address)` with the standard UUID.
- Produces: `SppPlatform::start_attempt(&self, address: &str, service_uuid: Uuid, events: Channel)`.
- Produces: `UpdraftMobile::start_spp_attempt(&self, address: &str, service_uuid: Uuid, events: Channel)`.
- Preserves: TypeScript `serviceUuid?: string`.
- Preserves: Kotlin `serviceUuid: String`.

- [ ] **Step 1: Add the failing settings test**

Add this test to `tauri/src/settings.rs`:

```rust
#[test]
#[traced_test]
fn bluetooth_with_invalid_service_uuid_warns_and_loads_defaults() {
    let directory = assert_ok!(tempdir());
    assert_ok!(std::fs::write(
        directory.path().join("settings.json"),
        concat!(
            "{\"externalDevices\":[{",
            "\"enabled\":true,",
            "\"type\":\"bluetooth\",",
            "\"address\":\"00:11:22:33:44:55\",",
            "\"serviceUuid\":\"invalid\"",
            "}]}"
        ),
    ));
    let file = SettingsFile::new(directory.path());

    assert_eq!(file.load(), SettingsSnapshot::default());
    assert!(logs_contain("Could not load settings"));
}
```

This test catches a change from `Uuid` back to `String`.

- [ ] **Step 2: Run the test and confirm the expected failure**

Run:

```bash
cargo test -p updraft_tauri \
  settings::tests::bluetooth_with_invalid_service_uuid_warns_and_loads_defaults \
  -- --exact
```

Expected: the equality assertion fails because the current `String` field accepts `"invalid"`.

- [ ] **Step 3: Add the direct UUID dependencies**

Add this dependency to `libs/updraft_core/Cargo.toml`:

```toml
uuid = { version = "=1.24.0", features = ["serde"] }
```

Enable the ts-rs UUID implementation on the existing dependency:

```toml
ts-rs = { version = "=12.0.1", features = ["uuid-impl"], optional = true }
```

Add this dependency to `tauri/Cargo.toml`:

```toml
uuid = "=1.24.0"
```

Add this dependency to `libs/tauri_plugin_updraft/Cargo.toml`:

```toml
uuid = { version = "=1.24.0", features = ["serde"] }
```

Run:

```bash
cargo check -p updraft_core -p updraft_tauri -p tauri-plugin-updraft
```

Expected: compilation still succeeds. `Cargo.lock` adds `uuid` to the three workspace package dependency lists. The locked `uuid` package stays at 1.24.0.

- [ ] **Step 4: Store a typed UUID in `ConnectionSpec`**

Change `libs/updraft_core/src/connection.rs` to use this constant and field:

```rust
use serde::{Deserialize, Serialize};
use uuid::{Uuid, uuid};

pub const STANDARD_SPP_SERVICE_UUID: Uuid =
    uuid!("00001101-0000-1000-8000-00805F9B34FB");

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
        service_uuid: Uuid,
    },
}

fn default_spp_service_uuid() -> Uuid {
    STANDARD_SPP_SERVICE_UUID
}

fn is_standard_spp_service_uuid(service_uuid: &Uuid) -> bool {
    *service_uuid == STANDARD_SPP_SERVICE_UUID
}

impl ConnectionSpec {
    pub fn tcp(host: impl Into<String>, port: u16) -> Self {
        Self::Tcp {
            host: host.into(),
            port,
        }
    }

    pub fn bluetooth_spp(address: impl Into<String>) -> Self {
        Self::BluetoothSpp {
            address: address.into(),
            service_uuid: default_spp_service_uuid(),
        }
    }
}
```

- [ ] **Step 5: Update the settings fixtures**

Import the macro in the `tauri/src/settings.rs` test module:

```rust
use uuid::uuid;
```

Use the typed standard constant without `to_owned()`:

```rust
service_uuid: STANDARD_SPP_SERVICE_UUID,
```

Use the macro for the custom UUID:

```rust
service_uuid: uuid!("e56617bf-f548-4f7c-9cef-4a26eec19b04"),
```

Keep the expected JSON strings unchanged. Serde must write the custom UUID as a lowercase, hyphenated string.

- [ ] **Step 6: Carry `Uuid` through the Tauri SPP transport**

Import the type in `tauri/src/transport/spp.rs`:

```rust
use uuid::Uuid;
```

Change each SPP transport UUID parameter:

```rust
trait SppPlatform: Send + Sync + 'static {
    fn start_attempt(
        &self,
        address: &str,
        service_uuid: Uuid,
        events: Channel,
    ) -> Result<(), String>;
}

impl<R: Runtime> SppPlatform for AndroidSppPlatform<R> {
    fn start_attempt(
        &self,
        address: &str,
        service_uuid: Uuid,
        events: Channel,
    ) -> Result<(), String> {
        self.0
            .updraft_mobile()
            .start_spp_attempt(address, service_uuid, events)
            .map_err(|error| error.to_string())
    }

    fn cancel_attempt(&self) -> Result<(), String> {
        self.0
            .updraft_mobile()
            .cancel_spp_attempt()
            .map_err(|error| error.to_string())
    }
}

pub fn run<R: Runtime>(
    device_id: ExternalDeviceId,
    address: String,
    service_uuid: Uuid,
    handle: DriverHandle,
    app: AppHandle<R>,
) -> StopFn;

async fn run_attempt(
    device_id: ExternalDeviceId,
    address: &str,
    service_uuid: Uuid,
    handle: &DriverHandle,
    platform: &dyn SppPlatform,
    events: &Channel,
    receiver: &mut mpsc::UnboundedReceiver<InvokeResponseBody>,
    mut stop_receiver: Pin<&mut oneshot::Receiver<()>>,
) -> AttemptResult;

async fn maintain(
    device_id: ExternalDeviceId,
    address: String,
    service_uuid: Uuid,
    handle: DriverHandle,
    platform: Arc<dyn SppPlatform>,
    stop_receiver: oneshot::Receiver<()>,
);

async fn maintain_on_channel(
    device_id: ExternalDeviceId,
    address: String,
    service_uuid: Uuid,
    handle: DriverHandle,
    platform: Arc<dyn SppPlatform>,
    events: Channel,
    mut receiver: mpsc::UnboundedReceiver<InvokeResponseBody>,
    mut stop_receiver: Pin<&mut oneshot::Receiver<()>>,
);

fn spawn_maintained(
    device_id: ExternalDeviceId,
    address: String,
    service_uuid: Uuid,
    handle: DriverHandle,
    platform: Arc<dyn SppPlatform>,
) -> Maintained;
```

Keep each existing function attribute and body.
Pass `service_uuid` from `maintain_on_channel()` to `run_attempt()`.

Change the fake platform storage:

```rust
service_uuids: Mutex<Vec<Uuid>>,

fn service_uuids(&self) -> Vec<Uuid> {
    self.service_uuids
        .lock()
        .expect("service UUIDs lock")
        .clone()
}
```

Store the copied UUID in `FakePlatform::start_attempt()`:

```rust
self.service_uuids
    .lock()
    .expect("service UUIDs lock")
    .push(service_uuid);
```

Import and define the custom UUID in the test module:

```rust
use uuid::{Uuid, uuid};

const CUSTOM_UUID: Uuid = uuid!("e56617bf-f548-4f7c-9cef-4a26eec19b04");
```

Remove local custom UUID string constants.
Pass `CUSTOM_UUID` and `STANDARD_SPP_SERVICE_UUID` to `run_attempt()`.
Pass copied UUID constants to `maintain()` and `spawn_maintained()`.
Remove each `to_owned()` call on a UUID constant.

- [ ] **Step 7: Keep `Uuid` in the Rust mobile plugin**

Import `Uuid` in `libs/tauri_plugin_updraft/src/mobile.rs`:

```rust
use uuid::Uuid;
```

Change the invocation argument and method:

```rust
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct StartSppAttemptArgs<'a> {
    address: &'a str,
    service_uuid: Uuid,
    events: Channel,
}

pub fn start_spp_attempt(
    &self,
    address: &str,
    service_uuid: Uuid,
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

Import `Uuid` in `libs/tauri_plugin_updraft/src/desktop.rs`.
Change the desktop stub to accept `_service_uuid: Uuid`.

Do not change the Kotlin `StartSppAttemptArgs`.
Serde keeps the `serviceUuid` payload field as a string.

- [ ] **Step 8: Regenerate TypeScript and update snapshots**

Run:

```bash
cargo run -p updraft_core --features ts --example generate_protocol_bindings
cargo test -p updraft_core --all-features
```

Expected: the TypeScript bindings still use `serviceUuid?: string`.
The scenario snapshots fail because `Uuid` debug output removes the string quotes and uses lowercase.

Inspect the generated files and each `.snap.new` file:

```bash
git diff -- frontend/src/lib/protocol/generated/ConnectionSpec.ts
git diff -- frontend/src/lib/protocol/generated/PublishedExternalDevice.ts
diff -u \
  libs/updraft_core/tests/snapshots/scenario__add_external_device_after_loaded_devices_uses_a_fresh_id.snap \
  libs/updraft_core/tests/snapshots/scenario__add_external_device_after_loaded_devices_uses_a_fresh_id.snap.new
diff -u \
  libs/updraft_core/tests/snapshots/scenario__edit_external_device_tcp_to_bluetooth.snap \
  libs/updraft_core/tests/snapshots/scenario__edit_external_device_tcp_to_bluetooth.snap.new
diff -u \
  libs/updraft_core/tests/snapshots/scenario__reorder_external_devices_publishes_and_persists_the_new_order.snap \
  libs/updraft_core/tests/snapshots/scenario__reorder_external_devices_publishes_and_persists_the_new_order.snap.new
```

Accept only the typed UUID body changes:

```bash
cargo insta accept
git diff -- libs/updraft_core/tests/snapshots
```

Remove new `assertion_line` metadata before the green test.
Existing scenario snapshots do not store this generated metadata.

Run:

```bash
cargo test -p updraft_core --all-features
```

Expected: all core tests pass.

- [ ] **Step 9: Run the focused green tests**

Run:

```bash
cargo test -p updraft_tauri \
  settings::tests::bluetooth_with_invalid_service_uuid_warns_and_loads_defaults \
  -- --exact
cargo test -p updraft_tauri settings::
cargo test -p updraft_tauri transport::spp::
cargo test -p tauri-plugin-updraft --all-features
cargo test -p updraft_core --features ts \
  bindings::tests::committed_bindings_are_up_to_date
```

Expected: all commands pass without warnings or unexpected log output.

- [ ] **Step 10: Run affected CI validation**

Run:

```bash
cargo fmt --all --check
cargo clippy --workspace \
  --exclude updraft_tauri \
  --exclude tauri-plugin-updraft \
  --all-targets \
  --all-features \
  -- -D warnings
cargo test --workspace \
  --exclude updraft_tauri \
  --exclude tauri-plugin-updraft \
  --all-features
cargo clippy -p updraft_tauri --all-targets --all-features -- -D warnings
cargo test -p updraft_tauri --all-features
cargo clippy -p tauri-plugin-updraft --all-targets --all-features -- -D warnings
cargo test -p tauri-plugin-updraft --all-features
cargo doc -p updraft_core -p updraft_tauri -p tauri-plugin-updraft \
  --no-deps \
  --all-features
git diff --check
```

If the Android Rust target and NDK are available, also run:

```bash
cargo clippy -p updraft_tauri \
  --target aarch64-linux-android \
  --all-targets \
  --all-features \
  -- -D warnings
cargo clippy -p tauri-plugin-updraft \
  --target aarch64-linux-android \
  --all-targets \
  --all-features \
  -- -D warnings
```

Report an unavailable Android target or NDK as an environment limit.
Do not change Kotlin code to work around a Rust toolchain problem.

- [ ] **Step 11: Review and commit the implementation**

Confirm these conditions:

- The diff contains no raw Rust UUID string fields.
- `serviceUuid` stays optional in generated TypeScript.
- The mobile invocation JSON shape does not change.
- No Kotlin file changes.
- No unrelated file changes.

Stage only the implementation files:

```bash
git add \
  Cargo.lock \
  libs/updraft_core/Cargo.toml \
  libs/updraft_core/src/connection.rs \
  libs/updraft_core/tests/snapshots/scenario__add_external_device_after_loaded_devices_uses_a_fresh_id.snap \
  libs/updraft_core/tests/snapshots/scenario__edit_external_device_tcp_to_bluetooth.snap \
  libs/updraft_core/tests/snapshots/scenario__reorder_external_devices_publishes_and_persists_the_new_order.snap \
  libs/tauri_plugin_updraft/Cargo.toml \
  libs/tauri_plugin_updraft/src/desktop.rs \
  libs/tauri_plugin_updraft/src/mobile.rs \
  tauri/Cargo.toml \
  tauri/src/settings.rs \
  tauri/src/transport/spp.rs
```

Do not stage generated TypeScript files when regeneration produces no diff.

Commit:

```bash
git commit -S -m 'bluetooth: Use typed SPP service UUID' \
  -m 'This keeps each SPP service UUID typed in Rust. Serde converts the UUID to the existing mobile invocation string.'
```
