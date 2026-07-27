# Android SPP transport

## Context

This design delivers the Bluetooth Serial Port Profile transport portion of
milestone 3 from the [application architecture](2026-07-25-app-architecture-design.md).
It builds on the foreground service, wake lock, permission flow, internal GPS,
and activity-relaunch handling delivered by the
[Android platform milestone](../plans/2026-07-25-android-platform.md).

The first hardware target is
[`Turbo87/esp32-bt-nmea-simulator`](https://github.com/Turbo87/esp32-bt-nmea-simulator/tree/83fb543aae7aeb153dc3829fa22b408aae295660)
connected to a Samsung S23. The simulator exposes an authenticated Classic
Bluetooth SPP service as `NMEA-SIM` and streams arbitrary RFCOMM byte chunks.

## Scope

The slice adds one hardcoded, read-only Android SPP connection. Android keeps
the existing hardcoded TCP connection and adds SPP under a distinct
`ConnectionId`. The temporary SPP address is `00:00:00:00:00:00` until the
simulator address is available.

The device is paired through Android Settings before Updraft starts. Bluetooth
is already enabled. Updraft does not scan, pair, or prompt to enable Bluetooth.

The connection is maintained for as long as the core requests it. A failed
attempt reconnects with the same exponential-backoff behavior as TCP. The
foreground service retains the `connectedDevice` type during connection
attempts and backoff whenever permission permits it, not only while the RFCOMM
socket is connected.

This slice excludes:

- Device configuration or selection UI.
- Bluetooth discovery.
- Outbound SPP data.
- Source selection between internal GPS, TCP, and SPP.
- Traffic parsing or presentation.
- BLE.
- Dynamic connection removal.

The physical acceptance test verifies the transport and bridge. It does not
assert which source controls the displayed ownship.

## Delivery order

The work lands as two independent PRs.

### PR 1: Generic connection diagnostics

The first PR contains no Bluetooth code. It adds structured connection
instrumentation to `updraft_core` and applies it to the existing TCP path.
This is independently useful and establishes the observability needed for the
physical SPP test.

### PR 2: Android SPP transport

The second PR adds the connection spec, Rust supervisor, mobile-plugin bridge,
Android RFCOMM worker, permission and foreground-service changes, hardcoded
configuration, and physical-device verification record.

## Connection diagnostics

`Core` owns private per-connection diagnostic state because it sees every
transport-neutral lifecycle transition and the byte chunks that actually reach
the core. `tracing` instrumentation does not alter domain state or effects.

When `Input::Start` opens a configured connection, the core already knows its
`ConnectionId` and `ConnectionSpec`. Subsequent inputs produce structured
instrumentation:

- `Connecting` emits at debug level with the connection ID and endpoint.
- `Connected` emits at info level and resets the attempt counters.
- The first known-connection `Bytes` input emits at info level and starts the
  delivered-byte count.
- Later byte inputs update the count without emitting routine activity.
- `Disconnected` emits at info level with the delivered-byte total when the
  attempt previously connected. If the attempt never connected, it emits at
  debug level with the connection ID and endpoint. Either case resets the
  attempt.
- Bytes for an unknown connection remain ignored and do not count as
  successful delivery.

TCP endpoints and Bluetooth MAC addresses are included because they are useful
for local diagnosis. Raw payload content is never logged.

Transport adapters retain concrete failure reporting. TCP logs its `io::Error`
with the host and port. SPP logs the Android or plugin failure with the MAC
address. Both then send the same transport-neutral `Disconnected` input, so a
failure produces one concrete warning plus either an info lifecycle summary
after connecting or a debug summary when connection was never established.

## Core and shell boundary

`ConnectionSpec` gains:

```rust
BluetoothSpp { address: String }
```

The core treats it like TCP. `Input::Start` emits `OpenConnection`, connection
states return as `Input::ConnectionChanged`, and incoming data returns as
`Input::Bytes`. No Bluetooth API, permission, retry, or parsing behavior enters
the core.

On Android, the shell configures both:

- TCP at `127.0.0.1:4353`.
- SPP at `00:00:00:00:00:00` until the simulator address replaces it.

Desktop retains TCP only.

The Rust SPP transport supervises the maintained connection. Kotlin performs
one blocking RFCOMM attempt at a time. This keeps retry policy beside TCP and
leaves Android code responsible only for Android mechanisms.

If implementation evidence shows that invoking one Android attempt per retry
causes a significant lifecycle or bridge problem, the retry loop may move into
`SessionService`. That fallback must preserve `ConnectionSpec`, the event
contract, connection states, backoff behavior, diagnostics, and foreground
service semantics.

## Retry behavior

TCP and SPP share a small reconnect-backoff helper:

- Initial delay: 250 milliseconds.
- Maximum delay: 10 seconds.
- Double the delay after every failed or empty attempt.
- Reset to 250 milliseconds only after an attempt delivers at least one byte.
- Retry until the process ends.

The helper owns delay progression only. Each transport retains its own socket
and event loop.

## Android attempt contract

When the maintained connection starts, Rust creates one Tauri channel and one
receiver for the lifetime of that connection. Reusing the channel avoids
accumulating Tauri's process-lifetime mobile channel registrations across
retries.

For each attempt, Rust:

1. Sends `Connecting` to the core.
2. Invokes the mobile plugin with the MAC address and maintained channel.
3. Translates channel events into core inputs.
4. Tracks whether the attempt delivered bytes.
5. Waits for the terminal `Disconnected` event.
6. Sends `Disconnected` to the core.
7. Waits for the current backoff and tries again.

The channel carries a tagged event union:

- `Connected`.
- `Bytes`, containing Base64-encoded raw bytes.
- `Disconnected`, optionally containing a failure description.

The terminal event is the attempt boundary. The Android plugin permits only one
pending or active attempt, emits events from that worker in order, and emits
exactly one `Disconnected` event last. Rust does not start another attempt
before receiving it. RFCOMM read boundaries are preserved. The existing core
decoder continues to own framing, resynchronization, and sentence parsing.
The wire contract does not carry an attempt generation.

The worker:

1. Obtains `BluetoothAdapter` from `BluetoothManager`.
2. Verifies that Bluetooth is enabled.
3. Resolves the configured MAC address.
4. Verifies that the device is already bonded.
5. Creates a secure RFCOMM socket with
   `00001101-0000-1000-8000-00805F9B34FB`.
6. Calls `connect()` and reads on a worker thread.
7. Copies every successful read before sending it over the channel.
8. Closes the socket and sends exactly one terminal event from `finally`.

Updraft follows XCSoar's current paired-device RFCOMM path and does not call
`BluetoothAdapter.cancelDiscovery()`. If S23 testing finds slow or unreliable
connection attempts, discovery cancellation and its additional scan permission
can be reconsidered with that evidence.

The service retains the active socket so service destruction or an explicit
attempt cancellation can abort a blocked `connect()` or read. A malformed
channel event causes Rust to cancel the active attempt and wait for its terminal
event before applying backoff.

## Permissions and foreground service

The plugin manifest adds:

- `android.permission.BLUETOOTH` with `maxSdkVersion="30"`.
- `android.permission.BLUETOOTH_CONNECT`.
- `android.permission.FOREGROUND_SERVICE_CONNECTED_DEVICE`.
- `connectedDevice` alongside `location` in the service type declaration.

It does not add `BLUETOOTH_SCAN` or `BLUETOOTH_ADMIN`.

Location and Nearby Devices permissions are requested independently during
initial startup while the activity is valid. Notification permission remains
optional. The service starts with the permitted requested source types:

- Location permission only starts internal GPS with the `location` type.
- Nearby Devices permission only starts SPP support with the
  `connectedDevice` type.
- Both permissions use the combined type mask.
- If neither source permission is granted, no foreground session starts and the
  failure is logged.

Denying Nearby Devices does not stop internal GPS. Denying location does not
stop SPP. If Nearby Devices is later granted through Android Settings while the
location foreground service remains alive, the next SPP attempt updates the
service to the combined mask before connecting.

The configured SPP request remains active while permission is denied, but
Android cannot activate the `connectedDevice` service type until its runtime
prerequisite is granted. While the process remains active, attempts continue
through the normal bounded backoff.

## Failure handling

Every failed attempt converges on one cleanup path:

1. Kotlin closes the socket.
2. Kotlin sends exactly one terminal event with the concrete failure when
   possible.
3. Rust logs that failure with the MAC address.
4. Rust sends `Disconnected` to the core.
5. The core emits the lifecycle and byte-count summary.
6. Rust waits according to backoff and retries.

The same path covers:

- Missing adapter.
- Disabled Bluetooth.
- Invalid or unbonded address.
- Missing permission.
- Plugin command rejection.
- RFCOMM connection failure.
- EOF or read failure.
- Malformed channel data.
- Service destruction.

Malformed channel data or invalid Base64 moves Rust into a cancelling state.
Rust logs the first protocol failure without payload content, requests
cancellation once, stops forwarding subsequent connection and byte events, and
waits for the terminal event. A cancellation-command failure is logged, but
does not permit another attempt. If no terminal event arrives, the maintained
SPP connection remains stalled until process restart.

The maintained Rust channel cannot close while its supervisor owns it. If the
receiver nevertheless closes, Rust reports the invariant failure, sends a final
`Disconnected` input, and stops the supervisor instead of creating another
channel. Process shutdown does not invoke a blocking mobile command from a
destructor.

Routine retries must not emit payloads or per-chunk logs. Process death ends
the Rust supervisor and Android worker together. A later launch starts a new
session, matching the Android platform milestone.

Activity destruction does not affect either side. The Rust process and
foreground service survive, and the Android worker uses the application and
service rather than the stale activity held by Tauri.

## Automated testing

PR 1 tests the structured core instrumentation:

- Connection ID and endpoint fields.
- Connecting and connected levels.
- One first-byte event per connected attempt.
- Accumulated byte totals.
- Counter reset across reconnects.
- Unknown-connection behavior.

The existing TCP integration test remains green and exercises the
instrumented path.

PR 2 adds host-side coverage for:

- Starting both configured Android connections.
- SPP event decoding, including Base64 bytes.
- Explicit failure on malformed events.
- A fake Android attempt that connects, sends chunks, reaches EOF, fails, and
  reconnects.
- Reuse of one channel across retries.
- Cancellation waits for the active worker's terminal event.
- Events received while cancelling are not forwarded.
- A failed cancellation command does not permit another attempt.
- Exponential delay progression.
- Backoff reset only after bytes.
- TCP behavior through the shared backoff helper.

Android unit tests additionally cover one pending or active source at a time,
cancellation closing the active socket, active-source cleanup before the next
attempt, and exactly one terminal event after EOF, connect failure, read
failure, or cancellation.

Repository tests, Clippy, formatting, frontend checks, and the Android debug
build must remain green.

## Physical acceptance

The S23 and `NMEA-SIM` provide the release gate that an emulator cannot:

1. Replace the temporary MAC and pair `NMEA-SIM` through Android Settings.
2. Install the debug APK and grant Nearby Devices.
3. Confirm connected and first-byte logs contain the SPP connection ID and
   MAC address.
4. Exercise small and large simulator chunk sizes and confirm delivery.
5. Background the app and turn the screen off. Disconnect afterward and
   confirm the accumulated byte total continued increasing.
6. Destroy and relaunch the activity. Confirm the process and SPP connection
   survive, then disconnect and inspect the byte total.
7. Run `disconnect spp` and confirm automatic reconnection.
8. Power-cycle the simulator and confirm automatic reconnection.
9. Deny Nearby Devices while allowing location. Confirm internal GPS
   continues and SPP failure is visible.
10. Deny location while allowing Nearby Devices. Confirm SPP still runs.
11. Inspect logs and confirm no NMEA payload appears.

Use `restart spp` before a playback-dependent check because the simulator does
not loop its built-in scenario automatically.

The acceptance record includes observed connection timing, reconnect behavior,
background and relaunch results, byte totals, permission outcomes, and any
deviation from the expected behavior. It does not use map position as evidence
until source selection has its own design and milestone.

## References

- [Android Bluetooth permissions](https://developer.android.com/develop/connectivity/bluetooth/bt-permissions)
- [Connect Bluetooth devices](https://developer.android.com/develop/connectivity/bluetooth/connect-bluetooth-devices)
- [Transfer Bluetooth data](https://developer.android.com/develop/connectivity/bluetooth/transfer-data)
- [Foreground service types](https://developer.android.com/develop/background-work/services/fgs/service-types)
- [XCSoar `BluetoothHelper.java`](https://github.com/XCSoar/XCSoar/blob/7b40e197a63120f4fe8016e5dbbead4665caf0e8/android/src/BluetoothHelper.java)
- [XCSoar `BluetoothClientPort.java`](https://github.com/XCSoar/XCSoar/blob/7b40e197a63120f4fe8016e5dbbead4665caf0e8/android/src/BluetoothClientPort.java)
