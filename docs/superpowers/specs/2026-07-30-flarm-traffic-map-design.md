# FLARM traffic on the map

## Context

The [application architecture](2026-07-25-app-architecture-design.md) assigns
FLARM traffic to milestone 3. The core already receives NMEA bytes from TCP and
Bluetooth SPP connections. `updraft_nmea` already parses `PFLAA` sentences.
The frontend already renders the ownship with MapLibre and uses the generated
SDF sprite sheet.

This design adds the first complete traffic path. It starts with `PFLAA`
observations and renders identified traffic targets on the map. It also defines
the state and update boundary that a later OGN source can use.

The application architecture says that each topic message contains a complete
snapshot. This design replaces that rule for the traffic topic. A new
subscription receives a complete traffic snapshot. Later traffic messages
contain deltas. This exception avoids repeated serialization of the complete
traffic state when OGN adds more targets.

## Scope

This slice adds:

- Core-owned traffic state.
- `PFLAA` ingestion from every external device.
- Absolute map positions for identified targets.
- Optional absolute MSL altitude.
- A complete traffic snapshot on subscription.
- Batched traffic deltas after subscription.
- Fresh, stale, and removed target states.
- A frontend traffic store.
- Incremental MapLibre GeoJSON updates.
- Traffic icons and absolute-altitude labels on the map.

This slice does not add:

- OGN ingestion.
- Association of observations that have no identity.
- A display for PFLAA observations that have no relative-east value.
- Relative-altitude labels.
- Configurable freshness periods.
- Movement interpolation or prediction.
- New traffic sprites.
- Target details or map interaction.
- Traffic-history storage.

## Core traffic model

`TrafficState` owns all current traffic targets. It indexes each target by
`TrafficTargetId`.

`TrafficTargetId` contains the normalized FLARM ID type and value. It is not
opaque. It is not session-local. Two observations identify the same target
when both fields are equal.

The normalized ID types are:

- Random.
- ICAO.
- FLARM.
- Other, with the protocol value.

The target value is the FLARM 24-bit identity value. A `PFLAA` observation must
contain both the ID type and the value. The core ignores the observation when
either field is absent.

Each internal `TrafficTarget` contains:

- `TrafficTargetId`.
- A position as `updraft_geo::LatLon`.
- Optional absolute altitude as `updraft_units::MslAltitude`.
- `TrafficType`.
- Optional true ground track as `updraft_units::Angle`.
- `TrafficAlarmLevel`.
- A stale flag.

The core keeps all quantities in their domain types. It uses
`updraft_units::Length` for relative positions and altitude differences. It
does not store unit-named scalar fields such as `altitude_msl_meters`.

`TrafficType` is a protocol-independent domain enum. It contains:

- Unknown.
- Glider.
- Tow plane.
- Helicopter.
- Skydiver.
- Drop plane.
- Hang glider.
- Paraglider.
- Piston aircraft.
- Jet aircraft.
- Balloon.
- Airship.
- UAV.
- Static obstacle.

An absent, reserved, invalid, or unsupported FLARM aircraft type becomes
`TrafficType::Unknown`.

The core stores the observation time with each internal target. It does not
publish that time to the frontend.

## Observation acceptance

The core processes each valid `PFLAA` sentence independently. One unusable
sentence does not stop the remaining sentences in the same bytes input.

The core accepts an observation only when all these conditions are true:

- The ID type is present.
- The ID value is present.
- Relative north is present.
- Relative east is present.
- A usable ownship position reference is available.

The core ignores an observation when relative east is absent. In this PFLAA
form, relative north contains an estimated distance instead of a north
coordinate. The observation does not define a map point.

Track is optional. The core accepts and publishes a target when track is
absent. Relative vertical position is also optional. The core publishes the
target without altitude when it cannot calculate absolute altitude.

An ignored observation does not delete or change an earlier valid target with
the same ID. The earlier target follows the normal freshness rules.

## Ownship reference selection

The core converts each accepted relative observation to an absolute position
at the time of receipt. It prefers ownship data from the external device that
sent the observation. It falls back to the displayed ownship data.

The core selects the horizontal and vertical references independently:

- For latitude and longitude, it uses a usable same-device ownship position.
  Otherwise, it uses the displayed ownship position.
- For altitude, it uses a usable same-device MSL altitude. Otherwise, it uses
  the displayed ownship MSL altitude.

The core ignores the observation when neither horizontal reference is usable.
It can still publish a position without altitude when no vertical reference is
usable.

The core adds the relative vertical value to the selected ownship MSL altitude.
The result is the target `updraft_units::MslAltitude`.

The core stores the resulting absolute position. It does not recalculate that
position when the ownship moves. A later observation replaces it.

## Multiple devices and replacement

All external devices write to the same `TrafficState`. The last accepted
observation in core input order wins for a `TrafficTargetId`. The source device
does not assign a priority.

