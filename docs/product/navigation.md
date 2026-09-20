# Goto navigation

Status: Current behavior

Goto has one primary target. A pilot can select a waypoint, a map position,
or a traffic aircraft. A new selection replaces the current target.
Navigation continues until the pilot selects another target or uses **Stop
navigation**. Passing a target does not stop navigation.

## Selection and display

Waypoint details, map inspection, and traffic details provide navigation
actions. Selection returns to the Flight View and preserves the map camera
and follow setting. The target bar reserves space above the map. The ten
instrument boxes remain unchanged. Selecting the bar opens target details
and the stop action.

The bar shows direct distance and bearing relative to ownship ground track.
If ground track is unavailable, it shows true bearing with a `T` suffix.
Missing ownship position makes bearing and distance unavailable. Stale
ownship position makes guidance stale. The map shows a target marker and a
course line when the required positions are available.

Waypoint and map targets show an arrival margin when the calculation inputs
are available. The calculation uses fused MSL altitude, the current glide
polar, MacCready setting, available wind, target elevation, and arrival
reserve. It uses the same calculation as waypoint arrival labels. Missing
wind uses calm air. The margin does not establish terrain clearance along
the route or landing suitability.

## Target lifetime

A waypoint target retains its selected name, coordinates, and elevation.
Source replacement, removal, or deactivation does not change this snapshot.
Selecting the waypoint again uses its current data.

A map target retains coordinates. The shell samples offline terrain for its
MSL elevation. Missing terrain leaves the arrival margin unavailable.
Bearing and distance remain available. Terrain inventory changes trigger a
new sample. A result for different target coordinates cannot change the
current arrival margin.

A traffic target follows its ID, which includes the ID type. The bar shows
relative altitude instead of arrival margin. Missing target or ownship
altitude makes this value unavailable. A retained report becomes stale
after five seconds. The bar shows the age of its position report. The
selected report remains available during the session after the ordinary
traffic list removes it. Fresh reports resume live guidance.

## Persistence and ownership

The core owns the selected target and calculates guidance. The frontend
sends select and stop commands and displays the navigation topic. The shell
handles storage and terrain access.

The shell atomically replaces the saved target after selection, replacement,
or stopping. It restores waypoint snapshots and map coordinates on startup.
For traffic, it saves only the ID. It does not save a traffic position,
altitude, identity label, or observation time. After restart, traffic
navigation shows **Waiting for traffic** without guidance or a course line
until a report arrives.

A save failure leaves the active navigation choice usable. The selection or
stop screen reports that the choice was not saved and that a previous target
can return after restart. The pilot can retry the action. Invalid saved data
is not restored, and the shell logs the load failure.

## Later work

Tracked waypoints, competition tasks, record flights, casual flight goals,
and emergency behavior require separate scope and design work. They are
not part of goto navigation.
