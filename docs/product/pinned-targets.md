# Pinned targets

Status: Current behavior

Pilots can pin waypoints, map positions, traffic, and the saved task independently of the primary
navigation target. Target details provide pin and unpin actions. Pinning does
not start navigation. Unpinning does not stop navigation.

## Display and selection

The Flight View shows compact pinned-target rows below the primary target.
Rows stay in pinning order. The panel scrolls within a limited height to retain
map space. A pin that matches the primary target is hidden. It returns to its
original position when navigation changes or stops.

Each row shows the target name and type, bearing, distance, and arrival margin
for a fixed target or relative altitude for traffic. Calculations, units, missing
inputs, and stale indications follow [goto navigation](navigation.md). Arrival
margins do not establish terrain clearance or landing suitability.

A task pin follows the current task point. Unpinning the task does not stop it
or add it to recents. Unpinning other targets moves them to recent goto history.

Selecting a row opens its details. **Navigate to target** makes it primary and
keeps it pinned. **Unpin target** removes only the pin.

## Identity and lifetime

Waypoint matching uses the exact name and approximate coordinates. Each coordinate
has a tolerance of 0.0001 degrees, with longitude wrapping at the antimeridian.
This is a coordinate comparison, not a distance threshold. Map positions match
by coordinates. Target types remain distinct. Traffic matches by its full ID,
including the ID type.

Pinning a matching target keeps the original snapshot and position in the list.
Waypoint pins retain their name, coordinates, and elevation independently of
source-file changes. Unpin and pin again to use updated source data. Map pins
retain coordinates and use the available offline terrain for arrival margins.

Traffic pins follow live reports and retain the latest report in memory after
traffic-list expiry. The row shows report age and stale values. Selecting a
retained traffic pin for navigation keeps that report available.

## Persistence and failures

The core owns the ordered pin list and guidance. The shell saves pin identities
and waypoint snapshots separately from the primary target. Internal pin IDs
remain stable across saved-list restoration. Traffic storage contains only its
ID, not its report, label, position, altitude, or observation time. After restart,
traffic pins wait for a new report.

A storage failure leaves the live pin change in place and shows an error. Retry
repeats the failed operation, including an unpin whose row has already gone.
Invalid saved lists are not restored, and the shell logs the failure.

Manual reordering and additional map decorations for pins are outside this slice.
Physical Android lifecycle and in-flight readability checks remain open.
