# Configurable display units

## Context

The [persisted locale setting](2026-07-28-settings-design.md) established the
core as the authority for application settings. The Tauri shell stores complete
settings snapshots. The frontend receives the active settings through
`Topic::Settings`.

The debug overlay currently shows MSL altitude in meters and ground speed in
kilometers per hour. The
[FLARM traffic map](2026-07-30-flarm-traffic-map-design.md) shows each available
MSL altitude in meters. These fixed units do not follow a user preference.

This design adds application-wide display units. It extends the existing
settings model and persistence flow. It also replaces the fixed traffic
altitude label that the FLARM traffic map design defines. Instrument and
traffic values continue to use canonical SI units.

## Scope

This slice adds settings for:

- Altitude.
- Distance.
- Speed.
- Vertical speed.

The settings screen exposes all four settings. The debug overlay consumes the
altitude and speed settings. The traffic map consumes the altitude setting.
The distance and vertical-speed settings have no display consumer in this
slice.

This slice does not add:

- Unit presets.
- Automatic defaults from the locale or device.
- Coordinate, temperature, pressure, mass, or volume units.
- New distance or vertical-speed displays.
- Unit-dependent flight calculations or sensor protocol values.
- A migration from browser storage.

## Unit model

The core adds four unit enums and one complete unit-settings value:

```rust
pub enum AltitudeUnit {
    Meters,
    Feet,
}

pub enum DistanceUnit {
    Kilometers,
    Miles,
    NauticalMiles,
}

pub enum SpeedUnit {
    KilometersPerHour,
    Knots,
    MilesPerHour,
}

pub enum VerticalSpeedUnit {
    MetersPerSecond,
    Knots,
    FeetPerMinute,
}

pub struct UnitSettings {
    pub altitude: AltitudeUnit,
    pub distance: DistanceUnit,
    pub speed: SpeedUnit,
    pub vertical_speed: VerticalSpeedUnit,
}
```

The enums use these serialized values:

- Altitude uses `m` and `ft`.
- Distance uses `km`, `mi`, and `nm`.
- Speed uses `km/h`, `kt`, and `mph`.
- Vertical speed uses `m/s`, `kt`, and `ft/min`.

`UnitSettings::default()` returns:

```rust
UnitSettings {
    altitude: AltitudeUnit::Meters,
    distance: DistanceUnit::Kilometers,
    speed: SpeedUnit::KilometersPerHour,
    vertical_speed: VerticalSpeedUnit::MetersPerSecond,
}
```

These defaults preserve the current metric behavior. All combinations of the
four selections are valid. The model does not contain a preset or unit-system
enum.

`Settings` contains the complete value:

```rust
pub struct Settings {
    pub locale: Option<Locale>,

    #[serde(default)]
    pub units: UnitSettings,
}
```

`UnitSettings` also uses Serde defaults for missing fields. The core generates
TypeScript types for the four enums and `UnitSettings`.

## Storage compatibility

The existing `SettingsSnapshot` continues to flatten `Settings` beside the
external-device list. A complete stored value has this shape:

```json
{
  "locale": "de",
  "units": {
    "altitude": "ft",
    "distance": "nm",
    "speed": "kt",
    "verticalSpeed": "ft/min"
  },
  "externalDevices": []
}
```

An existing file without `units` keeps its locale and external devices. The
loader supplies the complete metric unit default. A missing field inside
`units` also receives its metric default.

An unknown unit string makes the stored snapshot malformed. The shell uses its
existing malformed-file behavior. It logs a warning, loads the complete
default snapshot, and does not modify the file.

The shell continues to write complete snapshots through the existing FIFO
background worker. It writes `settings.json` with a temporary neighboring file
and an atomic replacement. A write failure logs a warning. It does not roll
back the active unit settings.

## Core and IPC flow

The boundary adds these values:

- `SetUnits`, with one complete `UnitSettings` value.
- A Tauri `set_units` command.
- `UpdraftClient.setUnits(units)`.

`SetUnits` returns `()`. The core compares the supplied value with its active
unit settings. An equal value is a successful no-op and produces no effects.

A different value replaces the complete active unit settings. The core then
emits the complete settings topic and the complete settings snapshot for
persistence. The shell dispatches both effects before the command completes.
The command does not wait for the background writer to finish.

`Topic::Settings` remains the only authoritative shared-state update path. A
command response does not update a frontend store. The shell does not keep a
second mutable unit-settings value.

The generated TypeScript unit enums replace the handwritten unit aliases in
`frontend/src/lib/units.ts`. The existing numeric conversion functions keep
their current responsibility. They accept canonical SI values and return
converted numbers. They do not create display strings.

## Settings screen

