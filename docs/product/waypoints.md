# Waypoints

Status: Current behavior

Updraft imports local SeeYou CUP waypoint files through Settings. Each file
remains an independent source. The core owns immutable parsed datasets. The
Tauri shell owns the file picker, persistent storage, and GeoJSON delivery.

## Import and storage

An import replaces the source with the exact same display filename. Other
sources remain unchanged. Identical waypoints in different files remain
separate. Restarting the application reloads the stored source files.

The importer uses `seeyou-cup` to parse the full file, including its task
section. Updraft retains only the waypoint data and shows the parser warnings
in Settings. A parser error or a file with no valid waypoints rejects the
import. A failed replacement preserves the previous source. Removing a file requires
confirmation and removes only that source.

The importer retains the name, coordinates, CUP type, MSL elevation, runway
direction and dimensions, frequency text, and description. It does not import
tasks, embedded images, or navigation targets.

A stored source that cannot be read or parsed appears as unavailable. Other
sources remain usable.

## Map and details

Grass, solid, and gliding airfields share a plain circle symbol. Other CUP
types have distinct symbols. Landable symbols remain visible at wide zoom
levels. Other symbols appear from zoom 6. Symbols grow smoothly between zoom
6 and 8. Labels appear from zoom 8, avoid overlap, and give priority to
landables. A landable with a runway direction shows an oriented runway.

A map tap opens the Nearby page. Waypoint results include symbols within a
12-pixel hit radius. Each row links to details and identifies its source.
Details use the selected altitude unit for elevation. Runway dimensions remain
in metres. Frequency and notes appear as source text.

Waypoint IDs include the catalog generation. A source change invalidates old
detail links. IDs do not persist across application restarts.

## Known limitation

`seeyou-cup` 0.3.1 can panic when a longitude degree field overflows its internal
integer. For example, `99900.000E` triggers this panic. An expected-panic test
records this dependency limitation. Updraft does not repair or suppress it.
The import worker reports a failed operation. The same panic during startup
can prevent the application from loading. Do not use affected CUP files.
