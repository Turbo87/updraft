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