The settings screen keeps the existing Language fieldset. It adds a sibling
Units fieldset with four labeled native `select` controls:

- Altitude.
- Distance.
- Speed.
- Vertical speed.

The labels use the existing localization system. Each option displays its unit
symbol. Unit symbols do not use translated forms.

The settings route keeps the latest intended `UnitSettings` value. It passes
that optimistic value to the unit component. Each selection replaces one field
and immediately sends one complete value through `setUnits()`.

The frontend does not serialize `setUnits()` calls. The core driver applies the
received inputs through its existing input queue. An older command completion
does not clear a newer optimistic value.

Settings topics remain authoritative. The route clears its optimistic value
when the corresponding command completes, after the shell has dispatched its
effects. The component then receives the published settings. If `setUnits()`
fails, the route also clears that optimistic value and logs the error. This
slice does not add settings error or persistence status UI.

The fake client owns the same complete unit-settings value. `setUnits()` emits
a settings topic when the value changes. An equal value remains a no-op.

## Presentation data flow

The frontend settings store applies each complete settings topic. The layout
passes the active `UnitSettings` through the flight-view and map component
tree. Display components receive explicit presentation settings as props.

The core instrument topic keeps `altitudeMslMeters` and
`groundSpeedMetersPerSecond`. The traffic topic keeps
`PublishedTrafficTarget.altitudeMslMeters`. The instruments and traffic stores
do not contain converted values or display strings.

Unit changes do not modify instrument or traffic topics. They cause the
frontend to format the existing canonical values again.

## Debug overlay

The debug overlay uses the configured altitude unit for MSL altitude. It
converts the canonical meter value and shows no decimal places. The label uses
`m` or `ft`.

The overlay uses the configured speed unit for ground speed. It converts the
canonical meters-per-second value and shows one decimal place. The label uses
`km/h`, `kt`, or `mph`.

Missing altitude and speed values continue to show `–`. Zoom, map center, and
position formatting do not change. The distance and vertical-speed settings do
not affect this overlay.

## Traffic GeoJSON projection

The traffic store keeps each published altitude in meters. The GeoJSON
projection receives the selected `AltitudeUnit`:

```ts
trafficFeature(target, altitudeUnit)
```

The projection converts and formats an available altitude. It rounds the
converted value to a whole unit and adds the unit symbol. It stores the result
in one `altitudeLabel` property. A target without altitude uses `null`.

The GeoJSON feature does not keep `altitudeMslMeters`. No current map filter or
style consumes a numeric altitude property.

Both traffic symbol layers read `altitudeLabel` for their text. They show no
text when the property is `null`. The layers do not perform unit conversion or
number formatting in MapLibre expressions.

An altitude-unit change rebuilds the complete GeoJSON source from the current
traffic store. The rebuild uses the existing serialized source-update queue.
Traffic snapshots, traffic deltas, unit rebuilds, and failure recovery cannot
apply source data out of order.

Each later traffic delta uses the active altitude unit when it creates its
features. A distance, speed, or vertical-speed change does not rebuild the
traffic source.

This section supersedes the fixed `altitudeMslMeters` GeoJSON property and the
fixed meter label in the FLARM traffic map design. It does not change that
design's core traffic model or wire protocol.

## Testing

Core tests cover:

- The complete metric default.
- A changed `SetUnits` input and its exact topic and persistence effects.
- An equal `SetUnits` input as a successful no-op.
- Current unit settings in the initial settings topic.

Shell tests cover:

- An existing locale and device file without `units`.
- Missing fields in a stored `units` object.
- A complete non-default value through write and reload.
- The exact stored JSON field and unit names.
- An unknown unit value through the existing malformed-file path.
- Real Tauri IPC deserialization for `set_units`.

Frontend tests cover:

- Four labeled selects with the active choices.
- One selection reporting one complete `UnitSettings` value.
- Two rapid optimistic selections reporting complete values without frontend
  serialization.
- Fake-client topic publication and equal-value no-op behavior.
- Debug-overlay altitude and speed formatting with non-default units.
- Meter and foot traffic labels.
- A missing traffic altitude.
- Existing traffic labels after an altitude-unit source rebuild.
- Normal traffic deltas after a unit change.

Existing conversion tests remain responsible for numeric conversion formulas.
Higher-level tests do not repeat every conversion pair.

## Acceptance

A real Tauri build must satisfy these checks:

1. Change each unit and confirm that its select changes immediately.
2. Open the debug overlay and confirm the selected altitude and speed units.
3. Confirm that existing and new traffic labels use the selected altitude
   unit.
4. Restart the application and confirm that all four selections remain active.
5. Confirm that `settings.json` contains the complete unit-settings object.

Browser-only development must use the same settings-topic interaction through
the fake client.
