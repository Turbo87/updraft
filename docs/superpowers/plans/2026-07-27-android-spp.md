# Android SPP Transport Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans to implement this plan task by task. Each step uses a checkbox for progress tracking.

**Goal:** Maintain one hardcoded, read-only Bluetooth Classic SPP connection on Android beside the existing TCP connection and feed its bytes through the existing core NMEA path.

**Architecture:** Rust owns connection requests, lifecycle inputs, Base64 decoding, and TCP-compatible retry. One process-lifetime Tauri channel carries serial RFCOMM attempts from a thin Kotlin worker that owns Android permissions, foreground-service state, secure socket creation, blocking reads, and cancellation. A terminal event is the attempt boundary, and the transport remains active while the core requests it, including retry delays.

**Tech Stack:** Rust 2024, Tauri 2.11.5, Tokio 1.53.1, `base64` 0.22.1, Kotlin, Android SDK 37, JDK 21, secure RFCOMM SPP UUID `00001101-0000-1000-8000-00805F9B34FB`.

This is PR 2 from the [Android SPP design](../specs/2026-07-27-android-spp-design.md), after the [connection diagnostics plan](2026-07-27-connection-diagnostics.md).

## Global Constraints

- Begin after the diagnostics PR merges, unless the user approves a stacked branch.
- Android keeps TCP at `127.0.0.1:4353` and adds SPP under a distinct `ConnectionId`. Desktop keeps TCP only.
- Use `00:00:00:00:00:00` as the temporary MAC sentinel until physical-device verification.
- Pair `NMEA-SIM` in Android Settings and enable Bluetooth before launch. Do not add scan, pairing, or enable-Bluetooth UI.
- Use secure RFCOMM and the standard SPP UUID. Do not call `cancelDiscovery()` or request `BLUETOOTH_SCAN`.
- Request Location and Nearby Devices independently at startup. Either source may operate when the other permission is denied.
- Keep the `connectedDevice` foreground-service type active while the core requests SPP and permission permits it, including retries.
- Retry after 250 ms, double to a 10 s cap, and reset only after an attempt delivers at least one byte.
- Each Android command starts one attempt. Rust owns the maintained loop unless implementation evidence shows a significant lifecycle or bridge advantage and the user approves moving it.
- Create one Tauri channel and receiver per maintained Rust connection and reuse them across retries. Do not create one mobile channel per attempt.
- Treat the terminal `Disconnected` event as the attempt boundary. Do not add attempt generations.
- After malformed channel data or invalid Base64, cancel once, ignore non-terminal events, and wait for the terminal event. A missing terminal event stalls SPP until process restart.
- Preserve RFCOMM read boundaries. Base64 is bridge encoding only. Never log payload content.
- Committed documentation must be standalone and safe for public review. Do not cite host-local paths or unavailable private artifacts. Summarize evidence inline and redact credentials, personal identifiers, and exact device addresses.
- Source selection, traffic handling, outbound SPP, BLE, discovery, dynamic removal, and device configuration UI are out of scope.
- Physical acceptance proves transport behavior, not ownship or map source selection.
- Do not edit generated files under `tauri/gen/android`.
- Dependencies remain pinned with `=`.
- If production changes approach 400 lines, stop and ask before expanding the PR.

## Task Completion Rule

For every task:

1. Write the smallest behavioral test first and confirm it fails for the intended missing behavior.
2. Implement only the described scope.
3. Run the listed verification.
4. Inspect the task diff with `git diff --check`, the listed `git diff`, and an `rg` scan for `TODO|TBD|FIXME|previously|removed|legacy|compat`.
5. Remove redundant comments, speculative helpers, duplicate state, and defensive branches not required by a real boundary.
6. Re-run verification after cleanup, review the complete task diff, then make the listed focused commit.

## Task 1: SPP connection specification and Android configuration

**Files:**

- Modify: `libs/updraft_core/src/connection.rs`
- Modify: `libs/updraft_core/src/core.rs`
- Modify: `tauri/src/lib.rs`
- Modify: `tauri/src/transport/mod.rs`

**Interfaces:**

```rust
ConnectionSpec::BluetoothSpp { address: String }
ConnectionSpec::bluetooth_spp(address: impl Into<String>) -> ConnectionSpec
configured_core(android: bool) -> CoreConfig
```

