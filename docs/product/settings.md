# Settings

Status: Current behavior

Updraft stores application-wide settings in the Rust core. The Tauri shell
loads and persists one complete settings snapshot. The frontend presents the
current settings topic and sends typed commands for changes.

## Navigation

The Flight View Menu control opens the Main Menu at `/menu`. It links to these
pages:

- `/navigation`
- `/flight-controls`
- `/settings`
- `/about`

The Main Menu also contains a quit action. It asks for confirmation, stops the
platform session, and closes the application.

On Android, the session notification also contains a quit action. It quits
without confirmation because Android usually shows notification actions only
after the pilot expands the notification. On Android 12 and later, the action
requires an unlocked device.

`/settings` is a category menu for saved settings. It links to these focused
pages:

- `/settings/language`
- `/settings/units`
- `/settings/map`
- `/settings/glide`
- `/settings/vario`
- `/settings/traffic`
- `/settings/data`
- `/settings/devices`

Each category page links back to `/settings`. The Flight controls and About
pages and `/settings` link back to `/menu`. Navigation links back to the
screen that opened it, the Main Menu or the Flight View. Its detail pages keep
this origin. Device creation and editing use routes below `/settings/devices`.

The root layout keeps the Flight View and map mounted while the Main Menu or a
Settings route is open. A return to the Flight View therefore keeps temporary map state.

The Flight controls page contains the session controls. Its MacCready control
uses the selected vertical-speed unit. MC defaults to 0.0 m/s and accepts finite,
nonnegative values. It remains active during navigation and resets when Updraft
restarts. The core publishes it through the separate `GlidePerformance` topic.
Settings writes exclude these session controls.

Bugs specifies the percentage of clean performance lost. Zero means clean,
and 10% means a 10% loss. Fractional percentages are accepted. Values must be
at least zero and less than 100%. Bugs resets to zero on restart. It adjusts
the active polar and updates netto immediately. Changing the selected polar
retains the current bugs value.

Ballast specifies litres of water added to the selected polar's reference mass,
at 1 kg per litre. It accepts finite, nonnegative values and resets to zero on
restart. It adjusts the active polar and netto. Changing the selected polar
retains the current ballast value. MC, bugs, and ballast changes also request
new waypoint arrival calculations.

## Ownership and updates

The core owns the active locale, display units, hillshade direction, trail
colour mode, trail length, trail drift compensation, ownship dead reckoning,
glide polar, arrival reserve, and external-device configuration. The `Settings`
topic contains the active locale, units, hillshade direction, trail colour mode,
trail length, trail drift compensation, ownship dead reckoning, polar, and
arrival reserve. The `ExternalDevices`
topic publishes the separate device projection.

The frontend can show an optimistic control value while a command is pending.
The next topic remains authoritative. A rejected command clears the optimistic
value and keeps the last published setting.

An equal setting change is a no-op. A successful change publishes the updated
topic and requests persistence of the complete snapshot.

## Persistence

The Tauri shell stores `settings.json` in the application configuration
directory. The file contains the locale, unit selections, hillshade direction,
trail colour mode, trail length, trail drift compensation, ownship dead
reckoning, glide polar, arrival reserve, and external-device configuration.

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

## Map settings

The Map page selects the hillshade lighting direction. Fixed is the default. It
uses MapLibre's conventional viewport-relative direction of 335 degrees. Wind
direction uses the reported meteorological direction as a map-relative light
source. Sun direction uses the calculated solar azimuth at the ownship position
as a map-relative light source. Wind and sun modes continue to use stale data.
They use the fixed direction when their selected data is unavailable.

The selection is saved across restarts. A settings file without this field uses
the fixed default. A pending command disables the control. Failure restores the
published value and shows an error.

The Map page also selects the trail colour: Fixed colour, Altitude, AGL, Vario,
Netto, or Relative vario. Relative vario is the default. Vario, netto, and
relative vario use one fixed scale from -5 m/s to +5 m/s, with zero at the
centre. Values outside the scale use the end colours. Altitude uses the MSL
altitude that the altitude infobox shows, on a scale from 0 m to the maximum
altitude of the flight recording. AGL uses a scale from 0 m to the maximum AGL
altitude of the recording. A new maximum changes the colours of the older
segments. Vario uses the vertical speed of a sample without a vario, as the
Vario infobox does. Netto and relative vario use Estimated netto and Estimated
relative vario in the same way. A segment without a value for the selected mode is grey. A change
recolours the whole trail. The selection is saved across restarts, and a
settings file without this field uses the default.

The Map page also sets the trail length in whole minutes from 0 to 1440. The
presets are Off, 15 min, 60 min, 3 h, 5 h, and 10 h. The default is 60 min.
Off, or 0 min, turns the trail off. The length counts back from the UTC of the
newest sample. A shorter length removes the older segments. A longer length
reads the trail again from the flight recording. The control rejects other
values. A pending command disables the control. Failure restores the published
value and shows an error. The length is saved across restarts, and a settings
file without this field uses the default.

Trail drift compensation moves each trail point downwind by its age multiplied
by the current wind estimate. A thermal then shows as a column relative to the
air mass. The age counts back from the UTC of the newest sample. A stale wind
estimate is used. Without a wind estimate, the trail has no drift. The trail
keeps its wind until the estimate changes by more than 0.5 m/s. The Map page
turns drift compensation on or off. It is off by default. A pending command
disables the control. Failure restores the published value and shows an error.
The selection is saved across restarts, and a settings file without this field
uses the default.

