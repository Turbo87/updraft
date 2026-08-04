# Flight data source selection

## Context

The core receives ownship data from every enabled external device. It also
receives ownship data from internal sensors. The current implementation writes
these observations into one displayed ownship state.

Two devices can report different flights at the same time. Each new observation
then replaces the preceding observation. The displayed ownship moves between
unrelated positions.

This design adds deterministic source selection for flight data. The core
selects one source independently for each data domain. It retains other source
data for immediate fallback.

## Supersession

This design supersedes the source-selection rules in
[`docs/design/devices.md`](../../design/devices.md) for GPS and pressure
altitude where the documents conflict.

The older document requires pilot notifications for source changes. This slice
does not add those notifications. It keeps source identity internal.

The [Devices screen design](2026-08-01-devices-screen-design.md) remains
authoritative for the Devices screen. This slice does not add source-priority
text or device-reorder controls.

The [FLARM traffic design](2026-07-30-flarm-traffic-map-design.md) remains
authoritative for traffic. This design does not replace its merge rules or its
ownship-reference rules.

## Scope

This slice adds source selection for these domains:

- GPS fix.
- Pressure altitude.

The GPS domain contains these values:

- Position.
- Optional MSL altitude.
- Optional ground speed.
- Optional track.
- Optional fix time.

This slice also changes the `Instruments` topic to group values by domain. It
adds pressure altitude, fix time, and stale state to that topic.

This slice does not add these items:

- Wind ingestion or source selection.
- TAS or IAS ingestion or source selection.
- GPS precision or Android accuracy.
- Satellite count or fix-quality metadata.
- Derived speed, track, vertical speed, or acceleration.
- QNH-adjusted barometric altitude.
- User-configurable freshness limits.
- Source identity in frontend topics.
- Pilot notifications for source changes.
- Source-priority text or reorder controls.
- A generic source-selector framework.
- New FLARM traffic behavior.

Wind and TAS/IAS can become separate domains in later designs. A later wind
design can define a different fixed freshness limit. Freshness limits must not
become user settings.

## Terms

- **Source**: One external device or one internal sensor input.
- **Candidate**: The retained values for one source and one domain.
- **Anchor**: The value that makes a source eligible for one domain.
- **Current**: A selected domain whose anchor is fresh.
- **Last known**: The frozen snapshot that was selected before its anchor became
  stale.
- **Unavailable**: A domain that has no current or retained selected snapshot.
- **Ingestion time**: The monotonic core timestamp that controls freshness.
- **Fix time**: Optional UTC time data reported by a GPS source.

## Source priority

The persisted external-device list defines source priority. The core checks
enabled external devices from first to last. Internal sensors have lower
priority than all external devices.

The core selects each domain independently. One source can provide GPS while a
different source provides pressure altitude.

The first eligible fresh source wins. A recovered higher-priority source
immediately preempts the selected lower-priority source. The core does not use
hysteresis or a recovery delay.

A disconnected source keeps its priority position. Its last valid data remains
eligible until it becomes stale. A reconnect can refresh the data before that
transition.

The core retains candidates for unselected sources. A lower-priority fresh
candidate is therefore available when the selected source becomes stale.

## Time and freshness

GPS and pressure altitude use one fixed 3-second freshness limit. The limit is
not a user setting.

A value is fresh while its age is less than 3 seconds. A value becomes stale
when its age is 3 seconds or more.

Freshness uses a monotonic ingestion time. GPS fix time does not control
freshness.

The driver assigns the ingestion time when it applies an input to the core. It
does not use the time when the producer added the input to the driver queue.
Queue delay therefore does not reduce the freshness period.

An NMEA sentence can span byte inputs. The decoder retains the timestamp of the
input that contained the first byte of the sentence. Accepted fields from that
sentence use this timestamp.

A repeated valid value refreshes its ingestion time. The new value can equal
the old value. An absent or invalid field does not refresh its ingestion time.