Reserve `ConnectionId(1)` for TCP and `ConnectionId(2)` for SPP.

- [ ] **Step 1: Add failing configuration tests**

Add these tests:

- Core `start_opens_every_configured_connection`: a config containing TCP and SPP produces both `Effect::open()` values in order.
- Core `spp_lifecycle_reports_the_mac_address`: a connecting event for ID 2 contains both the ID and placeholder MAC in the generic diagnostics endpoint.
- Tauri `android_configuration_keeps_tcp_and_adds_spp`: `configured_core(true)` returns TCP then SPP.
- Tauri `desktop_configuration_keeps_tcp_only`: `configured_core(false)` returns only TCP.

Keep the existing core `config()` helper TCP-only so unrelated tests do not change.

- [ ] **Step 2: Confirm the expected red state**

```bash
cargo test -p updraft_core start_opens_every_configured_connection
cargo test -p updraft_tauri android_configuration_keeps_tcp_and_adds_spp
```

Expected: failure because the SPP variant, constructor, and configuration helper do not exist.

- [ ] **Step 3: Implement the connection and configuration**

Add the variant and constructor. Extract the existing shell configuration into `configured_core(android)`. Always add TCP. Add the placeholder SPP connection only when `android` is true. Call it with `cfg!(target_os = "android")`.

Until Task 5 wires the bridge, handle SPP exhaustively in transport dispatch by logging the ID and address, sending `Disconnected`, and returning. Do not silently ignore it or add desktop SPP configuration.

- [ ] **Step 4: Verify and review**

```bash
cargo fmt --all --check
cargo test -p updraft_core --all-features
cargo test -p updraft_tauri --all-features
cargo clippy -p updraft_core --all-targets --all-features -- -D warnings
cargo clippy -p updraft_tauri --all-targets --all-features -- -D warnings
git diff --check
git diff -- libs/updraft_core/src/connection.rs libs/updraft_core/src/core.rs tauri/src/lib.rs tauri/src/transport/mod.rs
```

Confirm desktop is TCP-only, Android contains both stable IDs, the placeholder is deliberate, and no source-selection behavior entered the task.

- [ ] **Step 5: Commit**

```bash
git add libs/updraft_core/src/connection.rs libs/updraft_core/src/core.rs tauri/src/lib.rs tauri/src/transport/mod.rs
git commit -m "core: Add Bluetooth SPP connection spec"
```

## Task 2: Shared reconnect backoff

**Files:**

- Create: `tauri/src/transport/reconnect.rs`
- Modify: `tauri/src/transport/mod.rs`
- Modify: `tauri/src/transport/tcp.rs`

**Interface:**

```rust
ReconnectBackoff::default()
ReconnectBackoff::after_attempt(delivered_bytes: bool) -> Duration
```

- [ ] **Step 1: Add failing pure tests**

Test:

- Empty attempts return 250 ms, 500 ms, 1 s, 2 s, 4 s, 8 s, 10 s, then remain at 10 s.
- A byte-carrying attempt returns 250 ms and makes the following empty attempt return 500 ms.

- [ ] **Step 2: Confirm the expected red state**

```bash
cargo test -p updraft_tauri transport::reconnect
```

Expected: failure because `ReconnectBackoff` does not exist.

- [ ] **Step 3: Implement and adopt the helper**

Store only the next delay. `after_attempt(true)` resets it to 250 ms before returning and advancing it. `after_attempt(false)` returns the current delay and doubles it to the cap.

Refactor TCP so each attempt records whether it delivered bytes, sends the same lifecycle inputs as before, then sleeps for the helper's result. Delete the old constants and inline progression. Do not add async or transport dependencies to the helper.

- [ ] **Step 4: Verify and review**

```bash
cargo fmt --all --check
cargo test -p updraft_tauri transport::reconnect
cargo test -p updraft_tauri transport::tcp
cargo clippy -p updraft_tauri --all-targets --all-features -- -D warnings
git diff --check
git diff -- tauri/src/transport/reconnect.rs tauri/src/transport/mod.rs tauri/src/transport/tcp.rs
```

Confirm the helper owns only delay progression and TCP timing is unchanged.

- [ ] **Step 5: Commit**

