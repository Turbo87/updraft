# Typed Input Responses Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Give every core input a compile-time response type and make Tauri commands resolve only after the core has applied the input and dispatched its effects.

**Architecture:** `updraft_core` replaces its closed `Input` enum with sealed concrete input types implementing `Input<Response = ...>`. `Core::apply()` returns `Update<I::Response>`. The Tauri driver admits a private type-erased `Request<I>` carrying a matching oneshot sender, processes requests FIFO, dispatches effects, and then completes the sender. Concrete Tauri commands flatten driver and domain failures into command-specific serialized errors.

**Tech Stack:** Rust 2024, Tauri 2.11.5, Tokio 1.53.1, Serde 1.0.229, thiserror 2.0.19

## Global Constraints

- Follow red-green-refactor for every behavior change. Run each new or changed test before implementation and confirm that it fails for the expected reason.
- Keep one public `Input` trait. Do not add a parallel `Command` abstraction or a generic IPC command.
- Seal `Input` so only `updraft_core` can define inputs.
- Give each concrete input exactly one associated response type. Do not add a global response enum, `Any`, downcasts, or response tags.
- Keep one asynchronous `DriverHandle::send()` method for all core inputs, including unit-response observations.
- Keep the existing unbounded FIFO driver queue in this slice. Awaiting completion provides source-local backpressure without changing admission capacity.
- Complete `send()` only after the core applies the input and the driver dispatches every returned effect in order.
- Treat effect dispatch as adapter handoff. Do not await persistence, connection establishment, or connection closure.
- Once admitted, apply an input even if its caller is cancelled. Discarding a response because its receiver is gone is expected.
- Keep subscription separate from core input handling and preserve atomic snapshot-first delivery.
- Keep topics as the sole shared frontend state path. Do not update stores from command responses or add optimistic state.
- Treat repeated mutations as successful no-ops with no effects.
- Return expected domain rejections as typed values with no effects and no core warning.
- Preserve stale observation behavior. Bytes and connection changes for unknown or disabled devices remain ignored unit-response inputs.
- Derive `thiserror::Error` for every Rust error introduced by this change.
- Keep concrete Tauri commands. Do not expose arbitrary core inputs over IPC.
- Add no external-device settings UI, state revisions, recorder, compatibility path, or migration.
- Defer external-device `UpdraftClient` methods and fake-client behavior until frontend code needs them.
- Defer changes to `docs/design` until the implementation has validated the design.
- Do not regenerate committed TypeScript protocol bindings unless a Rust wire type used by `Topic` changes. The new host error types stay at the IPC boundary.
- Prefer existing behavioral coverage. Add a test only for a new contract at the lowest useful layer, and do not repeat domain cases in driver, IPC, and frontend tests.
- Keep every commit green and single-purpose.

---

## File Responsibilities

### Core

- Modify `libs/updraft_core/src/connection.rs`: deserialize `ExternalDeviceId` when it becomes a concrete command argument.
- Modify `libs/updraft_core/src/input.rs`: replace the enum with concrete input values, the sealed `Input` trait, and generic `Update<R>`.
- Modify `libs/updraft_core/src/core.rs`: make `Core::apply()` generic and implement each concrete input against core state.
- Modify `libs/updraft_core/src/external_device.rs`: expose typed external-device rejection values through the aggregate operations.
- Modify `libs/updraft_core/src/lib.rs`: re-export the input types, `Input`, `Update`, and domain errors.
- Modify `libs/updraft_core/Cargo.toml`: add the exact direct thiserror dependency used by domain errors.
- Modify `libs/updraft_core/tests/scenario.rs`: assert effects through `Update.effects` and pin natural responses.
- Delete `libs/updraft_core/src/snapshots/updraft_core__core__tests__invalid_external_device_mutations_warn.snap`: expected command rejection is no longer logging.

### Tauri runtime and adapters

