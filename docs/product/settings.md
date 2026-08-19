# Settings

Status: Current behavior

Updraft stores application-wide settings in the Rust core. The Tauri shell
loads and persists one complete settings snapshot. The frontend presents the
current settings topic and sends typed commands for changes.

## Navigation

`/settings` is a category menu. It links to these focused pages:

- `/settings/language`
- `/settings/units`
- `/settings/airspace`
- `/settings/devices`
- `/settings/about`

Each category page links back to `/settings`. Device creation and editing use
routes below `/settings/devices`.

`/settings` also contains a quit action. It asks for confirmation, stops the
platform session, and closes the application.

The root layout keeps the Flight View and map mounted while a Settings route is
open. A return to the Flight View therefore keeps temporary map state.

## Ownership and updates

The core owns the active locale, display units, and external-device
configuration. The `Settings` topic contains the active locale and units. The
`ExternalDevices` topic publishes the separate device projection.

The frontend can show an optimistic control value while a command is pending.
The next topic remains authoritative. A rejected command clears the optimistic
value and keeps the last published setting.

An equal setting change is a no-op. A successful change publishes the updated
topic and requests persistence of the complete snapshot.

## Persistence

The Tauri shell stores `settings.json` in the application configuration
directory. The file contains the locale, unit selections, and external-device
configuration.

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