Applied inputs and configuration changes cause immediate reselection. The
existing fixed core tick handles time-based expiry. The first tick at or after
the 3-second boundary applies the expiry transition. The core publishes only
when the final frontend projection changes.

## Core state

The core stores each candidate value with its ingestion time:

```rust
struct Timed<T> {
    value: T,
    ingested_at: Timestamp,
}
```

The core uses one shared representation for the selected state:

```rust
struct Selected<T> {
    source: SourceId,
    ingested_at: Timestamp,
    value: T,
}

enum DomainState<T> {
    Unavailable,
    Current(Selected<T>),
    LastKnown(Selected<T>),
}
```

`SourceId` identifies an external device or an internal source. It remains an
internal core value.

GPS and pressure altitude use separate candidate types. They also use separate
selection functions. The implementation does not add a generic selector,
policy trait, callback, or selector object.

For GPS, `Selected::ingested_at` contains the selected position ingestion time.
For pressure altitude, it contains the selected altitude ingestion time.

The transition to `LastKnown` retains the original anchor ingestion time. The
transition does not refresh the value.

## Input processing

One byte input can contain several complete NMEA sentences. The core processes
all complete sentences in sentence order. It updates all candidates before it
selects the affected domains.

After the complete byte input, the core reselects each affected domain. It
publishes at most one final `Instruments` state for that input. Intermediate GPS
fixes from the same input do not enter the future post-selection derivation
stage.

A valid sentence can update one domain while another sentence in the same input
is invalid. The invalid sentence does not prevent the valid update.

An internal GPS input contains one structured observation. The core updates its
source candidate and then reselects GPS.

A tick can expire several candidate fields and domains. The core applies all
expiry transitions before it calculates the final `Instruments` projection for
that tick. It publishes only when that projection changes.

## GPS candidate

Each source owns one mutable GPS candidate:

```rust
struct GpsCandidate {
    position: Option<Timed<LatLon>>,
    altitude: Option<Timed<MslAltitude>>,
    ground_speed: Option<Timed<GroundSpeed>>,
    track: Option<Timed<Track>>,
    fix_time: GpsTimeCandidate,
}
```

The exact speed and track type names can follow existing core conventions. All
fields use typed domain values.

RMC and GGA sentences from one source update the same candidate. The core does
not pair sentences by fix time. It updates each accepted field independently.

An Android location input updates the same normalized fields for the internal
GPS source. Existing Android altitude conversion continues to produce MSL
altitude before source selection.

### Observation acceptance

The core accepts an RMC sentence when both these conditions are true:

- Its status is Active.
- Its optional mode is not explicitly Not Valid.

The core accepts a GGA sentence when its fix quality is not Invalid.

The core ignores a complete invalid sentence. That sentence updates no
candidate field and no ingestion timestamp. The earlier valid candidate follows
the normal freshness rules.

An accepted sentence updates each valid field that it contains. It can update
optional fields when it has no position. An absent field preserves its previous
value and ingestion time.

### Eligibility and optional fields

Position is the GPS anchor. A source is eligible for GPS selection only when it
has a fresh valid position.

Altitude, ground speed, track, and fix time cannot make a source eligible. They
also cannot extend the lifetime of the GPS domain.

The selected GPS snapshot uses values from one source. The core never fills a
missing GPS field from another source.

Each optional field has its own 3-second freshness. A stale optional field
becomes absent while the selected position remains current. The selector does
not fall back to another source for that field.

When no source has a fresh position, the GPS domain freezes the last published
GPS snapshot as `LastKnown`. Optional fields do not continue to expire inside
that frozen snapshot.

## GPS fix time

GPS fix time is optional domain data. It does not contain or replace the
monotonic ingestion time.

Each GPS candidate stores these time forms:

```rust
struct GpsTimeCandidate {
    full: Option<Timed<UtcInstant>>,
    time_only: Option<Timed<UtcTime>>,
}
```

The inputs update the candidate as follows:

- RMC with a date and time updates `full`.
- Android `Location.getTime()` updates `full` from Unix epoch milliseconds.
- RMC without a date updates `time_only`.
- GGA updates `time_only`.