- Modify `tauri/src/driver.rs`: add typed request envelopes, safe private erasure, asynchronous `send()`, automatic startup, and focused completion and cancellation tests.
- Modify `tauri/src/transport/tcp.rs`: await every reported observation and stop the producer if the driver stops.
- Modify `tauri/src/transport/spp.rs`: await every reported observation and stop the producer if the driver stops.
- Modify `tauri/src/session.rs`: bridge synchronous GNSS callbacks through one ordered channel to one asynchronous sender task.
- Modify `tauri/src/ipc.rs`: make locale and external-device commands asynchronous and serialize only their possible failures.
- Modify `tauri/src/lib.rs`: register all concrete commands and let the driver process `Start` internally.
- Modify `tauri/Cargo.toml`: add the exact direct Serde and thiserror dependencies needed by command errors.

---

### Task 1: Replace the input enum and await driver processing

**Files:**

- Modify: `libs/updraft_core/src/input.rs`
- Modify: `libs/updraft_core/src/core.rs`
- Modify: `libs/updraft_core/src/lib.rs`
- Modify: `libs/updraft_core/tests/scenario.rs`
- Modify: `tauri/src/driver.rs`
- Modify: `tauri/src/ipc.rs`
- Modify: `tauri/src/lib.rs`
- Modify: `tauri/src/session.rs`
- Modify: `tauri/src/transport/mod.rs`
- Modify: `tauri/src/transport/tcp.rs`
- Modify: `tauri/src/transport/spp.rs`
- Modify: `tauri/Cargo.toml`

**Interfaces:**

- Produces: `Input::Response`
- Produces: `Update<R> { effects, response }`
- Produces: `Core::apply<I: Input>() -> Update<I::Response>`
- Produces: `DriverHandle::send<I: Input>() -> Result<I::Response, DriverStopped>`
- Preserves: every input's current effects
- Preserves temporarily: `Response = ()` for every concrete input
- Preserves: `DriverHandle::subscribe()` as a separate synchronous operation

- [ ] **Step 1: Establish the Rust baseline**

Run:

```bash
cargo test -p updraft_core
cargo test -p updraft_tauri
```

Expected: both commands pass before the refactor.

- [ ] **Step 2: Define the sealed input interface and concrete values**

Replace the enum in `libs/updraft_core/src/input.rs` with this interface and concrete domain-shaped structs:

```rust
mod private {
    pub trait Sealed {}
}

pub trait Input: private::Sealed + Send + 'static {
    type Response: Send + 'static;

    #[doc(hidden)]
    fn apply_to(self, core: &mut Core, at: Timestamp) -> Update<Self::Response>;
}

#[derive(Clone, Debug, PartialEq)]
pub struct Update<R> {
    pub effects: Vec<Effect>,
    pub response: R,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Start;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Tick;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Bytes {
    pub device_id: ExternalDeviceId,
    pub data: Vec<u8>,
}

impl Bytes {
    pub fn new(device_id: ExternalDeviceId, data: impl Into<Vec<u8>>) -> Self {
        Self {
            device_id,
            data: data.into(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ConnectionChanged {
    pub device_id: ExternalDeviceId,
    pub state: ConnectionState,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct InternalGps {
    pub fix: Fix,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SetLocale {
    pub locale: Locale,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AddExternalDevice {
    pub spec: ConnectionSpec,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DeleteExternalDevice {
    pub device_id: ExternalDeviceId,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReorderExternalDevices {
    pub order: Vec<ExternalDeviceId>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EditExternalDevice {
    pub device_id: ExternalDeviceId,
    pub spec: ConnectionSpec,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SetExternalDeviceEnabled {
    pub device_id: ExternalDeviceId,
    pub enabled: bool,
}
```

Implement `private::Sealed` for every concrete type inside `input.rs`. Re-export every type plus `Input` and `Update` from `libs/updraft_core/src/lib.rs`.

In `libs/updraft_core/src/core.rs`, make the public entry point generic:

