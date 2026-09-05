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
levels. Non-landable symbols appear from zoom 6 and grow smoothly between zoom
6 and 8. Landable symbols grow smoothly between zoom 6 and 8. Icon halos scale
with the symbols. Labels appear from zoom 8, reserve 8 pixels of collision
padding, and give priority to landables. Arrival labels allow overlap and use a
fixed position below the symbol. Other labels can move above or below the symbol.
A landable with a runway direction shows an oriented runway.

A map tap opens the Nearby page. Waypoint results include symbols within a
24-pixel hit radius. Each row shows elevation, frequency, and notes, in that order, and links to details.
The waypoint kind appears when notes are empty.
Details omit runway length and width when they are absent or zero.
Details use the selected altitude unit for elevation. Runway dimensions remain
in metres. Frequency and notes appear as source text.

Waypoint IDs include the catalog generation. A source change invalidates old
detail links. IDs do not persist across application restarts.

## Arrival margins

Updraft calculates direct-glide arrivals for grass airfields, solid airfields,
gliding airfields, and outlanding sites from every active waypoint source.
The second label line shows the signed arrival margin after subtracting field
elevation and the configured arrival reserve. For example, an arrival 450 m
above the field with a 200 m reserve displays `+250m`. Labels use the selected
altitude unit and round only to whole units.

Symbol and runway-outline colours use the unrounded result:

- Green means arrival at or above the reserve.
- Amber means arrival above field elevation but below the reserve.
- Red means arrival below the reserve and at or below field elevation.

The calculation uses fused MSL altitude, the selected polar with current bugs
and ballast, and the selected MacCready value. GNSS altitude alone can supply
the fused altitude. The last known wind remains usable after it becomes stale
or its estimator resets. When no wind estimate exists, the calculation assumes
zero wind.

The solver accounts for headwind, tailwind, and crosswind. It selects a true
airspeed between the density-corrected minimum-sink speed and 400 km/h.
It minimises `(sink + MC) / ground speed` along the track to the waypoint.
Height loss uses physical polar sink only. Air density remains constant for
each glide, using the ISA density at the current fused altitude. There is no
separate MC-0 or safety-MC result.

Stale position or altitude retains the last input and encloses the margin in
parentheses, such as `(+250m)`. Symbol colours remain active. Stale wind alone
does not add parentheses. Missing position or altitude, or wind that prevents
forward progress at any allowed speed, gives a violet symbol and a name-only
label. Negative margins remain valid results.

Arrival labels allow overlap and use a fixed anchor so changing values do not
restart label fading or wait for collision placement. Their offset includes
the spacing that variable-anchor placement adds from collision padding.
Nearby arrival labels can overlap. Basemap label fading remains enabled.

The map requests all landables in the viewport plus a 10% buffer on each side.
Viewport changes request an immediate calculation, limited to one start per
100 ms during movement. Sensor and glide-setting changes request a calculation
once one second has passed since the last start. Pending changes use the latest
inputs. Only one calculation runs at a time. Previous results remain visible
until a replacement arrives. Landables outside the previous buffer wait for
the next batch. Catalog changes discard results from the previous catalog.

These values describe a straight glide through uniform wind and still vertical
air. They do not check terrain, obstacles, airspace, or landing-site suitability.
Native desktop delivery has been confirmed. Final layout acceptance, Android
validation, and performance measurements remain open.

## Known limitation

`seeyou-cup` 0.3.1 can panic when a longitude degree field overflows its internal
integer. For example, `99900.000E` triggers this panic. An expected-panic test
records this dependency limitation. Updraft does not repair or suppress it.
The import worker reports a failed operation. The same panic during startup
can prevent the application from loading. Do not use affected CUP files.
