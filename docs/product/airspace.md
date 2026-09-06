# Airspace

Status: Current behavior

Updraft imports local OpenAir files as independent sources. Each source has
one canonical polygon dataset. The core owns the catalog and publishes its
status. The Tauri shell owns file selection, storage, and GeoJSON delivery.
All valid sources are active together. Duplicate airspaces remain separate.

## Import

The platform file picker supplies source bytes and a display name. The
`updraft_airspace` crate parses all OpenAir records before it changes active
state. A file must have a display filename. An import adds that filename or
replaces only the source with the exact same filename. Other sources remain
unchanged. Settings lists each source and requires confirmation before removal.

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

OpenAir v2 class and type values map to the OpenAIP numeric model where
possible. The importer converts recognized legacy class values to an
unclassified airspace with the matching type. It also converts the nonstandard
legacy `AC GSEC` form to a gliding sector. A conflicting class and type or an
unsupported class rejects the complete source. An unsupported or absent type
becomes `Other`. OpenAir currently supplies no country value, activity,
activation dates, or operating hours.

Country values are unvalidated source text. A later OpenAIP importer must keep
one scalar or ordered array without applying a country registry.

## Core state

The airspace topic contains a catalog generation and source statuses in
filename order. An empty source list means that no source is configured.
Each source is either:

- `active`, with its filename and airspace count.
- `unavailable`, with its filename and a safe load-error category.

The core stores the catalog and datasets behind immutable shared references.
Each catalog replacement advances the process-local generation. The query
returns the catalog and its generation from one snapshot. It does not clone or
serialize geometry on the driver task.

## Storage

The Tauri shell stores the original bytes of each source in the application
data directory under `airspaces/`. Encoded filenames retain exact source names,
including case differences. Long encoded names use subdirectories. The original
bytes remain authoritative. Updraft parses each file again at startup.

Import validates the complete file before it changes storage. Import and removal
prepare a catalog replacement from the current snapshot. A failed write keeps
the previous source. If catalog activation fails, the command restores the
previous stored file. Other sources remain unchanged.

A stored file that cannot be read or parsed appears as `unavailable`. Other
sources remain usable. An unreadable source subdirectory is logged and skipped.
Its filenames cannot be recovered until the directory becomes readable.
The frontend receives safe error categories instead of technical error text.

The earlier `airspace.txt` and `airspace.json` layout is not loaded or migrated.
Users must import the airspace file again. Updraft leaves those old files untouched.

## GeoJSON resource

The Tauri shell serves the current snapshot from
`updraft://localhost/airspace.geojson`. The response uses
`application/geo+json`, disables caching, and returns an empty FeatureCollection
when no dataset is active.

Each feature uses `generation:source-index:record-id` as its GeoJSON ID.
The source index follows catalog filename order. The properties include the
same `id` and the `sourceName`. MapLibre promotes the `id` property so polygon
queries retain the string ID. IDs do not persist across application restarts.
The geometry is the canonical polygon. Properties use OpenAIP-compatible names and
numeric codes. Optional values are absent instead of null. An unlimited limit
uses the Updraft extension `{ "unlimited": true }`.

The projection includes all canonical metadata needed by airspace details. It
does not include OpenAIP service, audit, or ownership fields.

## Map and details

MapLibre renders three layers: an independent invisible hit layer, one inner-band
layer, and one boundary layer. Each airspace receives at most one inner band.
Airspace type selects the boundary color, dash pattern, and band color.
Control zones, gliding sectors, radio mandatory zones,
airport traffic zones, and traffic information zones and areas receive bands.
Classes A through D and restricted and protected areas also receive bands.
The bands grow linearly from zero width at zoom 6 to full width at zoom 8.
Gliding-sector bands use ten pixels and 25% opacity. Other bands use seven pixels
and 20% opacity.
FIS sectors use green dotted boundaries. Military training areas use slate
dotted boundaries and translucent inner bands.
Airspace interiors remain transparent.
The catalog generation changes the resource URL after an import or removal.

A normal map tap can select rendered airspace features. The nearby page refreshes
its selected features when the catalog or map source changes. `/airspaces/[id]` displays
one feature from the current rendered dataset. Every import or removal invalidates old detail links, including links to
unchanged sources. A direct or stale ID produces an explicit not-found state
when it does not identify a current feature.

## Excluded behavior

The current contract does not include per-file enable/disable controls, an OpenAIP
importer, vertical or class filters, durable IDs, downloads, labels, airspace
warnings, or editing.