```rust
pub fn apply<I: Input>(&mut self, input: I, at: Timestamp) -> Update<I::Response> {
    input.apply_to(self, at)
}
```

Implement `Input<Response = ()>` for every type in `core.rs`. Move each existing match arm into its corresponding implementation without changing its effects yet:

```rust
impl Input for SetLocale {
    type Response = ();

    fn apply_to(self, core: &mut Core, _at: Timestamp) -> Update<Self::Response> {
        let effects = if core.settings.locale == Some(self.locale) {
            Vec::new()
        } else {
            core.settings.locale = Some(self.locale);
            vec![
                Effect::emit(Topic::Settings(core.settings)),
                Effect::persist_settings(core.settings_snapshot()),
            ]
        };
        Update {
            effects,
            response: (),
        }
    }
}
```

Keep helper methods such as decoding and settings snapshot creation on `Core`. Do not give input implementations direct access to I/O.

- [ ] **Step 3: Migrate core tests without changing behavior**

Update `libs/updraft_core/src/core.rs` tests and `libs/updraft_core/tests/scenario.rs` to construct the concrete types. Where a test only cares about effects, destructure or read `Update.effects`:

```rust
let effects = core
    .apply(Bytes::new(device_id, RMC), at(100))
    .effects;
```

Keep all existing snapshots and effect assertions unchanged. This is a
behavior-preserving refactor, so the existing tests are the durable coverage.
Do not add a test that merely restates associated types already enforced by
the compiler. The natural device responses remain unit values until Task 2.

Run:

```bash
cargo test -p updraft_core
```

Expected: all core tests pass.

- [ ] **Step 4: Pin the driver completion contract**

In `tauri/src/driver.rs`, change only the existing tests that naturally own
the new behavior:

- Remove the explicit `Start` send from
  `start_asks_for_a_transport_per_configured_connection`. The test should now
  time out until the driver applies `Start` internally.
- Await `SetLocale` in
  `locale_changes_reach_subscribers_and_persistence`, assert the send returns
  `()`, then call `try_recv()` on the existing topic and persistence receivers
  to prove both effects were already dispatched before the response.

Add two focused tests for contracts not covered elsewhere:

- `admitted_input_survives_a_dropped_response_receiver` runs one typed request
  after dropping its oneshot receiver and observes its settings effect.
- `send_to_a_stopped_driver_fails` awaits a send through the existing
  inactive-handle helper and expects `DriverStopped`.

Do not add separate FIFO, unit-response, or post-admission shutdown tests.
The single driver loop and Tokio channel already provide FIFO. The locale test
covers unit responses, and the oneshot maps a dropped queued request to the
same `DriverStopped` result without another test-only hook.

Run:

```bash
cargo test -p updraft_tauri driver::tests
```

Expected: compilation fails because `send()` is synchronous and has no
response. If compiled separately, the startup test times out because the
driver still depends on an externally sent `Start`.

- [ ] **Step 5: Add the typed request envelope and private erasure**

Add exact thiserror to `tauri/Cargo.toml`:

```toml
thiserror = "=2.0.19"
```

In `tauri/src/driver.rs`, replace `Message::Input(Input)` with a private typed envelope:

```rust
struct Request<I: Input> {
    input: I,
    reply: oneshot::Sender<I::Response>,
}

trait ErasedInput: Send {
    fn run(self: Box<Self>, driver: &mut DriverState, at: Timestamp);
}

enum Message {
    Input(Box<dyn ErasedInput>),
    Subscribe(Sink),
}

#[derive(Clone, Copy, Debug, thiserror::Error, PartialEq, Eq)]
#[error("driver stopped")]
pub struct DriverStopped;
```

Move the mutable loop-owned values into `DriverState`:

```rust
struct DriverState {
    core: Core,
    sinks: Vec<Sink>,
    transports: ActiveTransports,
    open: OpenFn,
    persist: PersistFn,
    handle: DriverHandle,
}

impl DriverState {
    fn apply<I: Input>(&mut self, input: I, at: Timestamp) -> I::Response {
        let Update { effects, response } = self.core.apply(input, at);
        for effect in effects {
            self.dispatch(effect);
        }
        response
    }
}
```