The core keeps both forms when both are available. A fresh full instant is
authoritative. A time-only observation cannot replace it. The core still
caches the time-only observation for later use.

This rule prevents normal RMC and GGA order from alternating the published time
form. It also prevents a GGA time after midnight from using an RMC date that
still describes the preceding day.

When `full` is absent or stale, the core selects the latest fresh `time_only`
value. When both forms are stale, the selected GPS snapshot omits fix time.

The core maps an NMEA leap second at `23:59:60.xxx` to `23:59:59.xxx`. This
rule applies to full instants and time-only values. The conversion preserves the
fractional milliseconds. The maximum time-only value is `86_399_999`
milliseconds since midnight. A converted full instant can duplicate the
preceding timestamp.

## Pressure altitude

Pressure altitude is a separate source-selection domain. It represents altitude
at the standard pressure of 1013.25 hPa.

This slice accepts pressure altitude only from parsed PGRMZ sentences. The core
accepts every successfully parsed PGRMZ altitude. It ignores the PGRMZ
`fix_dimension` field.

Other parsed pressure-altitude sentence types do not update this domain in this
slice. GGA MSL altitude remains part of the GPS domain.

Each source stores the PGRMZ value as `Timed<PressureAltitude>`. The core does
not store a plain meter value inside the candidate.

Pressure altitude is its own anchor. A fresh value makes the source eligible
for this domain.

A future stage can derive QNH-adjusted barometric altitude after source
selection. Pressure altitude and QNH-adjusted altitude do not select sources
independently.

## State transitions

The initial state for each domain is `Unavailable`.

When an eligible source exists, the first fresh source in priority order becomes
`Current`. A fresh higher-priority source immediately replaces a lower-priority
source.

When the selected source becomes stale, the core selects the next fresh source.
When no source is fresh, the last published selected snapshot becomes
`LastKnown`.

The core does not replace `LastKnown` with a different stale source. This rule
applies when another stale source has a newer observation.

`LastKnown` remains for the rest of the core session. Fresh selected data
replaces it. An applicable explicit configuration change can clear it. An app
restart always clears it. The core does not persist candidates or last-known
snapshots.

For example:

- Source A supplies a GPS position at 10 seconds.
- Source B supplies a GPS position at 11 seconds.
- Source A remains selected while it is fresh.
- Source A becomes stale at 13 seconds. Source B becomes selected.
- Source B becomes stale at 14 seconds. Its displayed snapshot becomes last
  known.

## Configuration changes

Disable, delete, and edit operations discard all cached domain data for the
affected external device. The core immediately reselects each affected domain.

If the operation removes the selected source and no fresh fallback exists, the
domain becomes `Unavailable`. The core does not retain data from the removed
source as `LastKnown`.

Reordering does not discard candidate data. It causes immediate reselection
with the new priority order. When no source is fresh, reordering does not switch
the current last-known snapshot to a different stale source.

Enabling a device does not restore its discarded cache. New accepted data must
make that source eligible.

## Tauri projection

The core projects typed domain state to scalar wire values only when it creates
an `Instruments` topic.

The generated TypeScript contract has this shape:

```ts
type FixTime =
  | {
      type: "utcInstant";
      unixMilliseconds: number;
    }
  | {
      type: "utcTimeOfDay";
      millisecondsSinceMidnight: number;
    };

type Instruments = {
  gps: {
    position: LatLon;
    altitudeMeters: number | null;
    groundSpeedMetersPerSecond: number | null;
    trackDegrees: number | null;
    fixTime: FixTime | null;
    stale: boolean;
  } | null;
  pressureAltitude: {
    meters: number;
    stale: boolean;
  } | null;
};
```

A null domain represents `Unavailable`. A non-null domain with `stale: false`
represents `Current`. A non-null domain with `stale: true` represents
`LastKnown`.

The projection converts typed altitude, speed, and track values to the explicit
wire units. It does not publish source identity or ingestion timestamps.

