# Devices screen

## Purpose

A fresh Updraft installation has no external devices. The core can store and
operate TCP and Bluetooth Classic SPP devices, but the frontend cannot configure
them. A user must edit `settings.json` before the existing transports are
usable.

This change adds a Devices screen. It uses the existing external-device topic
and mutation commands. It also adds one read-only Android query for bonded
Bluetooth devices.

## Scope

This change provides these items:

- A list of configured TCP and Bluetooth SPP devices.
- TCP device creation and editing.
- Android Bluetooth SPP device creation and editing from bonded devices.
- The standard SPP service UUID for new Bluetooth devices.
- A read-only display for an existing custom service UUID.
- Device enable, disable, and deletion controls.
- Complete frontend state synchronization through the existing
  `ExternalDevices` topic.
- Browser development and component testing through the existing fake client.

This change does not provide these items:

- Source-priority wording or source-selection changes.
- Connection status or connection errors.
- Detected device capabilities.
- Bluetooth scanning, pairing, or enable-Bluetooth controls.
- Manual Bluetooth address entry.
- Custom Bluetooth service UUID creation or editing.
- Persisted Bluetooth display names.
- Default device entries.
- Device reordering controls.
- BLE, UDP, serial, or USB configuration.
- Internal sensor configuration.

The list follows the stored topic order. The screen does not describe that
order as source priority.

## Navigation and routes

The Settings screen links to `/devices`. The flight view keeps its existing
Settings link and does not add a separate Devices link.

The device flow uses these routes:

- `/devices` shows the configured device list.
- `/devices/new` adds one device.
- `/devices/[deviceId]` edits one configured device.

`ExternalDeviceId` is valid only for one core session. The edit route waits for
the first external-device topic before it resolves the route ID. It shows a
loading state before that topic arrives. It shows `Device not found` with a
link to `/devices` when the received list does not contain the ID.

## Frontend state

A new `ExternalDevicesStore` owns the frontend projection of configured
devices. It stores the current ordered `PublishedExternalDevice` list and a
flag that records whether it has received the initial topic.

The application layout sends every topic to this store. An
`ExternalDevices` topic replaces the complete list. Other topics do not change
the store.

The topic is the only source of shared device state. Mutation responses report
success or failure. They do not update the store. Topic delivery and command
completion can arrive in either frontend order.

The frontend does not wait for a settings file write or a transport connection
attempt. Those operations retain their existing background behavior.

## Client boundary

`UpdraftClient` adds methods for these existing Tauri commands:

- `add_external_device`
- `edit_external_device`
- `set_external_device_enabled`
- `delete_external_device`

The client methods use the generated `ConnectionSpec`, `ExternalDeviceId`, and
`PublishedExternalDevice` types. `TauriClient` invokes the concrete commands.
It does not expose a generic core-input command.

`FakeClient` owns an ordered in-memory device list and a session-local ID
allocator. Each successful change emits one complete external-device topic.
The fake applies the same add, edit, enable, disable, and delete semantics that
the frontend observes from the core.

Both clients also expose the read-only bonded-device query defined below.

## Device list

A fresh installation shows an empty state and an Add device link. It does not
create a settings file or a default connection.

Each configured row shows these items:

- The transport name.
- `host:port` for TCP.
- The address for Bluetooth SPP.
- A custom service UUID when the configuration contains one.
- The current bonded-device name on Android when the platform query returns a
  matching address and a name.
- An enabled switch.
- A link to the edit route.

The bonded-device name is a current platform label. It is not stored in core
settings. A row falls back to its Bluetooth address when no current name is
available.

A standard SPP configuration does not show service UUID text. A custom service
UUID appears as read-only information.

The enabled switch sends the existing enable or disable command. The control
is disabled while its command is pending. The topic confirms the resulting
state. A rejected command restores the published value and shows an error near
the control.

The row does not show connection state, capability chips, selected-source
state, or source priority. The current topic does not contain those values.

## Add and edit flow

The add route starts with a transport choice. Android offers TCP and Bluetooth
SPP. Desktop offers TCP only.

The edit route uses the same form. Android permits a device to change between
TCP and Bluetooth SPP. The existing edit command keeps the device ID and list
position. Desktop permits TCP editing and TCP-to-TCP changes only. A saved
Bluetooth row remains visible on desktop, but its address and service UUID are
not editable there.

Save is disabled while a command is pending. A successful add or edit returns
to `/devices`. A rejected command keeps the form open and shows an inline
error.