```bash
git add tauri/src/transport/mod.rs tauri/src/transport/reconnect.rs tauri/src/transport/tcp.rs
git commit -m "tauri: Share connection retry backoff"
```

## Task 3: Rust SPP event contract and supervisor

**Files:**

- Modify: `libs/tauri_plugin_updraft/src/lib.rs`
- Modify: `libs/tauri_plugin_updraft/src/models.rs`
- Modify: `libs/tauri_plugin_updraft/Cargo.toml`
- Create: `libs/tauri_plugin_updraft/src/snapshots/tauri_plugin_updraft__models__tests__spp_events_use_a_tagged_camel_case_contract.snap`
- Create: `tauri/src/transport/spp.rs`
- Modify: `tauri/src/transport/mod.rs`
- Modify: `tauri/Cargo.toml`
- Modify: `Cargo.lock`

**Wire contract:**

```rust
#[serde(tag = "type", rename_all = "camelCase", deny_unknown_fields)]
pub enum SppEvent {
    Connected,
    Bytes { data: String },
    Disconnected { error: Option<String> },
}
```

**Supervisor seam:**

```rust
trait SppPlatform: Send + Sync + 'static {
    fn start_attempt(&self, address: &str, events: Channel) -> Result<(), String>;
    fn cancel_attempt(&self) -> Result<(), String>;
}

enum AttemptResult {
    Completed { delivered_bytes: bool },
    EventStreamClosed,
}
```

- [ ] **Step 1: Add failing wire-model tests**

Add `insta = { version = "=1.48.0", features = ["json"] }` as a plugin dev dependency. Deserialize these payloads through `InvokeResponseBody::Json(...).deserialize::<SppEvent>()`:

```json
{"type":"connected"}
{"type":"bytes","data":"JEc="}
{"type":"disconnected","error":"socket closed"}
{"type":"disconnected"}
```

Snapshot the resulting events with `insta::assert_json_snapshot!()`. Also assert that the IPC decoder rejects unknown fields. This tests the actual Tauri channel boundary without a direct `serde_json` dependency.

```bash
cargo test -p tauri-plugin-updraft spp_event
```

Expected: failure because `SppEvent` does not exist.

- [ ] **Step 2: Implement the wire model**

Derive `Clone`, `Debug`, `Deserialize`, `Serialize`, `PartialEq`, and `Eq`. Default a missing disconnect error to `None`. Re-export `SppEvent` beside `Fix`. Keep the model free of Android, Tauri, socket, and retry types.

Run the targeted test, inspect the pending snapshot, accept it with `cargo insta accept`, and rerun the test. Accept only the snapshot created by this task.

- [ ] **Step 3: Add failing supervisor tests**

Add `base64 = "=0.22.1"` to `tauri/Cargo.toml`. Use a fake `SppPlatform` that sends raw channel JSON and counts attempts and cancellations. Add:

- `connected_bytes_reach_the_existing_nmea_path`
  - Send connected, one Base64 RMC sentence, then disconnected.
  - Assert `run_attempt()` reports delivered bytes and the existing driver publishes the parsed position.
- `malformed_event_cancels_and_waits_for_the_terminal_event`
  - Send malformed JSON and assert cancellation is requested once.
  - Prove the attempt remains pending while subsequent connected and byte events are ignored.
  - Send disconnected and assert the attempt completes without delivering bytes.
- `invalid_base64_cancels_and_waits_for_the_terminal_event`
- `terminal_event_reconnects_after_the_current_delay_on_the_same_channel`
  - Pause Tokio time and prove the second attempt starts at 250 ms, not 249 ms.
  - Assert every attempt receives the same Tauri channel ID.
- `failed_cancellation_does_not_start_another_attempt`
  - Return an error from `cancel_attempt()`, omit the terminal event, advance paused time beyond 10 s, and assert only one attempt started.

```bash
cargo test -p updraft_tauri transport::spp
```

Expected: failure because the platform seam and supervisor do not exist.

- [ ] **Step 4: Implement the supervisor**

Create one `tauri::ipc::Channel` and Tokio receiver when `maintain()` starts. Keep both alive for the maintained connection and clone the same channel into every platform command. Deserialize each `InvokeResponseBody` into `SppEvent`.

`run_attempt()` must:

1. Send `Connecting`.
2. Start one platform attempt. On synchronous failure, warn with connection ID, address, and reason, then send `Disconnected`.
3. Translate `Connected` to the core.
4. Decode `Bytes` with Base64 `STANDARD`, forward the exact decoded vector, and remember whether any non-empty bytes were delivered.
5. On the first invalid Base64 or malformed channel value, warn without payload content, request cancellation once, and enter a cancelling state.
6. While cancelling, discard every non-terminal value without forwarding bytes or connection state.
7. Complete only after a valid terminal `Disconnected` event. A cancellation-command error is logged but does not permit another attempt.
8. On a disconnect error, warn with the connection ID, address, and reason.
9. Send exactly one final `Disconnected` and return `AttemptResult::Completed { delivered_bytes }`.

If the maintained receiver closes, warn with the connection ID, address, and cause, send the final `Disconnected`, return `AttemptResult::EventStreamClosed`, and stop `maintain()` without creating another channel.

`maintain()` otherwise loops forever, invokes `run_attempt()` with the maintained channel and receiver, and sleeps through `ReconnectBackoff`. A `Connected` event alone must not reset backoff. Do not add attempt generations, timeouts, payload logs, another channel, or another byte buffer. Process shutdown does not call the mobile bridge from `Drop`.

- [ ] **Step 5: Verify and review**

```bash
cargo fmt --all --check
cargo test -p tauri-plugin-updraft spp_event
cargo test -p updraft_tauri transport::spp
cargo clippy -p tauri-plugin-updraft --all-targets --all-features -- -D warnings
cargo clippy -p updraft_tauri --all-targets --all-features -- -D warnings
git diff --check
git diff -- Cargo.lock libs/tauri_plugin_updraft tauri/Cargo.toml tauri/src/transport/spp.rs tauri/src/transport/mod.rs
```

Confirm all failures include the MAC and cause, bad bridge input cancels once and waits for the terminal event, no payload is logged, ignored cancelling-state events do not reach the core, and every retry reuses one channel ID.

- [ ] **Step 6: Commit**

```bash
git add Cargo.lock libs/tauri_plugin_updraft/Cargo.toml libs/tauri_plugin_updraft/src/lib.rs libs/tauri_plugin_updraft/src/models.rs tauri/Cargo.toml tauri/src/transport/mod.rs tauri/src/transport/spp.rs
git commit -m "tauri: Add SPP transport supervisor"
```

## Task 4: Independent permissions and foreground-service types

**Files:**

- Modify: `libs/tauri_plugin_updraft/android/src/main/AndroidManifest.xml`
- Modify: `libs/tauri_plugin_updraft/android/src/main/java/UpdraftMobilePlugin.kt`
- Modify: `libs/tauri_plugin_updraft/android/src/main/java/SessionService.kt`
- Modify: `libs/tauri_plugin_updraft/android/build.gradle.kts`
- Create: `libs/tauri_plugin_updraft/android/src/test/java/SessionServiceTest.kt`

- [ ] **Step 1: Add failing foreground-type tests**

Add JUnit 4.13.2. Test `foregroundServiceTypes(location, spp)` for all four combinations:

- Location only returns `FOREGROUND_SERVICE_TYPE_LOCATION`.
- SPP only returns `FOREGROUND_SERVICE_TYPE_CONNECTED_DEVICE`.
- Both returns the bitwise combination.
- Neither returns zero.

Generate the Android project:

```bash
NDK_HOME="$ANDROID_HOME/ndk/28.1.13356709" pnpm tauri android build --target aarch64 --apk --debug
```

Then run from `tauri/gen/android`:

```bash
./gradlew :tauri-plugin-updraft:testDebugUnitTest
```

Expected: failure because `foregroundServiceTypes()` does not exist. If the generated module name differs, read `tauri/gen/android/tauri.settings.gradle` and use its exact name.

- [ ] **Step 2: Add manifest capabilities**

Declare:

- `BLUETOOTH` with `maxSdkVersion="30"`.
- `BLUETOOTH_CONNECT`.
- `FOREGROUND_SERVICE_CONNECTED_DEVICE`.
- Service type `location|connectedDevice`.

Do not add `BLUETOOTH_SCAN`, `BLUETOOTH_ADMIN`, or background location.

- [ ] **Step 3: Make foreground types source-dependent**