A newer observation replaces the complete internal target and resets its
freshness timer. This rule also applies when a different device supplied the
previous observation.

If one logical input contains several observations for the same ID, the core
publishes only the final resulting target from that input.

A device disconnection does not remove its targets. The ordinary freshness
rules remove them unless another device supplies a newer observation.

The FLARM no-track flag does not change local traffic state or map rendering in
this slice.

## Freshness

The core uses the timestamp supplied with each input. It does not read a
process clock.

A target is fresh for the first 5 seconds after its latest accepted
observation. On the first tick at or after 5 seconds, the core marks the target
stale and publishes a complete upsert.

On the first tick at or after 30 seconds, the core removes the target and
publishes its ID as removed. Removal takes priority when one tick crosses both
thresholds.

A later accepted observation can make a stale target fresh again. The core
publishes the fresh target as a complete upsert.

These periods are core constants in this slice. OGN can add a different
freshness policy later. A future design can make the periods configurable.

## Tauri projection

The core converts each internal target to a Tauri wire type only when it
creates a traffic topic:

```rust
use crate::topic::LatLon;

pub struct PublishedTrafficTarget {
    pub id: String,
    pub position: LatLon,
    pub altitude_msl_meters: Option<f64>,
    pub traffic_type: TrafficType,
    pub track_degrees: Option<f64>,
    pub alarm_level: TrafficAlarmLevel,
    pub stale: bool,
}
```

`PublishedTrafficTarget` is the serialization boundary. It reuses the existing
wire `LatLon` for position. It uses explicit unit names for the other scalar
values and generates the corresponding TypeScript type. The conversion reads
degrees and meters from the core domain types.

The core converts each `TrafficTargetId` to one canonical wire string. The
frontend treats this string as an opaque target ID. The formats are:

- `random:000001`.
- `icao:ABCDEF`.
- `flarm:000123`.
- `other:7:000123`.

The value uses uppercase hexadecimal digits and a minimum width of six digits.
The `other` format includes the numeric ID-type value. The internal
`TrafficTargetId` remains structured and typed. Only the wire projection uses
the string form.

The generated TypeScript protocol uses `string` for published and removed
target IDs. It does not export the internal Rust ID type or ID-type enum.

## Traffic topic

`Topic::Traffic(TrafficUpdate)` carries one of two message forms:

```rust
pub enum TrafficUpdate {
    Snapshot(Vec<PublishedTrafficTarget>),
    Delta(TrafficDelta),
}

pub struct TrafficDelta {
    pub upserts: Vec<PublishedTrafficTarget>,
    pub removed: Vec<String>,
}
```

Each upsert is a complete wire projection. The protocol does not contain field
patches. Each removal contains only the canonical target ID string. One delta
does not contain the same ID in both `upserts` and `removed`.

A new subscriber first receives one `TrafficUpdate::Snapshot`. The existing
single driver task takes the snapshot and registers the subscriber without an
intervening core input. The protocol therefore does not need a revision
number.

After subscription, the core emits at most one traffic delta for each logical
input:

- A bytes input drains all complete NMEA sentences and emits one batch with all
  resulting target changes.
- An input with one changed target emits a batch with one upsert.
- A future structured OGN input can emit all its changes in one batch.
- A tick input emits stale upserts and removals caused by that tick.

The core emits no traffic message when an input does not change traffic state.

## Frontend store

The frontend owns one traffic store. A snapshot replaces the complete store.
A delta inserts or replaces every upsert and then removes every listed ID.
The store uses each published target ID directly as its map key. It does not
rebuild or parse target IDs.

The store is the authoritative frontend copy. It continues to process traffic
messages when the MapLibre source does not exist. When the map creates the
source, it reads one complete projection from the store.

The store has a small subscription API for mounted consumers. After the store
applies one traffic update, it calls each subscriber with the update, the
previous target map, and the current target map. The map component subscribes
when it mounts and unsubscribes when it is destroyed. The target maps remain
immutable after publication.

The GeoJSON projection creates one point feature for each target. Each feature
uses the published target ID as its GeoJSON feature ID. It includes these
properties:

- Traffic type.
- Track, as degrees or `null`.
- Absolute altitude, as meters or `null`.
- Alarm level.
- Stale state.

The projection does not add an icon name.

The initial snapshot uses `GeoJSONSource.setData()` with a complete feature
collection. For later deltas, new IDs use feature additions, existing IDs use
feature updates, and removed IDs use feature removals through MapLibre's
`GeoJSONSource.updateData()` method. The projection creates complete GeoJSON
features for snapshots and additions. It creates existing-target updates
directly from the wire target.

Each existing-target update replaces the geometry. It also writes every
traffic property through `addOrUpdateProperties`. Unknown track and altitude
values use `null`. The update does not use `removeProperties` or
`removeAllProperties`.

