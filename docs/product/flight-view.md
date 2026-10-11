# Flight View

Status: Current behavior

The flight view keeps one MapLibre map mounted for the application session. It
shows the ownship position, imported airspace, and observed traffic. Other
application routes cover the map instead of replacing it.

## Map session

One shared map state stores the MapLibre instance, center, zoom, bearing, pitch,
and follow mode. The initial camera uses latitude `50.823`, longitude `6.186`,
zoom `11`, bearing `0`, and pitch `0`.

The current base map uses the OpenFreeMap Positron style. Updraft adds local
airspace data and live ownship and traffic layers after the MapLibre style and
Updraft sprite set are ready.

Opening the Main Menu, settings, nearby results, or detail routes keeps the map
and camera state alive. A new application session creates a new map state.

## Ownship dead reckoning

The ownship symbol moves between received GPS positions. On each animation
frame, the frontend advances the symbol from the latest received position along
a circular arc. The arc uses the ground speed, track, and turn rate of that
position. It limits the turn rate to 30 °/s.

Dead reckoning starts when the frontend receives the position, because the
frontend clock and the GPS clock are not synchronized. It stops 1.5 seconds
later. The symbol then stays at its last position until the next position
arrives. A new position replaces the symbol position immediately.

The symbol does not move without a ground speed or track, below 2 m/s, or while
the GPS state is stale. Dead reckoning changes only the symbol and the follow
camera. The trail, infoboxes, navigation, and flight recording use the received
positions.

## Position follow mode

The map starts in follow mode. On each animation frame, follow mode centers the
map on the ownship symbol. The camera keeps the current zoom, bearing, and
pitch. While a gesture, its inertia, or a zoom animation moves the map, the
camera does not change. It also does not change while a pointer is down on the
map, because a camera change resets a gesture that has not started yet. The
camera centers the map again when the movement ends. Scroll zoom and touch zoom
keep the map center while follow mode is on.

Dragging the map stops follow mode. The two-finger pan of a pinch zoom does not
stop it. The flight view then shows the Return to position control. This
control stops the active camera movement and enables follow mode. The camera
then moves from its current center to the moving ownship symbol in 300
milliseconds. If no position is available, the map waits for the next valid
position.

Zoom, bearing, and pitch changes do not stop follow mode.

## Flight infoboxes

The Flight View reserves space for ten read-only infoboxes. Their order is
Altitude, AGL, Ground speed, TAS, Bank angle, Vario, Netto, Wind speed, Wind
direction, and Average vario. Altitude uses the derived MSL altitude. AGL uses
the published height above terrain. TAS follows the fallback order in
[Flight data](flight-data.md#true-airspeed).

Portrait uses five columns and two rows below the map. Landscape uses two
columns and five rows on the right. Both layouts fill row by row. The map stays
mounted and resizes into the remaining space. Average vario uses the 20-second
value from [Flight data](flight-data.md#average-vario).
Tapping a cell has no action. Field replacement and page switching are not
implemented.

Altitude and speed values use whole numbers in the selected units. Vario and
Netto show one decimal, or whole numbers for ft/min, with a plus sign for
positive values. Wind direction uses whole degrees from 0 through 359. Bank
angle shows the unsigned magnitude with a left or right chevron. An angle that
rounds to zero shows both chevrons. Average vario uses the vario format.
Formatting follows the selected locale.

Stale values retain their units and use the stale-value color. Missing values
show a dash without a unit. Longer values use smaller type to fit the cell.

The dock surface extends to the screen edges. Portrait adds the bottom safe
inset to its height and protects content with the left and right insets.
Landscape adds the right inset to its width and protects content with the top
and bottom insets. The map controls use only the insets for edges that still
touch the screen, so the dock and controls do not apply the same inset twice.

## Map inspection

A normal map click opens
`/nearby/[latitude]/[longitude]`. The route stores each coordinate with six
decimal places. A direct nearby URL does not move the camera.

The nearby page shows the selected coordinate and its distance and true bearing
from the displayed ownship position. Invalid or non-finite route coordinates
show an error.

Airspace and traffic results come from MapLibre hit layers at the selected
coordinate. The query waits for the required style, source, and layer. It does
not move an off-screen position into view.

Airspace results refresh when the catalog or map source changes. A source
error produces an empty result. Catalog changes invalidate old airspace detail
links. The traffic result keeps its initial target membership for the
lifetime of the mounted nearby page. Traffic updates refresh the retained
targets and mark removed targets unavailable.

The nearby page links to airspace and traffic detail routes. Returning to the
map preserves its camera and follow state.

## Goto navigation

The target bar reserves space above the map when a primary target is active.
Waypoint details, map inspection, and traffic details can select a target.
See [Goto navigation](navigation.md) for target lifetime and guidance rules.
[Pinned targets](pinned-targets.md) appear below the primary target in a
scrollable panel. They remain independent of primary navigation.

## Excluded behavior

The current flight view does not include map orientation policy, automatic
zoom, smart ownship offsets, flight-mode behavior, task navigation, terrain,
weather, or configurable map layers.