Implement erasure only for already paired typed requests:

```rust
impl<I: Input> ErasedInput for Request<I> {
    fn run(self: Box<Self>, driver: &mut DriverState, at: Timestamp) {
        let Request { input, reply } = *self;
        let response = driver.apply(input, at);
        let _ = reply.send(response);
    }
}
```

Implement the one public send operation:

```rust
pub async fn send<I: Input>(&self, input: I) -> Result<I::Response, DriverStopped> {
    let (reply, response) = oneshot::channel();
    self.messages
        .send(Message::Input(Box::new(Request { input, reply })))
        .map_err(|_| DriverStopped)?;
    response.await.map_err(|_| DriverStopped)
}
```

Process `Start` through `DriverState::apply()` before entering the external message loop. Process ticker wakeups through `DriverState::apply(Tick, at)` rather than constructing a self-addressed message. Run an erased request only after computing its monotonic timestamp.

Run:

```bash
cargo test -p updraft_tauri driver::tests
```

Expected: the focused driver tests pass.

- [ ] **Step 6: Adapt every producer to awaited completion**

Add exact Serde to `tauri/Cargo.toml`:

```toml
serde = { version = "=1.0.229", features = ["derive"] }
```

Make `tauri/src/ipc.rs::set_locale()` asynchronous and await `SetLocale`. Map `DriverStopped` to the first module-contained public command error:

```rust
#[derive(Debug, Serialize, thiserror::Error)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum DriverCommandError {
    #[error("driver stopped")]
    DriverStopped,
}
```

Return `Result<(), DriverCommandError>` instead of converting driver shutdown to a string. Task 3 reuses this error for addition and adds the narrower domain-aware errors.

Remove the explicit startup send from `tauri/src/lib.rs`.

In `tauri/src/transport/tcp.rs`, await each `ConnectionChanged` and `Bytes` submission. If any send returns `DriverStopped`, stop that transport task without retrying or logging a transport failure.

In `tauri/src/transport/spp.rs`, await each `ConnectionChanged` and `Bytes` submission inside the existing ordered event loop. Represent `DriverStopped` separately from connection failure so the maintain loop exits without reconnecting or logging an SPP failure.

In `tauri/src/session.rs`, keep deserialization in the synchronous `Channel` callback but forward valid `Fix` values through one `mpsc::UnboundedSender`. Spawn one async adapter task with `tauri::async_runtime::spawn`:

```rust
let (sender, mut receiver) = mpsc::unbounded_channel();
tauri::async_runtime::spawn(async move {
    while let Some(fix) = receiver.recv().await {
        if handle.send(InternalGps { fix }).await.is_err() {
            break;
        }
    }
});
```

The callback sends values into this channel in callback order. Do not spawn one task per fix.

Update producer tests to await the driver path and retain their existing ordering and diagnostics assertions.

Add a GNSS regression test that reports two fixes back-to-back through one `Channel` and observes them in the same order. Keep the existing TCP and SPP byte-path tests as coverage that their awaited sends still reach the core.

Run:

```bash
cargo test -p updraft_tauri
```

Expected: all shell, TCP, SPP, and GNSS tests pass.

- [ ] **Step 7: Verify and commit the typed unit-response path**

Run:

```bash
cargo fmt --all --check
cargo test -p updraft_core
cargo test -p updraft_tauri
cargo clippy -p updraft_core -p updraft_tauri --all-targets --all-features -- -D warnings
git diff --check
```

Expected: all commands pass.

Review that no old enum construction or unawaited core send remains:

```bash
rg -n "Input::|handle\\.send\\(" libs/updraft_core tauri
```

Expected: no `Input::Variant` construction remains. Every production `handle.send()` call is awaited.

Commit:

```bash
git add libs/updraft_core/src/input.rs libs/updraft_core/src/core.rs libs/updraft_core/src/lib.rs libs/updraft_core/tests/scenario.rs tauri/Cargo.toml tauri/src/driver.rs tauri/src/ipc.rs tauri/src/lib.rs tauri/src/session.rs tauri/src/transport/mod.rs tauri/src/transport/tcp.rs tauri/src/transport/spp.rs
git commit -m 'runtime: Await typed core inputs'
```

---

### Task 2: Return natural external-device results

**Files:**

- Modify: `libs/updraft_core/Cargo.toml`
- Modify: `libs/updraft_core/src/external_device.rs`
- Modify: `libs/updraft_core/src/core.rs`
- Modify: `libs/updraft_core/src/input.rs`
- Modify: `libs/updraft_core/src/lib.rs`
- Modify: `libs/updraft_core/tests/scenario.rs`
- Modify: `tauri/src/driver.rs`
- Delete: `libs/updraft_core/src/snapshots/updraft_core__core__tests__invalid_external_device_mutations_warn.snap`

**Interfaces:**

- Changes: `AddExternalDevice::Response = ExternalDeviceId`
- Changes: `DeleteExternalDevice::Response = Result<(), UnknownExternalDevice>`
- Changes: `EditExternalDevice::Response = Result<(), UnknownExternalDevice>`
- Changes: `SetExternalDeviceEnabled::Response = Result<(), UnknownExternalDevice>`
- Changes: `ReorderExternalDevices::Response = Result<(), InvalidExternalDeviceOrder>`
- Preserves: unit responses for observations, startup, ticking, and locale

- [ ] **Step 1: Add failing response assertions at the core boundary**

Update the existing external-device cases in
`libs/updraft_core/tests/scenario.rs` to destructure both outputs:

```rust
let Update { effects, response } = core.apply(
    AddExternalDevice {
        spec: spec.clone(),
    },
    at(0),
);
assert_eq!(response, ExternalDeviceId(1));
assert_eq!(effects, expected_effects);
```

For unknown IDs:

```rust
let Update { effects, response } = core.apply(
    DeleteExternalDevice { device_id: unknown },
    at(0),
);
assert_eq!(
    response,
    Err(UnknownExternalDevice { device_id: unknown })
);
assert!(effects.is_empty());
```

For malformed reorders, assert the same `InvalidExternalDeviceOrder` for
unknown, missing, and duplicate IDs. For repeated device edits, enabled
states, and orders, assert success plus empty effects.

Keep the decoder and diagnostics unit tests in
`libs/updraft_core/src/core.rs`, but migrate them only as required by
`Update<R>`. Do not duplicate the scenario response matrix there.

Run:

```bash
cargo test -p updraft_core external_device
```

Expected: tests fail because all responses are still `()`.

- [ ] **Step 2: Define the public domain errors**

Add exact thiserror to `libs/updraft_core/Cargo.toml`:

```toml
thiserror = "=2.0.19"
```