Ownship dead reckoning moves the ownship symbol between GPS fixes. See
[Flight View](flight-view.md#ownship-dead-reckoning). The Map page turns it on
or off. It is on by default. A pending command disables the control. Failure
restores the published value and shows an error. The selection is saved across
restarts, and a settings file without this field uses the default.

## Glide settings

The Glide page selects a polar from the built-in catalog. The default is the
15 m LS8, listed as `LS 8`. The selected catalog name is saved across restarts.
A settings file without a polar uses the default. An unknown polar name makes
the settings snapshot invalid.

Sensor fusion uses the selected polar to calculate netto vario. Direct-glide
arrival calculations use the same polar with current bugs and ballast. A polar change
updates the derived instruments immediately when the required inputs are available.

The arrival reserve defaults to 200 m and is saved across restarts. The control
uses the selected altitude unit. It displays whole units and accepts fractional
values. Opening the page does not change the stored precision. The core stores
metres and accepts only finite, nonnegative values. A settings file without a
reserve uses the default.
Polar and reserve changes request new waypoint arrival calculations.

## Climb averaging

The persisted climb averaging method defaults to a smoothed 20-second average.
This method smooths height with a 5-second time constant before averaging.
The alternatives are normalized EMA with a 10-second time constant and
20-second and 30-second averages. Missing stored values use the default. The map updates its labels when the setting changes,
using estimates already collected by the core. The Vario page selects the method
with four radio options. A pending command disables the control. A failed command
restores the authoritative setting and shows an error. The setting is shared with
future ownship climb display.

Energy compensation is a separate shared setting. It defaults to enabled and
applies to all four methods. Disabled compensation uses altitude changes only.
Changing the setting clears existing climb estimates and restarts their histories.
The Vario page offers enabled and disabled options with command error feedback.

## Traffic correction

The Traffic page offers enabled and disabled options for experimental FLARM
position correction. It defaults to enabled and is saved across restarts.
Existing settings files enable it when the field is absent. A pending command
disables the control. Failure restores the published value and shows an error.
See [Traffic](traffic.md) for the reference model and fallback behavior.

## Installed data

The Data page lists imported files in Airspace and Waypoints groups and installed
basemap and terrain files in Basemap and Terrain groups. The group order is
Airspace, Waypoints, Basemap, then Terrain. Empty groups are hidden. Display names
are sorted alphabetically within each group. Basemaps and terrain use country/region names
and show download date and installed size. Rows omit source prefixes.
Active imported files show feature counts and waypoint warning counts. Disabled files show their disabled state.
Unavailable imported files show a read, parse, or geometry error.
The page updates when source status changes. Source catalogs and their status
are separate from the settings snapshot.

Select a file to open its details. The dialog shows source type, activation
status, feature count, and current errors or waypoint warnings. Close, Escape,
Back, and a tap outside dismiss the dialog. Back keeps the library open.

Basemap and terrain details show an Enabled control and a load error when an enabled file
is unavailable. They also show source, installed size, and download time.
Available updates add Update. Unavailable catalogued files offer Download
again.
The app maintains basemap and terrain status across navigation.
Loading and subscription failure messages are distinct from an empty inventory.

The Enabled control changes immediately and stays interactive while saving.
Activation changes run sequentially and continue after leaving Settings. The
latest choice stays visible until the command and its source status arrive.
A failed final change returns to the confirmed state and shows an error in
the library and file details. An enabled file can have a parsing error.
Disabling hides its counts and diagnostics until it is enabled again.
Basemap activation and removal refresh tiles without changing the map position or zoom.
Terrain activation and removal refresh tiles, metadata, and credits together.
Each operation rechecks tile-size compatibility across enabled files and
preserves the map camera.

Remove from device closes the details and opens a confirmation. Cancel returns
to the library. A removal failure stays in the confirmation with an error and
allows another attempt. Removal is unavailable while an activation change is pending.

Add data opens the country download catalog. Country pages separate Basemap and
Terrain selections. Both use one shared download queue. Import custom file opens a
single-file picker for OpenAir (`.txt`) and CUP (`.cup`) files from that catalog.
The action appears in the footer through 544px and in the header above it.
The library confirms replacement of an existing filename in the same dataset
type. Cancel keeps the installed file. Import enables the selected file even
when parsing fails. The library shows parsing errors in the row and details.

After import, the library scrolls only as far as needed to reveal the file.
It stays open and does not open details automatically. A visible file does not
change the scroll position. Read failures do not replace installed files.

The Settings Data row shows the available basemap and terrain update count. The library reports
failed checks with the last successful check time and Retry. Available updates
have a notice that opens Updates. Both views share file rows and transfer
controls. Update all skips active and queued files. See [Basemap](basemap.md)
for catalog, installation, cancellation, and update behavior.

## About

The About page shows the source repository, build commit, build time, and the
attributions reported by the active map style. Attribution text is rendered as
text and links. The page does not render source-provided HTML.

## Planned settings

The current settings model does not include named pilot, aircraft, device, or
display profiles. It also does not include a persisted theme or map-orientation
selection. These functions remain product-scope items until they have an
accepted focused design.
