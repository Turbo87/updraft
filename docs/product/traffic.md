# Traffic

Status: Current behavior

Updraft accepts FLARM `$PFLAA` reports from enabled external devices. The core
converts relative reports to absolute targets, stores them by identity, and
publishes traffic updates. The frontend renders those targets on the map and in
inspection routes.

Updraft also accepts periodic `$PFLAM,U` identity messages when a device sends
them. It does not enable FLARM Messaging or change device settings. Missing
messages do not produce warnings or diagnostics.

## Observation

A usable report requires a target identity and relative north and east
positions. The core converts the relative position with an ownship position.
It first uses a position from the same external device. It falls back to the
currently displayed GPS position.

The target MSL altitude requires relative vertical distance and an ownship MSL
altitude. The core first uses same-device altitude and then displayed GPS
altitude. Correction can replace these position and altitude references as
specified below.

Traffic is a merged domain. It does not use flight-data source selection. Two
devices that report the same typed identity update the same target.

A target stores its typed identity, absolute position, optional MSL altitude,
aircraft type, optional track, FLARM alarm level, and freshness state. Reports
update this data subject to the horizontal position acceptance rules below.

The core caches registration, pilot name, aircraft type, and callsign from
periodic FLARM Messaging sentences. It attaches cached identity to a matching
target, including a target with `no_track` set. An identity update publishes an
active target again without changing its report age. The cache lasts for the
current application session.

## FLARM correction

The experimental FLARM position correction defaults to enabled. The Traffic
settings page can disable it for devices that use a different reference model.
The setting is stored across restarts. Missing stored values enable it.
Correction applies to FLARM-source reports and older reports without a source
field. Other reported sources use the uncorrected calculation.

With correction enabled, the core identifies one-second traffic cycles from
RMC/GGA timestamps and PFLAU/PGRMZ order. Traffic before the next cycle marker
uses the preceding cycle's RMC fix, even if a newer GPS sentence has arrived.
The core projects that fix two seconds along its reported ground track and
speed before adding the relative traffic position.

Altitude correction uses the cycle's GGA height and its change from the
preceding GGA fix. Both GGA fixes must be fresh and one to three seconds apart
in GPS time. The core projects that height two seconds before adding the
relative traffic height.

If a cycle marker is missing, a repeated target can identify the next cycle.
The target must have reported a position while GPS time matched the active
cycle. Its next position report advances the cycle if GPS time has advanced by
exactly one second. The core switches both position and altitude references.
A first report from another target still uses the preceding cycle. Repeated
reports without GPS progress do not advance the cycle. This inference adds no
delay and does not revise reports accepted before the repeated target arrives.

The decoded target is a prediction for two seconds after the cycle timestamp.
The core uses the target's reported ground velocity and climb rate to move it
back to the latest same-device GPS timestamp. A report before the next GPS fix
moves back two seconds. A report after that fix moves back one second.
Horizontal correction requires target speed and track. Zero target speed needs
no track. Vertical correction requires the target climb rate.

Each device retains the latest projected position, raw and projected GPS
altitudes, and active-cycle references. New GPS fixes leave the active references
unchanged until the traffic cycle advances. Correction requires an exact cycle
match and a reference received less than three seconds ago.

Horizontal and altitude correction are independent. Missing altitude history or
climb rate uses the uncorrected altitude calculation. Missing horizontal
references or motion fields use the hold and fallback rules below. Disabling
correction permits the uncorrected calculation for both components.

Device runtime resets, connection changes, backward GPS time changes, and stale
input gaps clear the reference history. Midnight retains continuity. A setting
change affects subsequent reports and resets climb histories. The correction
does not shift ownship, pressure altitude, or reception time.

This model comes from one recording. It remains experimental until other
devices have been checked. Its predicted positions do not establish exact
sentence transmission times.

## Position acceptance

A target's position and altitude change only when a new traffic-position report
arrives. GPS fixes update the reference for later reports. They do not move
stored targets or add climb samples.