## Instruments topic

The `Instruments` topic remains a complete snapshot. The core emits it only
when the frontend projection changes.

An identical valid observation refreshes the candidate ingestion time. It does
not cause an unchanged topic emission.

An internal source change also produces no topic when both complete frontend
projections are equal. The core still records the selected source change.

One byte input, structured GPS input, configuration input, or tick emits at most
one `Instruments` topic.

A new subscriber receives the current complete `Instruments` snapshot through
the existing topic-subscription behavior.

## Frontend presentation

The map debug overlay adds these values:

- GPS fix time.
- GPS stale state.
- Pressure altitude.
- Pressure-altitude stale state.

The overlay formats altitude and speed with the configured display units. It
formats both fix-time variants for people.

The normal ownship marker continues to render a last-known GPS position. Its
style does not change in this slice. The frontend does not add a broader stale
presentation.

The Devices screen does not change. It does not show selected sources or
describe list order as source priority.

The debug overlay is temporary inspection tooling. It does not require
dedicated automated tests.

## FLARM traffic

FLARM traffic remains a merged domain. It does not use this source selector.

The core keeps the existing same-device ownship-reference preference for each
PFLAA observation. It uses the displayed selected GPS only as the existing
fallback. This design does not change target identity, merge, freshness, or
publication behavior.

## Future derivation

Future kinematic derivation runs after GPS source selection. It consumes only
selected `Current` GPS fixes.

Future derivation resets its history in these cases:

- The selected GPS source changes.
- GPS changes from `Current` to `LastKnown`.
- GPS changes from `Current` to `Unavailable`.

A later current fix starts new history. This rule also applies when the same
source returns after a stale gap.

## Automated tests

Implementation follows red-green-refactor. Core tests use synthetic timestamps.
They do not sleep or read a process clock.

Core tests cover these behaviors:

- Independent GPS and pressure-altitude source selection.
- External-device priority and internal-sensor fallback.
- Immediate preemption by a recovered higher-priority source.
- Freshness below and at the exact 3-second boundary.
- Fallback from source A to a still-fresh source B.
- Transition from source B to its frozen last-known snapshot.
- No switch to a different stale source.
- Disconnect grace behavior.
- Invalid RMC and GGA grace behavior.
- Identical observations that refresh freshness.
- GPS eligibility based only on position.
- Independent expiry of optional GPS fields.
- Optional field updates from a sentence without a position.
- No optional field values from another GPS source.
- RMC and GGA updates to one source candidate.
- Full-instant and time-only precedence.
- Full-instant expiry with fresh time-only fallback.
- Leap-second normalization.
- One final selection after all sentences in one byte input.
- First-byte ingestion time for a sentence that spans inputs.
- Tick-driven expiry at the exact boundary.
- Immediate reselection after configuration changes.
- Cache removal after disable, delete, and edit operations.
- Reordering with retained candidates.
- PGRMZ ingestion with ignored `fix_dimension`.
- Suppression of unchanged `Instruments` topics.
- Internal source changes with identical frontend projections.

The Rust and TypeScript compilers verify the typed core and wire boundaries.
Higher layers do not repeat parser or core behavior tests.

## Manual acceptance

The first manual test configures two TCP replay sources that contain different
flights. Both sources remain connected.

The tester confirms these results:

- The higher-priority fresh source controls GPS without position jumps.
- The lower-priority source takes control after the first source becomes stale.
- The first source takes control again after it resumes valid fixes.
- A disconnected source remains selected until its data becomes stale.
- The final selected position remains visible with stale state after all
  sources stop.

The second manual test supplies GPS from one source and PGRMZ from another. The
debug overlay must show both domains. It must show their stale states
independently after their inputs stop.

## Validation

The implementation must pass the relevant workspace checks:

```bash
cargo fmt --all --check
cargo test -p updraft_core
cargo clippy -p updraft_core --all-targets --all-features -- -D warnings
pnpm check
```

The complete continuous-integration checks must also remain green.