Implement the tested pure mask function. Extend `SessionService.start()` and its intent with `location` and `spp` booleans. On each start, combine requested types with the service's current mask before calling `startForeground()`.

Start `GpsSource` only when location was requested. An SPP-only session still holds the wake lock and foreground notification. Initial session startup must include `connectedDevice` when Nearby Devices is granted so the type remains active between SPP attempts.

- [ ] **Step 4: Make startup permissions independent**

Add a Nearby Devices permission alias for `BLUETOOTH_CONNECT`. For Android before S, treat it as granted.

The startup sequence is:

1. Request location only when missing.
2. Continue to the Bluetooth request regardless of the location result.
3. Continue to notification permission regardless of the Bluetooth result.
4. Reject only if both source permissions are unavailable.
5. Start the service with booleans reflecting the permissions actually granted.

Notification permission remains optional. Activity permission prompts occur only during initial startup. A location denial must not block SPP and a Bluetooth denial must not block GPS.

- [ ] **Step 5: Verify and review**

```bash
cargo fmt --all --check
cargo test -p tauri-plugin-updraft --all-features
NDK_HOME="$ANDROID_HOME/ndk/28.1.13356709" pnpm tauri android build --target aarch64 --apk --debug
```

From `tauri/gen/android`:

```bash
./gradlew :tauri-plugin-updraft:testDebugUnitTest
```

Also inspect the merged debug manifest for the combined service type, required connect permissions, and absence of scan permission.

```bash
git diff --check
git diff -- libs/tauri_plugin_updraft/android
```

Confirm all four permission combinations have an explicit outcome and the service retains `connectedDevice` through retry delays.

- [ ] **Step 6: Commit**

```bash
git add libs/tauri_plugin_updraft/android
git commit -m "android: Support Bluetooth session permissions"
```

## Task 5: Secure RFCOMM worker and end-to-end bridge

**Files:**

- Create: `libs/tauri_plugin_updraft/android/src/main/java/SppReader.kt`
- Create: `libs/tauri_plugin_updraft/android/src/main/java/SppSource.kt`
- Create: `libs/tauri_plugin_updraft/android/src/main/java/SppAttemptOwner.kt`
- Create: `libs/tauri_plugin_updraft/android/src/test/java/SppReaderTest.kt`
- Create: `libs/tauri_plugin_updraft/android/src/test/java/SppSourceTest.kt`
- Create: `libs/tauri_plugin_updraft/android/src/test/java/SppAttemptOwnerTest.kt`
- Modify: `libs/tauri_plugin_updraft/android/src/main/java/SessionService.kt`
- Modify: `libs/tauri_plugin_updraft/android/src/main/java/UpdraftMobilePlugin.kt`
- Modify: `libs/tauri_plugin_updraft/src/mobile.rs`
- Modify: `libs/tauri_plugin_updraft/src/desktop.rs`
- Modify: `tauri/src/transport/spp.rs`
- Modify: `tauri/src/transport/mod.rs`
- Modify: `tauri/src/lib.rs`
- Modify: `.github/workflows/ci.yml`

**Bridge interface:**

- Kotlin commands: `startSppAttempt` and `cancelSppAttempt`.
- Rust plugin methods: `start_spp_attempt(address, events)` and `cancel_spp_attempt()`.
- Channel events: connected, Base64 bytes, and disconnected with an optional error.

- [ ] **Step 1: Add failing blocking-I/O tests**

Create `SppReaderTest.kt` around a fake `SppSocket` and controlled `InputStream`. Test:

- Connect emits connected, copies each bulk-read chunk, stops at EOF, and closes.
- Connect failure is returned and closes.
- Read failure is returned and closes.
- `stop()` closes the socket.
- Separate reads produce separate copied byte arrays.

From `tauri/gen/android`, run:

```bash
./gradlew :tauri-plugin-updraft:testDebugUnitTest --tests aero.updraft.mobile.SppReaderTest
```

Expected: failure because `SppSocket` and `SppReader` do not exist.

- [ ] **Step 2: Implement the blocking reader**

Define:

```kotlin
internal interface SppSocket {
    val input: InputStream
    fun connect()
    fun close()
}

internal interface SppAttempt {
    fun run()
    fun stop(): Exception?
}

internal data class SppRequest(
    val address: String,
    val events: Channel,
    val onStarted: (Exception?) -> Unit,
)

internal class SppAttemptOwner {
    fun reserve(request: SppRequest): Boolean
    fun abandon(request: SppRequest)
    fun activate(factory: (SppRequest) -> SppAttempt): Pair<SppRequest, SppAttempt>?
    fun clear(attempt: SppAttempt)
    fun cancel(): Exception?
}
```

`SppReader.run()` connects, emits connected, performs 4096-byte bulk reads, copies exactly the bytes read into each callback, and stops at EOF. It returns the first connect, read, or close exception and always closes. `stop()` closes the socket and returns any exception. Do not merge adjacent reads or add another buffer.

- [ ] **Step 3: Add failing terminal-event and ownership tests**

Create `SppSourceTest.kt` with an injected fake `SppReader` and collecting Tauri `Channel`. Test:

- EOF emits connected and bytes in read order, then exactly one disconnected event last.
- Connect and read failures each emit exactly one disconnected event carrying the error.
- `stop()` before reader assignment still closes the reader when `run()` assigns it.
- Cancellation emits no event after the terminal disconnected event.

Create `SppAttemptOwnerTest.kt` with fake `SppAttempt` values. Test:

- The first request reserves the owner and a second pending request is rejected.
- Activation atomically moves the pending request to the active attempt.
- A request is rejected while an attempt is active.
- `cancel()` stops the active attempt without clearing it before worker exit.
- `clear()` permits the next reservation only for the active attempt instance.
- `abandon()` releases only the matching pending request after service startup failure.

From `tauri/gen/android`, run:

```bash
./gradlew :tauri-plugin-updraft:testDebugUnitTest --tests aero.updraft.mobile.SppSourceTest --tests aero.updraft.mobile.SppAttemptOwnerTest
```

Expected: failure because `SppSource`, `SppAttempt`, `SppRequest`, and `SppAttemptOwner` do not exist.

- [ ] **Step 4: Implement the Android adapter and one-attempt owner**

`SppSource` must:

- Check `BLUETOOTH_CONNECT` at the socket boundary on Android S and later.
- Resolve `BluetoothManager`, require an enabled adapter, resolve the configured address, and require `BOND_BONDED`.
- Open `createRfcommSocketToServiceRecord()` with the standard SPP UUID.
- Emit `{"type":"connected"}` after connect.
- Emit each read as `{"type":"bytes","data":"..."}` using `Base64.NO_WRAP`.
- Emit exactly one terminal disconnected event with a nullable error.
- Set its stop flag before reading the active reader so cancellation also wins the race before reader assignment.
- Avoid discovery APIs and payload logs.

`SppAttemptOwner` owns the single lock protecting one pending `SppRequest` and one active `SppAttempt`. `cancel()` reads the active attempt under the lock and calls `stop()` after releasing it. Only the worker's identity-matched `clear()` releases the active slot.

`SessionService.startSppAttempt()` must reject if the owner cannot reserve the request, store it before starting or delivering the service command, abandon that exact request if service startup fails, and report startup through its callback.

When the service receives the request:

1. Require the connected-device foreground type.
2. Move the pending request to an active source under the lock.
3. Start one named worker thread.
4. Acquire the wake lock and resolve the command.
5. Clear the active source when the worker exits.

`cancelSppAttempt()` and service destruction close the active socket. Socket close is the cancellation mechanism for blocked connect and read. Keep the service alive after an attempt so the Rust-owned retry loop remains covered by the foreground type.

- [ ] **Step 5: Add Kotlin and Rust plugin commands**

The Kotlin start arguments contain `address: String` and `events: Channel`. Reject start when Nearby Devices is unavailable, delegate to `SessionService`, and resolve cancellation after requesting socket close.

The Rust mobile methods invoke matching command names. Desktop methods may return `Ok(())` to keep the shared plugin API buildable, but desktop transport dispatch must report SPP unsupported and must not invoke them.

- [ ] **Step 6: Connect the platform to the Rust supervisor**

Implement an Android `SppPlatform` backed by `AppHandle<R>` and `UpdraftMobileExt`. Add Android-only `spp::run()` that spawns `maintain()`.

Change transport dispatch to accept an `AppHandle<R>`:

- TCP continues to call its existing runner.
- Android SPP calls `spp::run(connection, address, handle, app)`.
- Non-Android SPP warns with connection ID and address, then sends `Disconnected`.

