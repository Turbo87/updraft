# External device settings

## Context

Updraft currently builds a fixed connection list in the Tauri shell. Desktop
starts one TCP client, while Android starts that TCP client and one Bluetooth
Classic SPP client. The core receives this list through `CoreConfig`, opens
every connection on startup, and keeps separate collections for connection
configuration, decoders, and diagnostics.

Locale settings already follow the desired ownership boundary. The core owns
the active value and requests persistence after a mutation. The Tauri shell
loads and atomically writes complete snapshots through one FIFO background
worker.

This design moves external device connections into that persistence path and
makes the ordered list mutable through core inputs. It does not add a user
interface yet.

## Scope

This slice supports:

- Adding an enabled external device at the end of the list.
- Deleting an external device.
- Replacing an external device's complete connection specification, including
  changing between TCP and Bluetooth.
- Reordering the complete external device list.
- Enabling or disabling an external device.
- Persisting device order, enabled state, and connection specification.
- Starting, stopping, and replacing the corresponding shell transport workers.

This slice does not add:

- Frontend controls, stores, routes, or Tauri device-mutation commands.
- A user-visible connection status or error model.
- Bluetooth discovery, pairing, or enable-Bluetooth behavior.
- Parallel Android SPP attempts.
- Deterministic arbitration between multiple configured SPP devices.
- Source-selection changes.
- Default device entries or migration from the current hardcoded connections.

## Data model

`ConnectionId` becomes `ExternalDeviceId`. It is the only domain identity for
an external device:

```rust
pub struct ExternalDeviceId(pub u32);
```

This rename is the first implementation commit and changes no behavior. It
updates the core, shell transports, tests, and snapshots while retaining the
current numeric representation and connection semantics. The repository
remains green before later commits introduce the device aggregate and runtime
mutation.

An ID is stable within one running core across connection retries, edits,
reordering, and enable or disable cycles. IDs are assigned again after an
application restart and are not persisted.

The persisted configuration for one device is:

```rust
pub struct ExternalDeviceConfig {
    pub enabled: bool,

    #[serde(flatten)]
    pub spec: ConnectionSpec,
}
```

The internal device aggregate owns both configuration and connection-local
runtime state:

```rust
struct ExternalDevice {
    id: ExternalDeviceId,
    config: ExternalDeviceConfig,
    decoder: Decoder,
    diagnostics: ConnectionDiagnostics,
}
```

`ConnectionDiagnostics` becomes the state for one device rather than a registry
containing every device. It obtains the device ID and connection specification
from the containing aggregate when it logs an event.

`ExternalDevices` owns the ordered entries and the session-local ID allocator:

```rust
struct ExternalDevices {
    next_id: u32,
    entries: Vec<ExternalDevice>,
}
```

It owns lookup, addition, deletion, reordering, ID allocation, and projection
to persisted and published representations. It assigns monotonically
increasing IDs and does not reuse an ID within one core lifetime. Keeping the
allocator here preserves deterministic core behavior and avoids process-global
mutable state.

`Core` therefore replaces its connection list, decoder map, and diagnostics
registry with one aggregate:

```rust
pub struct Core {
    settings: Settings,
    external_devices: ExternalDevices,
    instruments: Instruments,
}
```

Reordering moves the complete `ExternalDevice`, so its decoder buffer and
diagnostic attempt state remain attached to the same device.

## Runtime publication

The internal aggregate is not serialized because its decoder and diagnostics
are private runtime state. A published device contains the session identity and
its flattened configuration:

```rust
pub struct PublishedExternalDevice {
    pub device_id: ExternalDeviceId,
    #[serde(flatten)]
    pub config: ExternalDeviceConfig,
}
```

The core adds:

```rust
Topic::ExternalDevices(Vec<PublishedExternalDevice>)
```

A new subscriber receives this complete ordered list alongside the current
instrument and settings topics. Every successful device mutation emits the
complete list. The topic contains no connection status in this slice.

## Stored settings snapshot

The stored representation reuses `Settings` rather than repeating its fields:

```rust
#[serde(rename_all = "camelCase")]
pub struct SettingsSnapshot {
    #[serde(flatten)]
    pub settings: Settings,

    #[serde(default)]
    pub external_devices: Vec<ExternalDeviceConfig>,
}
```

The core constructs this snapshot by projecting the ordered runtime devices to
their configurations. `Effect::PersistSettings` carries the complete snapshot
to the existing shell-owned writer. Locale changes and device changes therefore
share the same serialized FIFO and cannot be persisted out of order.

`ConnectionSpec` uses an internally tagged representation. The Bluetooth
Classic SPP variant remains explicit in Rust and uses the shorter stored
transport name:

```rust
#[serde(tag = "type")]
pub enum ConnectionSpec {
    #[serde(rename = "tcp")]
    Tcp { host: String, port: u16 },

    #[serde(rename = "bluetooth")]
    BluetoothSpp { address: String },
}
```

A future BLE specification will use `"type": "ble"`.

The resulting file has flat device rows:

```json
{
  "locale": "de",
  "externalDevices": [
    {
      "enabled": true,
      "type": "tcp",
      "host": "127.0.0.1",
      "port": 4353
    },
    {
      "enabled": false,
      "type": "bluetooth",
      "address": "00:11:22:33:44:55"
    }
  ]
}
```

`ExternalDeviceId` never appears in this file.

## Core inputs

The core adds distinct mutation inputs:

```rust
Input::AddExternalDevice {
    spec: ConnectionSpec,
}

Input::DeleteExternalDevice(ExternalDeviceId)

Input::ReorderExternalDevices(Vec<ExternalDeviceId>)

Input::EditExternalDevice {
    id: ExternalDeviceId,
    spec: ConnectionSpec,
}

Input::SetExternalDeviceEnabled {
    id: ExternalDeviceId,
    enabled: bool,
}
```

The inputs remain distinct rather than introducing a generalized settings
mutation protocol.

### Add

Adding allocates the next session ID, creates an enabled device with fresh
decoder and diagnostic state, appends it, and requests that its connection be
opened.

### Delete

Deleting removes the complete aggregate. An enabled device also produces a
close effect. Later inputs for the removed ID are ignored.

### Reorder

Reordering accepts the complete desired ID order. The supplied IDs must be an
exact permutation of the current IDs, with no unknown, missing, or duplicate
entry. A valid reorder moves the existing aggregates and produces no transport
effects.

### Edit

Editing replaces the complete `ConnectionSpec`, so it supports TCP to
Bluetooth and Bluetooth to TCP changes. An identical specification is a no-op.

When the specification changes, the device keeps its `ExternalDeviceId` and
resets its decoder and diagnostics. An enabled device produces a close effect
followed by an open effect for the new specification. A disabled device remains
disconnected.

### Enable and disable

Enabling an already enabled device and disabling an already disabled device are
no-ops.

Disabling retains the device's configuration and list position, resets its
decoder and diagnostics, and produces a close effect. Later connection and byte
inputs are ignored while the device is disabled.

Enabling resets runtime state and produces an open effect for the stored
specification.

### Successful and invalid mutations

Every successful mutation emits the complete external devices topic and queues
one complete settings snapshot. A no-op does neither.

Unknown IDs and invalid reorder permutations leave state unchanged, produce no
connection or persistence effects, and log a warning rather than panicking.

## Startup

The Tauri shell loads `SettingsSnapshot` and passes its settings and ordered
device configurations to the core. The core assigns fresh IDs in stored order.

`Input::Start` produces an open effect for every enabled device and skips every
disabled device. An empty list produces no connection effects.

A missing settings file loads the default locale and an empty device list
without creating the file. An existing locale-only file retains its locale and
loads an empty device list because `externalDevices` defaults to empty.

The current hardcoded desktop TCP and Android TCP plus SPP entries are removed.
No platform receives a default connection.

An enabled specification unsupported by the current platform remains
configured. Bluetooth on desktop follows the existing unsupported transport
path and reports a disconnection.

## Shell transport lifecycle

The shell gains a transport manager that owns at most one maintained worker per
`ExternalDeviceId`.

An open effect starts the TCP or SPP supervisor for its specification. Opening
an already active ID first signals the existing worker to stop, then starts its
replacement. A close effect signals the worker to stop and removes it from the
manager.

TCP cancellation stops its reconnect loop and drops an active socket. TCP
workers remain independent, so any number of configured TCP clients can connect
in parallel.

SPP cancellation asks Android to cancel only when that supervisor owns the
active attempt. Cancelling a supervisor that failed to acquire the Android
singleton does not disturb another device's attempt.

Android retains the current single pending or active SPP attempt. Multiple
configured SPP supervisors may compete for that slot, fail with the existing
concrete error, and retry. This slice does not enforce which configured device
wins.

The design deliberately adds no generation or active-token filtering between
transport workers and the driver. A transport event queued concurrently with an
enabled specification edit may reach the same `ExternalDeviceId` after its
runtime state was reset. This narrow theoretical race is accepted until tests
or observed behavior justify more machinery.

## Persistence behavior

The existing settings worker continues to process complete snapshots in FIFO
order. For each snapshot it creates the application configuration directory if
necessary, writes JSON through a neighboring temporary file, and atomically
replaces `settings.json`.

Device mutations apply immediately. A write failure logs a warning and does not
roll back the active list or transport effects. A later successful mutation
writes the newest complete locale and device snapshot.

A malformed file logs a warning, remains untouched on disk, and loads complete
defaults. Invalid core mutation inputs do not queue persistence.

## Testing

Core tests cover:

- Loading ordered configurations with fresh session IDs.
- Adding an enabled device at the end.
- Deleting enabled and disabled devices.
- Accepting an exact reorder permutation.
- Rejecting unknown, missing, or duplicate reorder IDs.
- Preserving decoder and diagnostic state while reordering.
- Editing enabled and disabled devices.
- Treating an identical edit as a no-op.
- Changing from TCP to Bluetooth and from Bluetooth to TCP.
- Enabling and disabling devices.
- Ignoring connection and byte inputs for unknown or disabled IDs.
- Emitting complete topics and persistence snapshots only after changes.
- Opening only enabled devices during startup.

Serialization and persistence tests cover:

- Exact flat JSON for TCP and Bluetooth devices.
- Absence of runtime IDs from stored JSON.
- Locale-only files loading an empty device list.
- Round trips preserving order, enabled state, and specifications.
- Missing and malformed files retaining the existing behavior.
- Atomic replacement writing complete locale and device snapshots.

Shell tests cover:

- Multiple TCP workers operating independently.
- Closing TCP stopping its socket and reconnect loop.
- Closing an active SPP worker invoking Android cancellation.
- Closing an SPP supervisor without an active attempt leaving another device
  untouched.
- Replacing a worker under the same ID.
- Existing TCP retry, SPP retry, Android bridge, and locale persistence tests
  remaining green.

Implementation follows red-green-refactor for each behavior change. Repository
formatting, Rust tests, Clippy, frontend checks, Tauri builds, and the Android
debug build remain required final validation.
