# External Devices

Status: Current behavior

Updraft connects to external instruments through saved device entries. The
core owns the ordered configuration. The Tauri shell owns transport workers and
platform APIs.

## Supported connections

The current model supports two connection types:

- A TCP client connects to a host and port.
- Bluetooth Classic SPP connects to a device address and service UUID.

TCP provides the desktop development path and supports instruments that expose
a TCP server. Android provides Bluetooth SPP. Other desktop platforms retain
saved Bluetooth entries but cannot create a Bluetooth connection.

New Bluetooth entries use the standard SPP UUID. A saved device can retain a
custom UUID. The current frontend does not offer a control to change that UUID.

## Identity and order

The core assigns an `ExternalDeviceId` when it loads or creates an entry. The ID
identifies runtime state during one core session. The stored snapshot contains
the ordered configurations, not the runtime IDs.

Order defines priority for selected flight-data domains. A disconnected or
disabled device keeps its position. Reordering must contain every current ID
exactly once.

Every byte batch and connection-state update includes its device ID. This keeps
decoders, diagnostics, and flight-data candidates separate when connections run
in parallel.

## Lifecycle

Adding a device creates an enabled entry. Enabling a device asks the shell to
open its transport. Disabling or deleting it asks the shell to close the
transport.

Editing replaces the complete connection specification. The core closes the old
transport, clears its runtime decoder and candidates, and opens the replacement
when the entry remains enabled.

The shell owns connection attempts, cancellation, and retry delays. Connection
state reports the current situation to the core. It does not ask the core to
perform a retry.

Each successful configuration change publishes the complete ordered
`ExternalDevices` topic and persists the complete settings snapshot. Unknown
IDs and invalid reorder requests are errors. They do not change state.

## Settings interface

`/settings/devices` shows the saved devices in core order. Each row shows its
connection type, endpoint, and enabled state. A Bluetooth row also shows the
current bonded-device name when Android reports one.

The list does not currently show live connection status, data-source status, or
detected capabilities.

`/settings/devices/new` creates a device. `/settings/devices/[deviceId]` edits or
deletes one device. The TCP form requires a nonempty host and a port in
`1..=65535`.

On Android, Bluetooth creation uses the current bonded-device list. The page
distinguishes unsupported Bluetooth, denied permission, disabled Bluetooth, an
empty bonded list, and query failure. An existing unbonded device remains
editable and visible.

Controls show pending and failure states. A command failure keeps the published
device list unchanged.

## Data ingestion

Each external device owns one NMEA decoder and separate candidates for GPS,
pressure altitude, and true airspeed. A transport can provide more than one
domain. Different domains can select different devices.

Traffic is not a selected flight-data domain. The core merges accepted traffic
reports by target identity.

## Planned device management

The current interface does not include manual connection control, capability
chips, per-signal source overrides, BLE, USB, serial ports, UDP, device
profiles, NMEA output, vendor setting synchronization, or binary transfer
sessions. These functions remain product-scope items until a focused design
accepts them.