The map queues source operations in traffic input order. A subscriber callback
contains the previous and current immutable maps for that update. These maps
let the projection classify additions and updates. They also let failure
recovery rebuild the exact state after the failed update.

If an incremental source update fails, the frontend logs a warning. It rebuilds
the source from the current map for that update.

## Icon selection

A MapLibre `icon-image` expression maps each traffic type to an existing SDF
sprite:

- Unknown, skydiver, UAV, and static obstacle use `unknown`.
- Glider uses `glider`.
- Tow plane, drop plane, and piston aircraft use `aircraft`.
- Helicopter uses `helicopter`.
- Hang glider uses `hang-glider`.
- Paraglider uses `paraglider`.
- Jet aircraft uses `jet`.
- Balloon uses `balloon`.
- Airship uses `airship`.

The first slice does not add a skydiver sprite.

## Symbol layers

Two MapLibre symbol layers read the same traffic source.

The fixed layer contains:

- Balloons.
- Targets without track.

Its icons use viewport rotation alignment. They remain upright when the map
uses north-up, track-up, or another rotated orientation.

The directional layer contains all other targets. Its icons use map rotation
alignment. The style rotates each icon by the target's true ground track.

The icon mapping uses traffic type directly. The fixed-layer filter selects a
`null` track. The directional-layer filter selects a non-null track. The
GeoJSON does not contain an icon name or a separate rotation flag.

Text uses viewport alignment in both layers. Altitude labels remain upright
when the map rotates.

Both layers show icons and text when they collide with other symbols. MapLibre
does not hide a traffic icon or altitude label because of a collision. Higher
FLARM alarm levels use a higher symbol sort key within each layer. The fixed
and directional layer order controls which layer appears on top when their
symbols use the same pixels. This slice does not add alarm-specific layers.

## Traffic style

The SDF icon color represents the FLARM alarm level:

- No alarm uses the normal traffic color.
- Low alarm uses a caution color.
- Important alarm uses a warning color.
- Urgent alarm uses a danger color.

Stale targets keep the color for their alarm level and use reduced opacity.
Exact color, size, and opacity values are visual tuning. They do not change the
traffic data contract.

The text label shows absolute MSL altitude in meters. It includes the unit
symbol. The style shows no label when altitude is `null`.

This slice does not show relative altitude. A future setting can select
absolute or relative altitude without changing the core traffic position.

## Failure behavior

Invalid and incomplete observations affect only that observation. They do not
clear traffic state. A valid sentence in the same bytes input can still update
another target.

The core emits no error effect for an observation that lacks the fields
required by this slice. Existing NMEA and connection diagnostics remain
responsible for transport and framing failures.

The frontend keeps applying deltas when the map is unavailable. Map creation
uses the current store, so it does not depend on the timing of earlier map
updates.

## Testing

Core tests cover:

- Typed internal position, altitude, and track values.
- Same-device ownship reference selection.
- Fallback to displayed ownship data.
- Independent horizontal and vertical fallback.
- Absolute position and altitude projection.
- Missing identity fields.
- Missing relative north or east.
- Missing ownship position.
- Missing track.
- Missing altitude inputs.
- Unknown traffic-type normalization.
- Several targets in one bytes input.
- Several updates for one target in one bytes input.
- Latest-observation-wins behavior across devices.
- Device disconnection.
- Snapshot delivery.
- Complete upserts and removals.
- Freshness at the exact 5-second boundary.
- Removal at the exact 30-second boundary.
- A tick that crosses both freshness thresholds.

These tests use input timestamps. They do not use wall-clock waits.

Frontend tests cover:

- Snapshot replacement.
- Delta upserts and removals.
- Subscriber notification and unsubscription.
- Canonical target ID strings from the Rust wire protocol.
- Stable GeoJSON feature IDs that use the wire target IDs.
- GeoJSON feature projection.
- Feature additions, updates, and removals.
- Nullable track and altitude properties in complete feature updates.
- Traffic-type icon mapping.
- Fixed-layer and directional-layer filters.
- Track rotation.
- Alarm color and sort expressions.
- Stale opacity.
- Icon and text overlap settings.
- Initial source data and incremental source diffs.
- A complete source rebuild after an incremental update failure.

A focused map test verifies that both traffic layers read the traffic source.
Existing `updraft_nmea` tests remain responsible for PFLAA field parsing.
Higher layers do not repeat those parser tests.

Topic tests cover the conversion from typed core targets to
`PublishedTrafficTarget`. They cover each canonical target ID format. They also
cover the nested position fields, the remaining scalar field names, and the
serialized snapshot and delta forms.

Manual validation sends a scripted PFLAA stream through the desktop TCP
transport. It checks target creation, movement, stale appearance, and removal.
Android validation repeats the data path through a paired SPP device.
