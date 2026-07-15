# Devices Screen

The Devices screen presents the source-selection and merging model defined in
[devices.md](../devices.md). The ordered rows themselves show priority,
capabilities, and which data the app currently uses. A separate signal summary
is unnecessary.

## Device List

External devices form one user-ordered list. Each row shows the device name,
connection state, connection kind and endpoint, and capability chips such as
GPS, Pressure, Vario, or FLARM. A visible reorder handle changes the priority.

The connection summary uses transport-specific wording that identifies where
the data comes from. Examples include `Bluetooth SPP to LX9000`,
`Bluetooth LE to Foo`, `TCP client to 1.2.3.4:4353`,
`TCP server on port 4353`, and `UDP listener on port 10110`. It remains visible
while disconnected so similar device configurations remain distinguishable.

For a selected signal such as GPS or pressure altitude, the app scans the list
from top to bottom and uses the first source with fresh, valid data. The choice
is independent for every signal. One device may provide GPS while a lower device
provides pressure altitude.

Capability chips communicate their current role:

- A filled, highlighted chip means the app currently uses that capability.
- A plain, outlined chip means the device provides the capability but the app is
  not currently using it.

Only capabilities provided by the device are shown. The UI does not add
placeholder chips for capabilities the device does not have. The row's
connection state and its details distinguish a healthy fallback from a disabled
or stale source.

Shape and text accompany color so the state remains clear in sunlight and for
users with color-vision differences. A stale or disconnected device retains its
position. When it provides fresh data again, it automatically regains its
configured priority.

For merged information such as traffic, every contributing device chip is
highlighted. The merged result retains source provenance for details and
diagnostics.

For example:

- **Device A:** `[GPS]` `[FLARM]`
- **Device B:** `GPS` `[Pressure]`

The brackets represent the filled highlight in this text example.

## Internal Sensors

On platforms that expose at least one supported built-in sensor, **Internal
sensors** appears as a configurable row fixed below every external device. It
may provide GPS, pressure, acceleration, rotation, or other platform sensors. It
has no reorder handle because its selected signals are always the lowest-priority
fallback. If the platform exposes no supported internal sensors, as is expected
on most desktop systems, the row is not shown.

The row uses the same chip states as an external device. An internal capability
is highlighted when it is actively used because no higher source provides valid
data. Tapping the row opens details and configuration for individual sensors,
including enabled state, current value, freshness, quality, permissions,
calibration when supported, and diagnostics.

## Add Device

Discovered but unconfigured external devices appear in a separate **Add device**
flow. They do not affect priority until the user adds them to the configured
list.
