# Airspace

Status: Current behavior

Updraft imports one local OpenAir source. It converts the complete source into
one canonical polygon dataset. The core owns the active dataset and publishes
its status. The Tauri shell owns file selection, storage, and GeoJSON delivery.

## Import

The platform file picker supplies source bytes and a display name. The
`updraft_airspace` crate parses all OpenAir records before it changes active
state.

The importer converts supported points, circles, arcs, and polygon segments to
polygon exterior rings. Curves use a maximum one-metre chord error. It rejects
unsupported or invalid geometry instead of publishing a partial dataset.

Each imported airspace receives an `AirspaceId` from its zero-based position in
the parsed dataset. The ID is stable only for that dataset. It is not durable
across source replacement.

## Canonical model

The canonical `Airspace` stores:

- dataset-local ID and optional name
- ICAO class, airspace type, and optional activity
- lower and upper limits with optional minimum and maximum limits
- activation flags, dates, operating hours, and remarks
- source country values
- frequencies and transponder settings
- one polygon exterior ring

Altitude limits distinguish ground, MSL altitude, AGL height, flight level, and
unlimited. Physical values use typed units.

OpenAir class and type values map to the OpenAIP numeric model where possible.
An unsupported or absent type becomes `Other`. OpenAir currently supplies no
country value, activity, activation dates, or operating hours.

Country values are unvalidated source text. A later OpenAIP importer must keep
one scalar or ordered array without applying a country registry.

## Core state

The airspace topic has three states:

- `none` means that no source is configured.
- `active` includes the source display name, airspace count, and generation.
- `unavailable` includes the display name and a safe load-error category.

The core stores an active dataset behind an immutable shared reference. A
replacement, removal, or later activation advances the generation. Reusing the
same dataset with a different display name does not advance it.

The core query returns a shared dataset snapshot. It does not clone or serialize
the complete dataset on the driver task.

## Storage

The Tauri shell stores the original OpenAir bytes as `airspace.txt` in the
application data directory. Optional metadata stores the source display name.
The original source remains the durable authority. Updraft parses it again at
startup.

Import prepares the replacement files before it activates the new core
dataset. A failed write or activation keeps or restores the previous durable
source. Removing the source removes the stored files and clears the core state.

A missing source starts with `none`. A stored source that cannot be read or
parsed starts as `unavailable`. The technical error is logged. The frontend
receives only the safe error category.

## GeoJSON resource

The Tauri shell serves the current snapshot from
`updraft://localhost/airspace.geojson`. The response uses
`application/geo+json`, disables caching, and returns an empty FeatureCollection
when no dataset is active.

Each feature uses its numeric airspace ID as the top-level GeoJSON ID. The
geometry is the canonical polygon. Properties use OpenAIP-compatible names and
numeric codes. Optional values are absent instead of null. An unlimited limit
uses the Updraft extension `{ "unlimited": true }`.

The projection includes all canonical metadata needed by airspace details. It
does not include OpenAIP service, audit, or ownership fields.

## Map and details

MapLibre renders one invisible hit layer, one fill layer, and one outline layer.
Type has styling priority. ICAO class supplies the controlled-airspace fallback.
The source generation changes the resource URL after a dataset replacement.

A normal map tap can select rendered airspace features. The nearby page keeps
the selected feature sequence for the mounted route. `/airspaces/[id]` displays
one feature from the current rendered dataset. A direct or stale ID can produce
an explicit not-found state.

## Excluded behavior

The current contract does not include multiple active sources, an OpenAIP
importer, vertical or class filters, durable IDs, downloads, labels, airspace
warnings, or editing.
