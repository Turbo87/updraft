# Traffic

How FLARM and OGN traffic data are handled, from source to screen.

## Sources

- **FLARM** devices deliver nearby targets as `PFLAA` sentences and their own collision-alarm/status assessment as `PFLAU`, both at ~1 Hz, over whatever transport the device is connected through (see [devices.md](devices.md)).
- **OGN** live positions are pulled over the network by an async adapter task (see [core.md](core.md) and _WeGlide Live_ below). This is an optional online enhancement: the system stays fully functional without connectivity, and OGN data arrives with multi-second latency.

## The Traffic Table

The core merges both sources into one table keyed by target ID. A glider visible through FLARM and OGN at the same time is a single target, with the direct FLARM data taking priority (lower latency, no network path). Targets age out visibly after a while. It should be easy to distinguish live FLARM targets, OGN targets, and outdated ones.

The table is published on the `traffic` topic as a keyed collection (see [core.md](core.md)). Each target is a **kinematic state vector** — position, track, ground speed, turn rate, climb rate, and timestamp — not a bare coordinate.

A FLARMNet/OGN device database for resolving IDs to registrations/callsigns is used to enhance the table with additional information.

## Warnings

Traffic warnings come exclusively from relayed `PFLAU` alarms, nothing self-computed. FLARM already runs the collision-risk assessment on the device, so Updraft relays its alarm levels to the warning output.

## WeGlide Live

OGN data is pulled via the **WeGlide Live API** rather than the raw OGN/APRS stream: polled every ~3 seconds, scoped to a bounding box around the current map view. The API serves the latest known position per aircraft, so a connectivity gap heals on the next poll, whereas the raw stream only delivers updates and every update missed during an outage is lost.

The endpoint is `GET https://live.weglide.org/api/locations?format=json`, optionally filtered with a `bbox=west|south|east|north` query parameter. It returns a JSON array of aircraft records: FLARM ID, display name, unix timestamp, longitude/latitude, altitude, bearing, and vario.

## Display Extrapolation

FLARM updates at ~1 Hz and OGN much slower, so displayed targets need extrapolation to stay current. The kinematic state vectors published by the core carry everything needed: the frontend extrapolates each target to render time (see [frontend.md](frontend.md)). Every client extrapolates from identical states, the transport carries one message per target update instead of per frame, and staleness stays explicit — the vector's timestamp drives the aging display.
