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

Opening settings, nearby results, or detail routes keeps the map and camera
state alive. A new application session creates a new map state.

## Position follow mode

The map starts in follow mode. Each valid displayed GPS position starts a
300-millisecond center-only camera transition. The transition keeps the current
zoom, bearing, and pitch.

Dragging the map stops follow mode. The flight view then shows the Return to
position control. This control stops the active camera transition and enables
follow mode. If no position is available, the map waits for the next valid
position.

Zoom, bearing, and pitch changes do not stop follow mode. A later position
update keeps those camera values and changes only the center.

## Flight infoboxes

The Flight View reserves space for ten read-only infoboxes. Their order is
Altitude, AGL, Ground speed, TAS, Bank angle, Vario, Netto, Wind speed, Wind
direction, and Zoom. Altitude uses the derived MSL altitude. AGL uses the
published height above terrain. TAS follows the fallback order in
[Flight data](flight-data.md#true-airspeed).

Portrait uses five columns and two rows below the map. Landscape uses two
columns and five rows on the right. Both layouts fill row by row. The map stays
mounted and resizes into the remaining space. Zoom follows the live map camera.
Tapping a cell has no action. Field replacement and page switching are not
implemented.

Altitude and speed values use whole numbers in the selected units. Vario and
Netto show one decimal, or whole numbers for ft/min, with a plus sign for
positive values. Wind direction uses whole degrees from 0 through 359. Bank
angle shows the unsigned magnitude with a left or right chevron. An angle that
rounds to zero shows both chevrons. Zoom shows two decimals without a unit.
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

## Excluded behavior

The current flight view does not include map orientation policy, automatic
zoom, smart ownship offsets, flight-mode behavior, task navigation, terrain,
weather, or configurable map layers.
