# Multiple Android SPP connections

## Purpose

Updraft can configure and start multiple external device connections. TCP
connections run in parallel. Bluetooth Classic SPP connections do not. The
Android mobile plugin permits only one pending or active SPP attempt for the
complete application.

This change lets every enabled SPP configuration maintain an independent
Android connection. It keeps the existing core model, connection settings,
NMEA ingestion, source handling, and Rust reconnect policy.

## Scope

This change provides these items:

- Parallel secure RFCOMM connections for all enabled SPP configurations.
- Independent connection, retry, byte delivery, and cancellation behavior.
- A keyed Android registry for pending and active SPP attempts.
- Targeted cancellation that cannot stop another SPP connection.
- Automated isolation tests in Rust and Kotlin.
- Physical validation with two simultaneous SPP endpoints.

This change does not provide these items:

- A connection limit or queue.
- Source arbitration or changes to displayed instrument selection.
- Endpoint deduplication.
- Device configuration user interface changes.
- Bluetooth discovery, pairing, or enable-Bluetooth controls.
- BLE support.
- Insecure RFCOMM fallback.
- Retry or backoff changes.

## Existing ownership

The core already assigns an `ExternalDeviceId` to each configured device. It
owns one decoder and one diagnostics state for each device. It emits one open
effect for every enabled device.

The Tauri driver already owns a separate transport worker for each
`ExternalDeviceId`. Multiple TCP workers therefore connect and retry in
parallel. The Rust SPP transport also creates a separate supervisor and Tauri
event channel for each configured SPP device.

The single-connection limit is inside the Android mobile plugin.
`SessionService` stores one pending SPP request. `SppAttemptOwner` stores one
active attempt. The cancel command has no connection identity. A second SPP
supervisor is rejected while the first supervisor is pending or active.

Kotlin continues to own Android permissions, foreground-service state, secure
RFCOMM sockets, blocking reads, and byte forwarding. Rust continues to own
retry, backoff, event validation, Base64 decoding, and core inputs.

## SPP connection identity

Each maintained Rust SPP supervisor already creates one Tauri `Channel` and
reuses it for every retry. Tauri gives each channel a process-local numeric ID.
A replacement supervisor creates a new channel and receives a new ID.

This design uses that channel ID as `SppConnectionId`. The ID identifies one
maintained SPP supervisor. It does not identify an external device or one
RFCOMM socket attempt.

The ID has these properties:

- All retries from one supervisor use the same ID.
- A replacement worker for the same `ExternalDeviceId` uses a different ID.
- The ID is not stored in settings.
- The ID is not sent to the core or frontend.
- The ID is not a credential.

Rust obtains the value from `Channel::id()`. Android obtains the same value
from `Channel.id`. Android stores it as a `Long` because the Tauri Android API
uses that type for channel IDs.

The Rust mobile bridge wraps the channel's `u32` ID in `SppConnectionId`.
`cancelSppAttempt` serializes it as the camel-case `connectionId` field. The
Android cancel arguments deserialize that field as a `Long`.

The mobile start request already carries the event channel. It does not need a
second copy of the ID. The cancel request adds the matching connection ID.

Using `ExternalDeviceId` for cancellation is not sufficient. An edit can stop
an old worker and start a replacement before the old worker finishes. Both
workers use the same external device ID. A late cancel from the old worker
must not close the replacement.

## Android SPP registry

The singleton owner becomes a synchronized registry keyed by
`SppConnectionId`. Each entry has one of these states:

- Pending owns the address, service UUID, event channel, and start callback.
- Active owns the endpoint context and the `SppSource` that controls one
  RFCOMM socket attempt.

The registry supplies narrow mutation operations:

- Reserve a pending request when its ID is unused.
- Activate the pending request for one ID.
- Abandon only the matching pending request after a start failure.
- Cancel only the active attempt for one ID.
- Clear only the matching ID and attempt instance.
- Drain all entries when the service is destroyed.

The registry rejects a duplicate reservation for one ID. It does not inspect
or block other IDs. A cancel for an unknown ID succeeds as a no-op. This makes
cancellation safe when a remote disconnect and a Rust stop request happen at
the same time.

The exact attempt-instance check protects against late thread cleanup. A stale
thread cannot remove a newer active entry that uses the same connection ID.

The registry holds its lock only while it inspects or changes the map. It calls
start callbacks and closes sockets after it releases the lock. Cleanup of one
socket therefore cannot block registry changes for another connection.

## Service lifecycle

`SessionService.startSppAttempt()` reserves the request by its event channel
ID. It puts that ID in the foreground-service intent. A `Channel` cannot travel
in an Android intent, so the registry continues to hold the complete request.

