# Local OpenAir Airspace Design

## Context

Updraft must display airspace from one local OpenAir file. The user selects the file. Updraft copies the file into app-owned storage and loads it again at each startup.

This feature is for map display only. It does not provide airspace warnings. It does not download or update airspace data.

The [`openair`](https://crates.io/crates/openair) crate already parses the OpenAir format. Updraft will use that crate directly. Updraft will not implement another OpenAir parser.

The [SeeYou OpenAir 2.1 specification](https://github.com/naviter/seeyou_file_formats/blob/main/OpenAir_File_Format_Support.md) separates airspace class from airspace type. The `AC` record contains the class. The optional `AY` record contains the type. Updraft must retain both values.

The parsed `openair::Airspace` already contains `class` and optional `type_` fields. Updraft does not need a parser extension for this distinction.

## Scope

This design includes these functions:

- Select one local OpenAir file on desktop or Android.
- Parse all airspaces in the selected file.
- Convert every supported boundary to a polygon.
- Store the original selected file in app-owned storage.
- Restore the selected file at startup.
- Show the active file name and airspace count in Settings.
- Replace or remove the active file.
- Serve the active dataset as GeoJSON through `updraft://localhost/airspace.geojson`.
- Render every imported airspace on the map.
- Apply a semantic style group from the OpenAir class and type.

This design does not include these functions:

- Airspace warnings or route intersection checks.
- Altitude, class, activity, or viewport filters.
- Airspace labels, selection, or detail panels.
- Downloads, catalogues, or automatic updates.
- Multiple active files.
- Editing or exporting OpenAir data.
- A general local-resource framework.

## Ownership

The Tauri shell owns platform operations. It opens the file picker, reads the selected file, and writes app-owned storage. Android reads the content URI through the Tauri file-system integration. A platform path or content URI does not cross the frontend boundary.

The [Tauri dialog plugin](https://v2.tauri.app/plugin/dialog/) provides frontend and Rust APIs. Updraft uses the Rust API from the import command. The command uses the nonblocking picker API. Desktop systems return a file-system path. Android returns a content URI. The Tauri file-system plugin reads both forms.

This choice keeps the complete import in one backend operation. The webview does not receive the selected locator or file bytes. It also does not need direct dialog or file-system permissions.

The core owns the canonical in-memory dataset. It owns normalized geometry, classes, types, altitude limits, names, stable dataset-local identifiers, and the active status. The core does not access the file system. It does not depend on Tauri or GeoJSON.

The frontend owns user interaction, localized messages, and MapLibre layers. It does not receive the complete geometry through Tauri IPC.

The custom URI boundary projects a core snapshot to GeoJSON. The projection is a pure helper in the Tauri resource module. The URI handler calls that helper. Conversion logic does not live inside the handler or the core.

This separation keeps `updraft_core` independent from MapLibre and `serde_json`. It also prevents GeoJSON serialization from blocking the core driver task.

## Parser adapter

The parser adapter depends directly on the `openair` crate. It collects the complete parser iterator before it returns a dataset. A parser error rejects the complete import.

The adapter maps the parser types into Updraft types. It does not copy fields that this feature does not use. The canonical airspace contains:

- A dataset-local identifier.
- The name, when present.
- The OpenAir class, when the source provides a class.
- The OpenAir type, when the source provides a type.
- The lower altitude limit.
- The upper altitude limit.
- One normalized polygon.

The identifier is a sequence number in parser order. A new parse of the same source produces the same identifiers.

The canonical class type follows the modern `AC` values:

- `A` through `G`.
- `Unclassified`.

The canonical type is optional. It stores a nonempty OpenAir type code. The adapter converts ASCII letters to uppercase. `AY NONE` produces no canonical type. The adapter retains other unknown type codes instead of rejecting the import. This behavior supports `CUSTOM` and future type values.

The `openair` crate also supports legacy files that put a type in the `AC` record. The adapter maps these parser class variants to canonical types:

- `Ctr` becomes `CTR`.
- `Restricted` becomes `R`.
- `Danger` becomes `Q`.
- `Prohibited` becomes `P`.
- `GliderProhibited` becomes `GP`.
- `WaveWindow` becomes `GSEC`.
- `RadioMandatoryZone` becomes `RMZ`.
- `TransponderMandatoryZone` becomes `TMZ`.

A legacy type value does not produce a canonical class. An explicit `AY` type takes precedence over a legacy type in `AC`. A legacy type with `AY NONE` is a conversion error. Each canonical airspace must have a class, a type, or both.

The altitude type supports ground, feet above mean sea level, feet above ground level, flight level, and unlimited. It stores physical lengths with the existing core units. A parser altitude that only has an unstructured `Other` value is a conversion error. This error rejects the complete import.

The core does not retain the parser model. The parser dependency stays behind the adapter boundary.

## Polygon normalization

The adapter converts polygons, circles, and arc segments to polygon-only geometry immediately after parsing. The core does not store circles or arcs.

Each canonical polygon contains one exterior ring. It has no repeated closing vertex. It must contain at least three distinct vertices. The GeoJSON projection adds the closing vertex.

For a circle or arc, the adapter adds vertices at equal angular intervals. The interval must keep the maximum distance between the source curve and each chord at or below 1 metre. It must preserve the source endpoints and direction. A complete circle must contain at least three distinct vertices.

The one metre limit is a geometric import requirement. It is not a warning-safety guarantee. A future warning design must decide whether it needs a conservative boundary.

The adapter rejects an airspace when it cannot make a valid polygon. This includes invalid coordinates, invalid radii, insufficient distinct vertices, and an unsupported boundary. One rejected airspace rejects the complete import.

The original file remains the durable source. Updraft can change its normalization rules later without a data migration.

## Core model and state

The core stores the active dataset behind an immutable shared reference. A driver query clones only that reference. It does not clone all airspaces or serialize data on the driver task.

The core exposes an airspace status topic with these states:

- `None` means that no file is selected.
- `Active` contains the source name when available, airspace count, and generation.
- `Unavailable` contains the source name when known and a machine-readable load error.

The generation is local to the current process. It changes when the active dataset changes. The frontend uses it only to make MapLibre request a new resource URL.

A new topic subscriber receives the current status immediately. The topic never contains airspace geometry.

The core supports these state transitions:

- Replace the active dataset after durable storage succeeds.
- Clear the active dataset after durable removal succeeds.
- Set an unavailable startup state when the stored source cannot load.
- Return an immutable snapshot for the URI resource.

## App-owned storage

Airspace data uses the Tauri app data directory. It does not use the settings file or the user-selected source path.

The app data directory contains:

- `airspace.txt`, which contains the exact selected bytes.
- `airspace.json`, which contains the selected display name.

The metadata file never stores the original platform path or content URI. It stores the selected display name when the platform provides one. The source file is authoritative. Missing or invalid metadata does not make valid airspace unavailable.

An import uses this sequence:

1. The shell reads the selected bytes.
2. The parser adapter parses and normalizes all airspaces.
3. The shell writes and flushes a temporary source file.
4. The shell writes and flushes a temporary metadata file.
5. The shell atomically replaces `airspace.json`.
6. The shell atomically replaces `airspace.txt`.
7. The shell replaces the core dataset.

The `airspace.txt` replacement is the durable commit point. A failure before that point keeps the previous source and core dataset active. The shell removes temporary files when it can.

Two fixed files cannot form one atomic file-system transaction. A crash between the two replacements can associate the previous source with the new display name. This mismatch affects only Settings. It cannot combine or partially replace airspace geometry. This narrow tradeoff is acceptable for display-only metadata.

If source replacement fails after metadata replacement, the shell tries to restore the previous metadata. A metadata restore failure does not change the active dataset.

If the source commit succeeds but the core driver stops before activation, the import is durable. Startup activates the new file. The command reports `driverStopped` because the current process could not update its map.

Removal deletes `airspace.txt` before it clears the core dataset. A failure to delete the source keeps the current dataset active. The shell deletes `airspace.json` after the source as best-effort cleanup. Leftover metadata does not make a dataset active.

The shell never modifies or removes the user's original file.

## Startup

At startup, the shell reads `airspace.txt` and `airspace.json` before it starts normal interaction.

A missing source produces the `None` state. A valid source produces the `Active` state and generation zero. Missing or invalid metadata omits the source display name. Settings then uses a localized fallback label.

A source read failure, parser failure, or geometry conversion failure produces the `Unavailable` state. The shell preserves both files. The map does not display partial or stale airspace. Settings shows a localized error and lets the user replace or remove the source.

Startup logs the technical cause. It does not put raw dependency errors or full platform paths in frontend state.

## Commands and errors

The frontend invokes one import command. The command opens the native picker and completes the import. The frontend does not pass a path or URI to this command. A picker cancellation is a normal result and does not change state.

The frontend also invokes a remove command. Only one airspace mutation can run at a time. Controls stay disabled while a command is pending.

Commands return machine-readable error kinds with safe context fields. The frontend converts these values into localized Paraglide messages. Backend code does not return complete user-facing sentences.

The error kinds include:

- `pickerFailed`.
- `readFailed`.
- `parseFailed`.
- `geometryFailed`.
- `storageFailed`.
- `driverStopped`.
- `busy`.

An error can include the selected display name. It must not include the full path, content URI, or raw dependency error. Backend logs contain the detailed technical error.

The parser adapter does not inspect dependency error text to extract a line number. A future parser API can add a structured line number without changing the localization boundary.

## GeoJSON resource

The frontend uses this production URL:

```text
updraft://localhost/airspace.geojson?v={generation}
```

The custom URI handler obtains an immutable core snapshot. It then calls `airspace_geojson()` outside the driver task. The helper returns one GeoJSON feature for each canonical polygon.

Each feature contains only these public properties:

- `id`, which is the dataset-local identifier.
- `class`, which is the OpenAir class code or `null`.
- `type`, which is the normalized OpenAir type code or `null`.

The core still retains the name and altitude limits. The resource does not expose them until a user-facing function needs them.

The resource uses `application/geo+json`. It uses `Cache-Control: no-store`. The `v` query parameter causes MapLibre to request the resource again after a dataset change.

If no active dataset exists, the resource returns an empty GeoJSON feature collection. This behavior also handles a request that races with removal. The frontend normally removes the source and layers when the status is not `Active`.

## Settings user interface

Settings contains an Airspace section.

The `None` state shows that no file is selected and provides an Import action.

The `Active` state shows the source display name and airspace count. It uses a localized fallback when the source name is not available. It provides Replace and Remove actions.

The `Unavailable` state shows the source display name when known and a localized error. It provides Replace and Remove actions.

A successful import or removal updates Settings and the map immediately. A validation or storage failure keeps the current state. Removal does not need confirmation because it deletes only the app-owned copy.

## Map rendering

The map adds one GeoJSON source for the active dataset. It adds a fill layer and an outline layer. Both layers use data-driven expressions on the `class` and `type` properties.

A matching type takes precedence over the class. This rule lets `AC E` with `AY RMZ` use the mandatory-zone style instead of the general Class E style.

The first implementation starts with these semantic style groups:

- Controlled: classes `A` through `E`, and types such as `CTR`, `CTA`, `TMA`, `ATZ`, and `AWY`.
- Prohibited, restricted, and danger: types `P`, `R`, `Q`, `GP`, `OFR`, and `TFR`.
- Mandatory zones: types `TMZ` and `RMZ`.
- Gliding and wave: types `GSEC` and `ASRA`.
- Other: classes `F`, `G`, and `UNC`, and unmatched or missing values.

The implementation must consider both class and type. It must give a matching type precedence. Exact group membership, colors, opacity, widths, and dash patterns will be tuned with real map examples.

The airspace fill is above the base map. The outline is above the fill. Traffic and ownship symbols stay above all airspace layers.

The map displays every imported feature. It does not filter by current altitude, class, activation time, or viewport. It does not add labels or pointer interaction.

When the status is `None` or `Unavailable`, the map does not add the airspace source or layers.

## Browser and test seams

The production map container creates the `updraft://` resource URL from the active generation. The leaf airspace map component also accepts inline GeoJSON for Storybook and browser tests. This seam does not become a general data-source abstraction.

Browser-only development does not open a native file picker. It uses no airspace by default or a fixed test fixture. Import acceptance tests run in Tauri environments.

## Tests

The parser adapter has focused fixtures for a polygon, a circle, clockwise and counter-clockwise arcs, and a parser failure. It also has fixtures for modern class and type values, `AY NONE`, legacy type values in `AC`, and an unknown `AY` value. These tests verify the adapter. They do not repeat the dependency's OpenAir grammar tests.

Geometry tests verify these requirements:

- Every canonical shape is a valid polygon with at least three distinct vertices.
- Circle and arc chords have at most 1 metre source-curve error.
- Arc endpoints and direction are preserved.
- One invalid airspace rejects the complete import.

Core tests verify initial state, replacement, removal, generation changes, immediate topic snapshots, and immutable snapshot queries.

Storage tests verify first import, atomic source replacement, failures before the source commit, metadata restore behavior, and removal failure. They also verify startup with an invalid stored source or metadata file.

URI tests verify GeoJSON structure, class and type properties, closed rings, content type, cache headers, and the empty collection response.

Frontend tests verify source URLs, generation changes, class and type precedence, semantic style groups, layer order, and the absence of layers outside the `Active` state.

A MapLibre end-to-end test uses a fixed fixture and verifies that the fill and outline layers load.

Desktop Tauri acceptance verifies native file selection and replacement. Physical Android acceptance verifies selection through a content URI, persistence, restart, replacement, and removal.

## Acceptance criteria

This feature is complete when all of these statements are true:

- A desktop or Android user can select one local OpenAir file.
- A valid file becomes visible on the map and remains active after restart.
- Polygons, circles, and both arc directions render as polygons.
- Every approximated chord has at most 1 metre error from its source curve.
- One parser or geometry error rejects the complete import.
- An import or removal storage failure keeps the previous dataset active.
- Modern files retain both `AC` class and `AY` type values.
- Legacy type values in `AC` remain available as canonical types.
- Settings shows the active source name or a localized fallback, and the airspace count.
- Settings can replace or remove the active source.
- User-facing errors use frontend translations and do not expose raw backend errors.
- GeoJSON conversion occurs outside the core and outside the driver task.
- The map applies semantic class and type groups and keeps traffic and ownship above airspace.
- The implementation does not add downloads, warnings, filters, multiple files, or a general resource framework.