The edit route contains a Delete button. Delete opens a confirmation dialog.
The dialog names the device by its visible endpoint. A successful deletion
returns to `/devices`. A failed deletion keeps the route open and shows the
error.

## TCP form

The TCP form contains a host field and a port field.

The host must contain non-whitespace text. The form preserves host names and
IP address text as entered after it removes leading and trailing whitespace.

The port must be an integer from 1 through 65535. The form does not submit
until both fields are valid.

## Bonded Bluetooth query

The configured-device topic cannot supply current Android bond information.
The Tauri host therefore adds one read-only
`bonded_bluetooth_devices` command. This command does not send a core input.

The shared result distinguishes these states:

- `unsupported` means that the platform has no supported Bluetooth SPP
  configuration path.
- `permissionDenied` means that Android has not granted Nearby Devices access.
- `disabled` means that Android Bluetooth is off.
- `available` contains the current bonded devices.

Each available device contains an address and an optional display name.

The Rust mobile bridge asks the Kotlin plugin for the result. Kotlin checks the
existing Nearby Devices permission and reads the bonded set from the platform
Bluetooth adapter. It does not start discovery, request scan permission, pair
a device, or enable Bluetooth.

The desktop plugin returns `unsupported`. The fake client can return any state
so browser tests and stories can show every form state.

## Bluetooth form

The Android Bluetooth form loads the bonded-device result when the route
opens. It provides a Refresh button because bonding can change in Android
settings while Updraft remains open.

The available state shows a picker. Each option shows the display name when
available and always shows the address. An empty list tells the user to pair a
device in Android settings and then refresh. Permission denial and disabled
Bluetooth show distinct messages. This slice does not open Android settings.

New Bluetooth devices must use one address from the bonded-device result.
There is no manual address field.

An existing configuration can refer to an address that is no longer bonded.
The edit form keeps that address as the current value and marks it as not
currently bonded. The user can preserve it or select another bonded device.

New Bluetooth devices omit `serviceUuid`. The existing core default supplies
`00001101-0000-1000-8000-00805F9B34FB`. The form does not show this value.

An existing configuration can contain a custom `serviceUuid`. The edit form
shows the custom value as read-only information. An address edit sends the
same custom value in the replacement connection specification. The form does
not let the user change or remove the value.

## Error behavior

Form validation errors identify the field that must change. A command error
appears near the action that failed. The screen does not log a successful
mutation before the command completes.

An unknown edit or delete ID becomes `Device not found`. A stopped driver
produces a general device-action error. The UI does not guess whether a
background settings write or connection attempt succeeded.

## Automated tests

Implementation follows red-green-refactor.

Frontend store and client tests cover these behaviors:

- The first external-device topic marks the store as initialized.
- A complete topic replaces the ordered list.
- Other topics do not change the device store.
- Fake-client mutations emit complete authoritative topics.
- Mutation rejections do not change shared state.

Frontend component tests cover these behaviors:

- The fresh-install empty state.
- TCP and Bluetooth row summaries.
- Enable and disable commands.
- TCP host and port validation.
- Standard UUID creation without service UUID text.
- Read-only display and preservation of an existing custom UUID.
- Transport changes during Android editing.
- Desktop Bluetooth restrictions.
- An unbonded current address.
- Unknown edit-route IDs.
- Delete confirmation.
- Pending and failed commands.

Storybook presents the empty list, mixed enabled and disabled rows, TCP and
Bluetooth forms, an existing custom UUID, unsupported and denied Bluetooth
states, an unbonded current address, and command errors. Stories show component
states. They do not repeat behavior tests.

Rust and Kotlin tests cover the bonded-device query. They cover platform
support, permission denial, disabled Bluetooth, a missing adapter, an empty
bonded set, optional names, and returned addresses. Tauri tests use the real IPC
deserializer for the new concrete command.

## Manual acceptance

The Android acceptance test starts with no configured external devices. The
tester adds a bonded SPP simulator. The stored configuration must omit
`serviceUuid`. The tester confirms that the existing transport connects. The
tester disables and enables the entry, edits it, and deletes it. The tester
restarts Updraft after each stored change and confirms that the current
configuration returns.

The tester then loads a settings file that contains a custom service UUID. The
Devices screen must show the custom value without an edit control. An address
edit must preserve the custom value.

Desktop acceptance adds and edits a TCP endpoint. It also confirms that a
saved Bluetooth row remains manageable but cannot open Bluetooth configuration
fields.
