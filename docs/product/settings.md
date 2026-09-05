# Settings

Status: Current behavior

Updraft stores application-wide settings in the Rust core. The Tauri shell
loads and persists one complete settings snapshot. The frontend presents the
current settings topic and sends typed commands for changes.

## Navigation

`/settings` is a category menu. It links to these focused pages:

- `/settings/language`
- `/settings/units`
- `/settings/glide`
- `/settings/airspace`
- `/settings/devices`
- `/settings/about`

Each category page links back to `/settings`. Device creation and editing use
routes below `/settings/devices`.

`/settings` also contains a quit action. It asks for confirmation, stops the
platform session, and closes the application.

The root layout keeps the Flight View and map mounted while a Settings route is
open. A return to the Flight View therefore keeps temporary map state.

The Settings index also contains a Flight controls panel. Its MacCready control
uses the selected vertical-speed unit. MC defaults to 0.0 m/s and accepts finite,
nonnegative values. It remains active during navigation and resets when Updraft
restarts. The core publishes it through the separate `GlidePerformance` topic.
Settings writes exclude these session controls.

## Ownership and updates

The core owns the active locale, display units, glide polar, arrival reserve,
and external-device configuration. The `Settings` topic contains the active
locale, units, polar, and arrival reserve. The `ExternalDevices` topic publishes
the separate device projection.

The frontend can show an optimistic control value while a command is pending.
The next topic remains authoritative. A rejected command clears the optimistic
value and keeps the last published setting.

An equal setting change is a no-op. A successful change publishes the updated
topic and requests persistence of the complete snapshot.

## Persistence

The Tauri shell stores `settings.json` in the application configuration
directory. The file contains the locale, unit selections, glide polar, arrival
reserve, and external-device configuration.

A missing file loads defaults and remains absent until a setting changes. A
malformed or unreadable file produces a warning and loads defaults. Updraft
does not overwrite that file during the failed load.

Writes use a background FIFO writer. Each write creates a temporary file in the
same directory and replaces `settings.json`. A write failure produces a
warning. It does not roll back the active core setting.

Missing unit fields use metric defaults. A stored Bluetooth device without a
service UUID uses the standard SPP UUID. Other invalid stored values make the
complete snapshot invalid and load defaults.

## Language

The stored locale is optional. Updraft currently accepts `en` and `de`. The
frontend uses the stored value when present. Otherwise, it uses the locale that
the localization runtime resolves.

Changing the locale updates the core setting. The frontend applies the new
locale from the published topic.

## Display units

Unit settings are independent selections for:

- altitude: metres or feet
- distance: kilometres, miles, or nautical miles
- horizontal speed: kilometres per hour, knots, or miles per hour
- vertical speed: metres per second, knots, or feet per minute

The core and protocol retain canonical SI values. Frontend presentation code
converts and formats values with the active unit settings.

## Glide settings

The Glide page selects a polar from the built-in catalog. The default is the
15 m LS8, listed as `LS 8`. The selected catalog name is saved across restarts.
A settings file without a polar uses the default. An unknown polar name makes
the settings snapshot invalid.

Sensor fusion uses the selected polar to calculate netto vario. A polar change
updates the derived instruments immediately when the required inputs are available.

The arrival reserve defaults to 200 m and is saved across restarts. The control
uses the selected altitude unit. It displays whole units and accepts fractional
values. Opening the page does not change the stored precision. The core stores
metres and accepts only finite, nonnegative values. A settings file without a
reserve uses the default.

## Airspace source

The Airspace page imports, replaces, or removes one local OpenAir source. The
airspace dataset and source status do not live in the settings snapshot. The
page is part of Settings because it manages application data.

## About

The About page shows the source repository, build commit, build time, and the
attributions reported by the active map style. Attribution text is rendered as
text and links. The page does not render source-provided HTML.

## Planned settings

The current settings model does not include named pilot, aircraft, device, or
display profiles. It also does not include a persisted theme or map-orientation
selection. These functions remain product-scope items until they have an
accepted focused design.