`SessionService.onStartCommand()` reads the ID from the SPP action. It promotes
only the matching request from pending to active. It starts one worker thread
for that `SppSource`. Different IDs therefore run on different threads and own
different sockets.

The foreground service remains shared. Parallel SPP connections use one
notification, one connected-device service type, and one wake lock. Repeated
foreground promotion and wake-lock acquisition remain idempotent.

The existing SPP event contract does not change:

- `Connected` reports successful socket connection.
- `Bytes` carries one Base64-encoded read buffer.
- `Disconnected` is the terminal event for one socket attempt.

Each Rust supervisor receives events only from its own channel. It forwards
connection state and bytes with the existing `ExternalDeviceId`. It then
applies its own reconnect backoff.

An Android attempt clears its exact registry entry when its thread finishes.
If Rust retries before that cleanup finishes, Android rejects only that retry.
Rust treats the rejection as an empty failed attempt and applies the normal
backoff. It does not affect another connection.

## Cancellation

The Rust `SppPlatform::cancel_attempt()` operation gains an
`SppConnectionId`. `AndroidSppPlatform` sends this value to
`cancelSppAttempt`. The desktop test implementation accepts the same argument.

Rust sends cancellation only after the matching start command succeeded. It
uses targeted cancellation in the existing cases:

- The configured device is disabled, deleted, or edited.
- The driver stops while an attempt is active.
- The SPP event channel contains malformed data.
- A byte event contains invalid Base64.

Android closes only the active socket stored under that ID. Rust still waits
for the terminal event before it ends the supervisor or starts another
attempt. Cancelling one supervisor does not change any other registry entry.

`SessionService.onDestroy()` drains the complete registry. It rejects every
pending request because the request did not start. It asks every active attempt
to close its socket. One socket-close failure is logged with connection
context, but it does not stop cleanup of the remaining entries. Active source
threads still emit their terminal events as they exit.

## Error behavior

A foreground-service start failure abandons and rejects only the matching
pending request. The service keeps an existing foreground session when GPS or
another SPP connection already uses it. If no foreground session exists, it
keeps the current stop behavior.

Socket creation, UUID parsing, Bluetooth state, bond state, connection, and
read failures remain local to one `SppSource`. The source emits its terminal
event. The matching Rust supervisor reports `Disconnected` and applies its
existing backoff.

Malformed channel data and invalid Base64 cancel only the connection that owns
that channel. Raw NMEA data stays out of logs.

Duplicate endpoint configurations remain valid. Two rows can contain the same
address and service UUID. Android tries both sockets independently. If the
remote device accepts only one socket, the other supervisor reports the
concrete failure and retries. Updraft does not add endpoint arbitration.

## Automated tests

Implementation follows red-green-refactor.

Rust transport tests cover these behaviors:

- Two maintained SPP supervisors receive different connection IDs.
- Both supervisors can report connected state and deliver bytes to their own
  `ExternalDeviceId`.
- Stopping one supervisor cancels only its connection ID.
- The other supervisor remains active after targeted cancellation.
- A malformed event on one channel does not cancel or consume events from the
  other channel.
- Replacing a worker creates a new connection ID, so a late old cancellation
  cannot target the replacement.
- Existing retry, terminal-event, driver-stop, UUID, and logging tests remain
  green.

Android registry tests cover these behaviors:

- Two different IDs can be pending or active at the same time.
- A duplicate ID is rejected without changing another entry.
- Activation selects the request for the supplied ID.
- Cancellation stops only the selected active attempt.
- Unknown cancellation is a no-op.
- Clear removes only the matching attempt instance.
- Service destruction rejects all pending requests and stops all active
  attempts.
- Service-start actions select the ID carried by their intent.

Existing `SppSource` tests continue to verify secure socket UUID selection,
connection events, byte encoding, stopping before reader assignment, and one
terminal event.

## Physical acceptance

The physical test configures two enabled Bluetooth connections:

- An ESP32 simulator that uses the standard SPP UUID.
- A macOS simulator that uses its custom RFCOMM service UUID.

Both endpoints start before Updraft. Android logs must show a distinct
connected event and first-byte event for each external device. Both connections
must deliver bytes during the same sustained interval.

The test uses the existing external-device command to disable the ESP32
configuration while the macOS connection continues to deliver bytes. Logs
must show targeted ESP32 cancellation and no macOS disconnect. Re-enabling the
ESP32 configuration must reconnect it without restarting the macOS connection.
The test then repeats the isolation check in the other direction.

Both connections must continue while the application activity is in the
background. Final shutdown must close both sockets. Sanitized logs must show
independent lifecycle events and delivered-byte totals. They must not contain
raw NMEA payloads.

The test does not assert which external source controls the displayed
instruments.