With FLARM position correction enabled and available, the core accepts the first
horizontal position for each target GPS timestamp immediately. Later reports for
the same timestamp retain that position and track. They still update report age,
alarm level, and other target data. This prevents position revisions at one time
from appearing as flight movement. Some discarded revisions can be more accurate.

The displayed track follows the bearing between accepted positions at advancing
GPS timestamps. Steps shorter than 5 meters and report gaps of 5 seconds or more
use the reported track instead. The first accepted report also uses reported
track. Clock rewinds, source changes, connection resets, and changes to the
correction setting start a new position sequence. Expired targets lose their
position history.

This policy adds no buffering and no movement between reports. It does not change
reported velocity, altitude correction, or energy compensation.

If a corrected target temporarily loses its ownship reference or required motion
fields, the core holds its horizontal position and track. Reports still update
freshness, alarms, and other target data. The hold ends when correction recovers
or the accepted position reaches two seconds of age. Reports without a usable
reference do not extend this limit. After the limit, reports use fallback
coordinates. Recovery can produce a catch-up step after the held interval.

Targets without a prior corrected position use fallback coordinates immediately.
Disabling correction or changing the source also permits immediate updates.
The hold does not cross connection or position-history resets.

A future 30-second traffic trace must append only accepted positions at advancing
GPS timestamps. Same-timestamp reports must not append or replace trace points.
Metadata updates must not create trace points. Reception gaps remain gaps in the
observations and must not be filled with synthetic positions.

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
and 30-second climb averages. A fourth estimate smooths height with a 5-second
time constant, then calculates a 20-second window average. Its first sample seeds
the height filter. It keeps separate filtered height history.
The reported FLARM climb rate only aligns target altitude in time. The core
derives climb estimates from the aligned altitude samples.
All estimates use target MSL altitude. Ownship altitude must satisfy the existing
three-second freshness rule before it can supply an estimator sample.

Corrected altitude samples use GPS intervals for the averages and filters.
Reception time still controls history expiry and target freshness. Changing
between GPS and reception intervals resets the estimator. Backward GPS time or
a sample gap longer than 60 seconds also resets it. Repeated sample times do
not add another measurement.

When energy compensation is enabled and FLARM provides ground speed and track,
the core estimates target airspeed
by subtracting current ownship wind from the target ground-velocity vector.
It adds the airspeed energy-height change to the altitude change before averaging.
This estimates total-energy climb without a target polar or sink-rate correction.
Both velocity endpoints use the same wind estimate to avoid energy offsets from
wind updates. Reported velocity uses its report timestamp.

When either reported velocity field is missing, the core derives both components
from consecutive absolute target positions. Ownship motion is included in that
reconstruction. Both position references must be fresh and use the same source.
The reception interval must be positive and at most five seconds.

Derived velocity represents the midpoint of its position interval. The core
interpolates altitude to the velocity timestamps. It delays the compensated result
by half the latest derivation interval, at most 2.5 seconds. This midpoint estimate
is approximate during turns. Missing derivation inputs use raw climb and flush any
pending altitude change once. Reported velocity is preferred when both fields return.

Compensation requires current wind, horizontal separation at most 10 km, and
absolute vertical separation at most 1,500 m. Missing velocity or geometry, or
stale wind, uses raw altitude change. Compensation and raw climb share continuous
averaging history. Recovery establishes a new velocity baseline before compensation
resumes. Switching modes does not add or remove an energy-height offset.

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

The map shows the selected climb estimate and defaults to the smoothed 20-second average.
The Vario settings page selects the shared climb averaging method.
Map climb labels appear only for unknown traffic, gliders, hang gliders, and
paragliders. Other traffic types retain their name and altitude labels.

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
altitude, with callsign or registration above it. Broadcast identity has
priority. United FlarmNet supplies missing values.

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

Traffic details show callsign, registration, aircraft model, and pilot from
broadcast identity when available. United FlarmNet supplies missing values and
adds airfield, frequency, and FLARM ID. Empty fields are hidden. The identity
aircraft model is separate from the reported aircraft category.

Traffic details show all four climb estimates in the selected vertical-speed
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
