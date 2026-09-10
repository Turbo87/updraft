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

The core resolves FLARM and ICAO addresses through the United FlarmNet database.
Lookup uses the six-digit hexadecimal value and ignores letter case. Random and
unknown ID types are excluded. United FlarmNet shares one address namespace, so
it cannot distinguish a FLARM address from an ICAO address with the same value.

Published targets include the matching database record, when available. Database
replacement refreshes current targets without changing their report age or stale
state.

## Freshness

A new report marks the target current. At five seconds without a report, the
core publishes the target once with `stale: true`. At 30 seconds, the core
removes the target.

A fresh report before removal replaces the stale target and clears the stale
state. Tick inputs apply stale and removal transitions.

## Climb estimates

The core publishes a normalized EMA with a 10-second time constant and 20-second
and 30-second altitude-change averages. It ignores the reported FLARM climb rate.
All estimates use target MSL altitude. Ownship altitude must satisfy the existing
three-second freshness rule before it can supply an estimator sample.

The window averages use partial history and interpolate altitude at the window
boundary. The shared core climb module keeps window history separate from the
EMA's previous sample, weighted sum, and weight. The EMA normalizes its accumulated
weight during startup. Both algorithms weight intervals by their duration.

Missing usable altitude makes the published estimates unavailable without
refreshing averaging history. Reception gaps through 60 seconds use the altitude
change across the gap. Longer gaps reset the estimates. A change of reporting
device or ownship altitude source also resets them. Duplicate or older samples
do not change averaging state.

Averaging history survives target removal at 30 seconds and expires after more
than 60 seconds without usable altitude. The next sample after a reset establishes
a baseline. A subsequent sample produces the first estimates.

The map shows the selected climb estimate and defaults to normalized EMA.
The Vario settings page selects the shared climb averaging method.

## Topic updates

A new subscriber receives one complete traffic snapshot. Later reports use
deltas with ordered upserts and removed IDs. Database replacement publishes a new
snapshot when the information attached to existing targets changes. A repeated
report that does not change the published target produces no delta, but it still
refreshes the target age.

The frontend store replaces all entries for a snapshot. For a delta, it applies
upserts and then removals. Store subscribers receive the update after the store
is current.

If an incremental MapLibre source update fails, the frontend logs a warning and
rebuilds the source from the complete traffic store. It does not report success
while leaving the map source partially updated.

## Map presentation

MapLibre uses one GeoJSON point per target. The feature contains the typed ID,
aircraft type, FLARM alarm level, stale state, optional track, and formatted MSL
altitude, with callsign or registration above it.

Symbols use aircraft-type icons. Directional targets rotate with the map track.
Balloons and targets without track use fixed symbols. Icon size changes with map
zoom.

FLARM alarm level controls symbol color. Stale targets use reduced opacity. The
label appears from zoom level 7 and uses the configured altitude unit. The first
line shows callsign, with registration as the fallback. The second line shows
MSL altitude. A third line shows the positive selected climb estimate in the selected
vertical-speed unit. It uses one decimal place for m/s and knots, and whole ft/min.
Stale targets, missing estimates, and estimates that round to zero or below have
no climb line. Missing name or altitude removes that line. A target without any
label values has no label.

One invisible 24-pixel-radius hit layer supports map inspection. The debug
overlay can make this hit area visible.

## Inspection and details

A nearby page captures the target IDs that match the selected map point. That
membership stays fixed while the route remains mounted. Later topic updates
refresh retained targets with the same ID.

A removed target remains in that mounted result as unavailable. A later update
for the same ID restores it. `/traffic/[id]` supports direct visits and the same
live or unavailable behavior.

Traffic details also show callsign, registration, aircraft model, pilot, airfield,
frequency, and FLARM ID from the matching database record. Empty fields are hidden.
The database aircraft model is separate from the reported aircraft category.

Traffic details show all three climb estimates in the selected vertical-speed
unit, independent of the map method setting. Details include positive, zero,
and negative values. Missing estimates show a dash. Retained values use the
existing stale presentation when the target is stale or unavailable.

## Database refresh

The shell downloads [United FlarmNet](https://turbo87.github.io/united-flarmnet/united.json)
at startup. It loads the saved database before the first download completes.
A successful download schedules the next refresh three hours later. Failed attempts
retry after 1, 5, 15, 30, and then 60 minutes. Further failures retry hourly.
Success resets the retry sequence.

Refresh runs while the application process is alive, including during Android
background operation. Android resume events check for an overdue refresh.
One worker serializes downloads. Closing the process stops refresh.

Requests have a 30-second timeout and an 8 MiB response limit. The shell validates
the JSON before atomic file replacement and core publication. It rejects empty
files, invalid IDs, and duplicate IDs. HTTP, parsing, and storage failures retain
the previous database and are logged without a user notification. A missing or
unreadable cache leaves traffic usable without database information until refresh
succeeds.

## Excluded behavior

The current contract does not include OGN or ADS-B input, cross-network
deduplication, trails, radar view, navigation toward traffic,
warning presentation, acknowledgement, or Updraft-calculated collision risk.