In `libs/updraft_core/src/external_device.rs`, replace private `ReorderError` with:

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
#[error("unknown external device: {device_id:?}")]
pub struct UnknownExternalDevice {
    pub device_id: ExternalDeviceId,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
#[error("invalid external device order")]
pub struct InvalidExternalDeviceOrder;
```

Return `InvalidExternalDeviceOrder` directly from `ExternalDevices::reorder()`. Re-export both errors from `libs/updraft_core/src/lib.rs`.

- [ ] **Step 3: Implement natural responses without changing successful effects**

Change the five associated response types and pair every existing effects path with its natural result.

For addition, return the allocated ID after constructing the same open, topic, and persistence effects.

For delete, edit, and enable:

- Return `Err(UnknownExternalDevice { device_id })` with no effects when lookup fails.
- Return `Ok(())` with no effects for a repeated mutation.
- Return `Ok(())` with the existing effects for a state change.

For reorder:

- Return `Ok(())` with no effects for the current order.
- Return `Ok(())` with topic and persistence effects for a valid changed order.
- Return `Err(InvalidExternalDeviceOrder)` with no effects for any invalid order.

Remove the core warning calls for these expected rejections. Delete the
warning-focused core tests and obsolete snapshot because the scenario tests
already cover each rejection and its lack of effects.

Run:

```bash
cargo test -p updraft_core
```

Expected: all core and scenario tests pass.

- [ ] **Step 4: Assert typed driver results**

Update `tauri/src/driver.rs::external_device_mutations_drive_one_worker_and_complete_snapshots` so every mutation is awaited. Capture the ID directly from addition instead of discovering it from the topic:

```rust
let device_id = handle
    .send(AddExternalDevice {
        spec: first_spec.clone(),
    })
    .await
    .expect("the driver remains active");
```

Await the successful edit, enable, disable, and delete calls in that same
test. Do not repeat unknown-ID or invalid-order cases in the driver suite.
Those are core domain rules already covered by the scenario tests. Returning
the generated ID through `send()` is sufficient coverage that a non-unit
response crosses the erased driver queue.

Run:

```bash
cargo test -p updraft_tauri driver::tests
```

Expected: all driver tests pass.

- [ ] **Step 5: Verify and commit the natural responses**

Run:

```bash
cargo fmt --all --check
cargo test -p updraft_core
cargo test -p updraft_tauri
cargo clippy -p updraft_core -p updraft_tauri --all-targets --all-features -- -D warnings
git diff --check
```

Expected: all commands pass. No expected external-device rejection emits a core warning.

Commit:

```bash
git add libs/updraft_core/Cargo.toml libs/updraft_core/src/external_device.rs libs/updraft_core/src/core.rs libs/updraft_core/src/input.rs libs/updraft_core/src/lib.rs libs/updraft_core/tests/scenario.rs tauri/src/driver.rs libs/updraft_core/src/snapshots/updraft_core__core__tests__invalid_external_device_mutations_warn.snap
git commit -m 'core: Return external device mutation results'
```

---

### Task 3: Expose concrete request-response Tauri commands

**Files:**

- Modify: `libs/updraft_core/src/connection.rs`
- Modify: `tauri/Cargo.toml`
- Modify: `tauri/src/ipc.rs`
- Modify: `tauri/src/lib.rs`

**Interfaces:**

- Produces: `set_locale(locale) -> Result<(), DriverCommandError>`
- Produces: `add_external_device(spec) -> Result<ExternalDeviceId, DriverCommandError>`
- Produces: `delete_external_device(deviceId) -> Result<(), ExistingExternalDeviceCommandError>`
- Produces: `edit_external_device(deviceId, spec) -> Result<(), ExistingExternalDeviceCommandError>`
- Produces: `set_external_device_enabled(deviceId, enabled) -> Result<(), ExistingExternalDeviceCommandError>`
- Produces: `reorder_external_devices(order) -> Result<(), ReorderExternalDevicesCommandError>`
- Preserves: `subscribe(channel)` as the state-stream operation

- [ ] **Step 1: Add failing real-IPC tests**

Add a `#[cfg(test)]` module to `tauri/src/ipc.rs`. Build a mock app with the real generated handler, manage a real `DriverHandle`, and send `tauri::webview::InvokeRequest` values through `tauri::test::get_ipc_response()`:

Enable Tauri's `test` feature through a `tauri` dev-dependency in
`tauri/Cargo.toml` so these real-IPC test utilities are available.

```rust
fn request(command: &str, body: serde_json::Value) -> tauri::webview::InvokeRequest {
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
```

Keep this integration layer small. Cover only the three distinct wire
contracts:

- `add_external_device` deserializes a tagged TCP specification and returns
  the allocated numeric ID,
- `delete_external_device` deserializes camel-case `deviceId` and serializes
  `{ "kind": "unknownExternalDevice", "deviceId": ... }`,
- `reorder_external_devices` deserializes a numeric order and serializes
  `{ "kind": "invalidExternalDeviceOrder" }`.

Do not repeat every successful command, no-op, unknown-ID variant, or driver
shutdown case here. Core scenario tests own domain behavior, driver tests own
shutdown and completion, and the generated handler must compile with every
registered command.

Use a multi-thread Tokio runtime around the mock app so synchronous `get_ipc_response()` does not block the driver task.

Run:

```bash
cargo test -p updraft_tauri ipc::tests
```

Expected: compilation fails because the external-device commands and serialized errors do not exist.

- [ ] **Step 2: Define only the error variants each command can produce**

Add `Deserialize` to `ExternalDeviceId` in `libs/updraft_core/src/connection.rs` so numeric IDs and ID lists are valid command arguments. The type's JSON and generated TypeScript representation remain unchanged.

Retain `DriverCommandError` from Task 1 and define the two module-contained public domain-aware IPC errors in `tauri/src/ipc.rs`:

```rust
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
```

Map the driver's outer `Result` before mapping the input response's inner `Result`. Do not make locale or addition claim they can return device-domain errors.

- [ ] **Step 3: Implement and register the concrete commands**

Add asynchronous commands in `tauri/src/ipc.rs`:

```rust
#[tauri::command]
pub async fn add_external_device(
    spec: ConnectionSpec,
    handle: tauri::State<'_, DriverHandle>,
) -> Result<ExternalDeviceId, DriverCommandError> {
    handle
        .send(AddExternalDevice { spec })
        .await
        .map_err(|_| DriverCommandError::DriverStopped)
}
```

Implement delete, edit, enable, and reorder by flattening driver termination and the corresponding domain response into the restricted error enum. Make `set_locale` use `DriverCommandError`.

Register all commands in the one handler in `tauri/src/lib.rs`:

```rust
tauri::generate_handler![
    ipc::set_locale,
    ipc::add_external_device,
    ipc::delete_external_device,
    ipc::reorder_external_devices,
    ipc::edit_external_device,
    ipc::set_external_device_enabled,
    ipc::subscribe,
]
```

Run:

```bash
cargo test -p updraft_tauri ipc::tests
cargo test -p updraft_tauri
```

Expected: real IPC deserialization, successful results, and serialized rejections all pass.

- [ ] **Step 4: Verify and commit the Tauri commands**

Run:

```bash
cargo fmt --all --check
cargo test -p updraft_tauri
cargo clippy -p updraft_tauri --all-targets --all-features -- -D warnings
git diff --check
```

Expected: all commands pass.

Commit:

```bash
git add libs/updraft_core/src/connection.rs tauri/Cargo.toml tauri/src/ipc.rs tauri/src/lib.rs
git commit -m 'tauri: Add external device mutation commands'
```

- [ ] **Step 5: Run the complete validation matrix**

Run:

```bash
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test -p updraft_core --features ts bindings::tests::committed_bindings_are_up_to_date
git diff --check
```

Expected: all commands pass. The bindings check reports no generated protocol
changes. Frontend and packaged application builds are outside this Rust-only
slice.

- [ ] **Step 6: Review the complete diff**

Run:

```bash
git status --short
git diff --stat origin/main...HEAD
git diff origin/main...HEAD
rg -n "Input::|Any|downcast|fire.and.forget" libs/updraft_core tauri
rg -n "DriverStopped|struct .*Error|enum .*Error" libs/updraft_core tauri
```

Expected:

- no old enum variant syntax remains,
- no runtime response uses `Any`, downcasting, or a global response enum,
- all production input sends are awaited,
- external-device domain rejections produce no effects and no core warning,
- every new Rust error derives `thiserror::Error`,
- no frontend or `docs/design` files changed,
- no generic IPC input endpoint was added.

- [ ] **Step 7: Confirm final repository state**

Run:

```bash
git status --short --branch
git log --oneline --decorate -5
```

Expected: the worktree is clean on the named feature branch and the design,
plan, runtime, domain response, and IPC commits are present.