Capture a cloned app handle in the driver's `OpenFn` closure. Do not change `OpenFn` or give the core an app handle.

- [ ] **Step 7: Add the JVM tests to CI**

After the Android debug APK build, run:

```yaml
- run: ./gradlew :tauri-plugin-updraft:testDebugUnitTest
  working-directory: tauri/gen/android
```

- [ ] **Step 8: Verify and review**

```bash
cargo fmt --all --check
cargo test -p updraft_core --all-features
cargo test -p tauri-plugin-updraft --all-features
cargo test -p updraft_tauri --all-features
cargo clippy -p updraft_core --all-targets --all-features -- -D warnings
cargo clippy -p tauri-plugin-updraft --all-targets --all-features -- -D warnings
cargo clippy -p updraft_tauri --all-targets --all-features -- -D warnings
cargo clippy -p tauri-plugin-updraft --target aarch64-linux-android --all-targets --all-features -- -D warnings
cargo clippy -p updraft_tauri --target aarch64-linux-android --all-targets --all-features -- -D warnings
NDK_HOME="$ANDROID_HOME/ndk/28.1.13356709" pnpm tauri android build --target aarch64 --apk --debug
```

From `tauri/gen/android`:

```bash
./gradlew :tauri-plugin-updraft:testDebugUnitTest
```

Then inspect:

```bash
git diff --check
git diff -- .github/workflows/ci.yml libs/tauri_plugin_updraft tauri
rg -n "cancelDiscovery|BLUETOOTH_SCAN|payload" libs/tauri_plugin_updraft tauri/src
```

Confirm one terminal event, at most one pending or active attempt, one active socket, copied buffers, closure on every path, owner release only after worker exit, reuse of the maintained Rust channel across retries, no discovery cancellation, no payload logs, and foreground-type retention across retries.

- [ ] **Step 9: Commit**

```bash
git add .github/workflows/ci.yml libs/tauri_plugin_updraft tauri
git commit -m "android: Connect paired SPP devices"
```

## Task 6: Physical S23 acceptance and durable record

**Files:**

- Modify: `tauri/src/lib.rs`
- Create: `docs/superpowers/verification/2026-07-27-android-spp.md`

- [ ] **Step 1: Resolve and install the real test build**

Read the paired `NMEA-SIM` MAC from Android Settings or:

```bash
adb shell dumpsys bluetooth_manager
```

Replace only the production sentinel with the observed uppercase colon-delimited address. Confirm it is gone from production configuration:

```bash
rg -n "00:00:00:00:00:00" tauri/src/lib.rs
```

Build and install:

```bash
NDK_HOME="$ANDROID_HOME/ndk/28.1.13356709" pnpm tauri android build --target aarch64 --apk --debug
adb install -r tauri/gen/android/app/build/outputs/apk/universal/debug/app-universal-debug.apk
```

- [ ] **Step 2: Capture clean lifecycle evidence**

```bash
adb logcat --clear
adb logcat -v threadtime
```

Launch Updraft and grant Nearby Devices. Save logs for `Connecting`, `Connected`, `First bytes`, `Disconnected`, `SPP`, `SessionService`, and `SppSource`.

Confirm connection ID 2 and the real MAC appear in lifecycle events, first bytes arrive, a connected disconnect reports a positive total, and no payload appears.

- [ ] **Step 3: Exercise RFCOMM boundaries**

Run these simulator commands in order:

```text
chunk spp 1
restart spp
chunk spp 512
restart spp
```

For each size, observe connection, first bytes, and a positive final total. Do not use displayed ownship position as evidence.

- [ ] **Step 4: Exercise lifecycle and reconnection**

With SPP streaming:

1. Keep the simulator serial console available. Its playback does not loop.
2. Run `restart spp` immediately before each five-minute interval and again before playback reaches EOF. Record every restart time.
3. Background Updraft for five minutes while restarting simulator playback as needed.
4. Turn the screen off for five minutes while restarting simulator playback as needed.
5. Run `disconnect spp` and confirm the byte total increased across both intervals.
6. Confirm automatic reconnect.
7. Swipe the Updraft activity from recents without force stopping the app.
8. Relaunch and confirm the process, foreground service, and stream survived.
9. Run `disconnect spp` again and record the total.
10. Power-cycle the simulator and confirm bounded reconnect.

