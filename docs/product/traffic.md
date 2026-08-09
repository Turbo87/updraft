# Traffic

Status: Current behavior

Updraft accepts FLARM `$PFLAA` reports from enabled external devices. The core
converts relative reports to absolute targets, stores them by identity, and
publishes traffic updates. The frontend renders those targets on the map and in
inspection routes.

## Observation

A usable report requires a target identity and relative north and east
positions. The core converts the relative position with an ownship position.
It first uses a position from the same external device. It falls back to the
currently displayed GPS position.

The target MSL altitude is available only when the report contains relative
vertical distance and an ownship MSL altitude is available. The core first uses
same-device altitude and then the displayed GPS altitude.

Each accepted report replaces the complete stored target with the same
identity. A target stores:

- typed FLARM identity
- absolute position
- optional MSL altitude
- aircraft type
- optional track
- FLARM alarm level
- freshness state

Traffic is a merged domain. It does not use flight-data source selection. Two
devices that report the same typed identity update the same target.

## Identity

Internal identity contains the FLARM ID type and 24-bit value. The frontend ID
is a stable string for that typed value, such as `icao:ABC123` or
`flarm:ABC123`.

The current model does not resolve registration, competition ID, pilot name, or
aliases.

## Freshness

A new report marks the target current. At five seconds without a report, the
core publishes the target once with `stale: true`. At 30 seconds, the core
removes the target.

A fresh report before removal replaces the stale target and clears the stale
state. Tick inputs apply stale and removal transitions.

## Topic updates

A new subscriber receives one complete traffic snapshot. Later changes use
deltas with ordered upserts and removed IDs. A repeated report that does not
change the published target produces no delta, but it still refreshes the
target age.

The frontend store replaces all entries for a snapshot. For a delta, it applies
upserts and then removals. Store subscribers receive the update after the store
is current.

If an incremental MapLibre source update fails, the frontend logs a warning and
rebuilds the source from the complete traffic store. It does not report success
while leaving the map source partially updated.

## Map presentation

MapLibre uses one GeoJSON point per target. The feature contains the typed ID,
aircraft type, FLARM alarm level, stale state, optional track, and formatted MSL
altitude.

Symbols use aircraft-type icons. Directional targets rotate with the map track.
Balloons and targets without track use fixed symbols. Icon size changes with map
zoom.

FLARM alarm level controls symbol color. Stale targets use reduced opacity. The
altitude label appears from zoom level 7 and uses the configured altitude unit.

One invisible 24-pixel-radius hit layer supports map inspection. The debug
overlay can make this hit area visible.

## Inspection and details

A nearby page captures the target IDs that match the selected map point. That
membership stays fixed while the route remains mounted. Later topic updates
refresh retained targets with the same ID.

A removed target remains in that mounted result as unavailable. A later update
for the same ID restores it. `/traffic/[id]` supports direct visits and the same
live or unavailable behavior.

## Excluded behavior

The current contract does not include OGN or ADS-B input, cross-network
deduplication, traffic lookup, trails, radar view, navigation toward traffic,
warning presentation, acknowledgement, or Updraft-calculated collision risk.
