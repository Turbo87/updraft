# Traffic

How FLARM and OGN traffic data are handled, from source to screen.

## Sources

- **FLARM** devices deliver nearby targets as `PFLAA` sentences and their own collision-alarm/status assessment as `PFLAU`, both at ~1 Hz, over whatever transport the device is connected through (see [devices.md](devices.md)).
- **OGN** live positions are pulled over the network by an async adapter task (see [runtime.md](runtime.md#effects) and _WeGlide Live_ below). This is optional. The system stays fully functional without connectivity, and OGN data arrives several seconds late.

## The Traffic Table

The core merges all traffic observations into one table keyed by target ID. This is a merged data category, not a selected signal (see [devices.md](devices.md#source-selection-and-merging)). A glider visible through FLARM and OGN at the same time is one target. Direct FLARM data wins because it is faster and does not use the network. Old targets remain visible for a short time with a clear stale marker.

The table is published as changes keyed by target ID (see [core.md](core.md)). Each target is a kinematic state vector (see [core.md](core.md#outputs)), not only a coordinate.

A FLARMNet/OGN device database for resolving IDs to registrations/callsigns is used to enhance the table with additional information.

## Warnings

Traffic warnings come exclusively from relayed `PFLAU` alarms, nothing self-computed. FLARM already runs the collision-risk assessment on the device, so Updraft relays its alarm levels to the warning output.

## WeGlide Live

OGN data is pulled via the **WeGlide Live API** rather than the raw OGN/APRS stream: polled every ~3 seconds, scoped to a bounding box around the current map view. The API serves the latest known position per aircraft, so a connectivity gap heals on the next poll, whereas the raw stream only delivers updates and every update missed during an outage is lost.

The endpoint is `GET https://live.weglide.org/api/locations?format=json`, optionally filtered with a `bbox=west|south|east|north` query parameter. It returns a JSON array of aircraft records: FLARM ID, display name, unix timestamp, longitude/latitude, altitude, bearing, and vario.

## Display Extrapolation

FLARM updates about once per second, and OGN is slower. The frontend estimates each target's current render position between updates (see [frontend.md](frontend.md)). It uses the kinematic state vector from the core. Every client starts from the same values, and the transport sends one message per target update instead of one per frame. The timestamp tells the frontend when data is old. This smoothing is only visual because traffic warnings come directly from FLARM alarms.