Record PID evidence. Activity destruction should not change it. Force stop is a separate process-death case.

- [ ] **Step 5: Exercise independent permissions**

Through Android Settings, verify:

- Location granted and Nearby Devices denied: internal GPS continues and SPP failures remain visible.
- Nearby Devices granted and Location denied: SPP connects and streams.

If revocation kills the process, record it and verify the surviving source after relaunch. Inspect service types:

```bash
adb shell dumpsys activity services aero.updraft.debug
```

Confirm location-only, connected-device-only, or combined types as appropriate.

- [ ] **Step 6: Write the evidence record**

Create `docs/superpowers/verification/2026-07-27-android-spp.md` with:

- S23 model, Android version, build fingerprint, and WebView version.
- Tested app configuration and APK SHA-256 and size.
- Simulator firmware configuration, paired name, and confirmation that the
  configured address matched the bonded device. Do not publish the exact device
  address.
- Timings from `Connecting` to `Connected` and `First bytes`.
- Chunk-size results for 1 and 512.
- Background, screen-off, activity-destruction, and relaunch durations with PID evidence and simulator playback restart times.
- Disconnect and power-cycle reconnect timings.
- Permission combinations and foreground-service type masks.
- Delivered-byte totals supporting each streaming claim.
- Every failure, deviation, or test not run.
- Statements that no payload was logged and map position was not used as evidence.

Use measured values and captured output. Do not add unchecked pass claims.

- [ ] **Step 7: Run full verification**

```bash
cargo fmt --all --check
cargo test --workspace --exclude tauri-plugin-updraft --all-features
cargo test -p tauri-plugin-updraft --all-features
cargo clippy --workspace --exclude tauri-plugin-updraft --all-targets --all-features -- -D warnings
cargo clippy -p tauri-plugin-updraft --all-targets --all-features -- -D warnings
cargo doc --workspace --exclude tauri-plugin-updraft --no-deps --all-features
cargo doc -p tauri-plugin-updraft --no-deps --all-features
pnpm lint
pnpm build
pnpm build:storybook
pnpm check
pnpm test
pnpm test:e2e
NDK_HOME="$ANDROID_HOME/ndk/28.1.13356709" pnpm tauri android build --target aarch64 --apk --debug
```

From `tauri/gen/android`:

```bash
./gradlew :tauri-plugin-updraft:testDebugUnitTest
```

Classify every failure as branch-caused, pre-existing, or environmental and retain the evidence.

- [ ] **Step 8: Deslop and review the acceptance diff**

```bash
git diff --check
git diff -- tauri/src/lib.rs docs/superpowers/verification/2026-07-27-android-spp.md
rg -n "TODO|TBD|FIXME|00:00:00:00:00:00|obviously|clearly|successful" tauri/src/lib.rs docs/superpowers/verification/2026-07-27-android-spp.md
```

Remove repeated narrative and unsupported conclusions. Keep timings, PID, type masks, permissions, and byte totals. Confirm the public record contains no NMEA payload, host-local paths, unavailable artifact references, credentials, personal identifiers, or exact device addresses.

- [ ] **Step 9: Commit**

```bash
git add tauri/src/lib.rs docs/superpowers/verification/2026-07-27-android-spp.md
git commit -m "docs: Record Android SPP device verification"
```

## PR 2 Review Gate

Do not push or create a pull request.

```bash
git diff --check origin/main...HEAD
git diff --stat origin/main...HEAD
git diff origin/main...HEAD
git log --oneline origin/main..HEAD
rg -n "TODO|TBD|FIXME|cancelDiscovery|BLUETOOTH_SCAN|BLUETOOTH_ADMIN" libs/updraft_core libs/tauri_plugin_updraft tauri
rg -n "00:00:00:00:00:00" tauri/src/lib.rs
```

Repeat Task 6 Step 7. Confirm the complete branch matches the approved design, reuses one maintained Tauri channel, treats terminal disconnect as the only retry boundary, and contains no PR 1 implementation, attempt generations, source selection, configuration UI, payload logging, or generated Android edits.

Present the complete code diff, commits, automated results, physical evidence, production-line count, and deslop findings to the user. Wait for explicit approval before pushing or opening PR 2.
