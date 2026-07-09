# The Rust Core

Updraft is a native Rust modular monolith. One `App` owns the authoritative flight and domain state, while coarse domain modules own their own rules and storage. Tauri embeds the app directly in-process, and the axum server hosts the same Rust interface. The core has no UI, transport, or networking assumptions.

The architecture is deliberately ordinary: one process, one owner, direct typed method calls, and explicit native side effects. It does not use internal services, actors, an event bus, event sourcing, or a plugin framework.

## State Ownership

Rust owns state that must agree across every client:

- selected sensor values and flight state,
- aircraft, task, and navigation state,
- traffic and warnings,
- device connections and source selection,
- flight logging state,
- domain settings,
- active dataset identities and indexes.

Clients own presentation state such as the map viewport, open dialogs, selected panels, animations, and per-client layout. A secondary client observes the same authoritative flight state without sharing its presentation state with the primary.

Large imported datasets are managed by the data module. `App` holds identifiers and loaded indexes rather than embedding source files or map resources in the transactional state.

## Module Shape

The initial modules are intentionally coarse and may be adjusted as features reveal better boundaries:

```text
core/src/
├── app.rs
├── flight/
├── navigation/
├── traffic/
├── devices/
├── data/
└── protocol/
```

- **Flight** owns selected sensor values, flight-mode detection, aircraft state, warnings, and logging decisions.
- **Navigation** owns waypoints, tasks, final glide, reachability, and airspace evaluation.
- **Traffic** owns FLARM and OGN targets, source merging, aging, and relayed collision alarms.
- **Devices** owns connection state, capability detection, source priority, normalization, and outbound device commands.
- **Data** owns imported datasets, terrain access, dataset storage, and versioned map resources.

`App` coordinates operations that cross these boundaries with direct method calls. There is no internal message bus. Shared crates are limited to stable primitives and reusable parsers such as units, coordinates, polars, NMEA, CUP, and OpenAir. Domain modules remain in the core crate until an independently reusable library or strict dependency boundary justifies another crate.

## Application Interface

The application has a small Rust interface:

```rust
impl App {
    pub fn handle(&mut self, input: Input) -> Update;
    pub fn query(&self, query: Query) -> QueryResult;
    pub fn snapshot(&self) -> Snapshot;
}

pub struct Update {
    pub changes: Vec<Change>,
    pub effects: Vec<Effect>,
}
```

Commands, queries, changes, and effects are grouped by domain rather than collected into one flat list. The grouping keeps the public surface discoverable while preserving one entry point for hosts.

## Inputs and Runtime

One `CoreRuntime` owns `App` and handles inputs sequentially in FIFO order. Inputs are:

- user commands,
- normalized sensor observations,
- monotonic time advancement,
- results of completion-sensitive native effects.

Device parsers and platform sensors normalize their output before it reaches `App`. An observation carries its source, monotonic observation time, domain value, quality, and GPS timestamp where applicable. Vendor wire messages remain in the device layer and optional raw captures.

There is no general priority or coalescing scheduler. Commands, GPS fixes used for logging, warning transitions, and device-state changes are lossless. A specific high-rate replaceable input may gain a bounded policy only after measurement demonstrates a need for one.

## Time

Time advances through inputs containing a monotonic timestamp. `App` never reads a wall clock or a `Clock` trait. Production adapters create time inputs from the system clock, while tests and replay provide them directly.

GPS time remains part of position observations. It is used for flight records and is never conflated with monotonic scheduling time.

## Changes and Native Effects

Handling an input returns client-visible changes and native effects. Changes describe observable state and are safe to replace with a fresh snapshot. Effects request interaction with the outside world, for example:

```rust
Effect::PlayWarning(...)
Effect::WriteFlightLog(...)
Effect::SaveSnapshot(...)
Effect::FetchOgnTraffic(...)
Effect::SendToDevice(...)
```

The runtime executes effects with concrete Rust adapters. This is a small `match`, not a generic effect framework. Completion-sensitive operations return a typed result as another input. Fire-and-observe operations such as audio do not travel through UI subscriptions.

The flight module owns when logging starts, which IGC records are produced, their ordering, and resume state. An ordered native `FlightLogWriter` owns file creation, append, flush, durability, and filesystem errors. It remains independent of the webview and transport.

## Computation

Cheap derived values and warnings are computed synchronously after relevant inputs. This keeps ordering and warning latency visible in the normal control flow. The runtime measures handler duration and queue depth.

Background work is introduced per calculation only when profiling shows that synchronous execution threatens the input loop. A background job captures its domain inputs and a generation ID. Its result is accepted only if that generation remains current. There is no general staged pipeline or worker protocol.

## Determinism and Replay

Diagnostic recording captures normalized inputs after transport parsing and results returned by completion-sensitive effects. Replay disables live adapters and feeds the recorded sequence through `App::handle()` with recorded monotonic and GPS times.

If a calculation later moves to a background worker, its returned payload is recorded. Replaying recorded results is simpler and more portable than requiring byte-identical floating-point recomputation across platforms.

Replay is separate from persistence. Production state is restored from snapshots and flight logs, not by replaying every command ever issued.

## Queries and State Streams

Queries read authoritative state without changing it. Hosts route queries through the owning runtime so no concurrent mutable access is exposed.

A client state stream begins with a `Snapshot` and then delivers FIFO-ordered batches of `Change` values. Subscription creation and snapshot capture happen together inside the runtime loop, so no update can fall between them. A reconnecting client starts again with a fresh snapshot. There is no replay buffer or global application revision.

Bulk resources carry their own revisions because clients and hosts need them for cache invalidation.

## Protocol and Hosts

The Rust interface is transport-agnostic. Tauri invokes it directly in-process. The axum host maps HTTP requests and an ordered state stream onto the same commands, queries, snapshots, and changes. Tauri does not start a hidden HTTP server.

Commands that may be retried carry transport-level client and request IDs. Deduplication belongs to the host because its lifetime and persistence policy depend on the transport. Authentication, remote-client roles, CORS, and origin validation also remain host responsibilities.

Rust protocol types use `serde`, with matching TypeScript types generated through `ts-rs`. Generated files are committed, and CI fails if regeneration changes them unexpectedly. JSON is the starting encoding. A high-rate stream may gain a dedicated binary representation only after measurement justifies it.

Long-running operations return an operation ID and publish progress or completion changes instead of holding a transport request open indefinitely.

## Bulk Geodata

Bulk geodata never travels through JSON messages. The data module exposes opaque resource references:

```rust
pub struct ResourceRef {
    pub id: ResourceId,
    pub revision: ResourceRevision,
    pub media_type: MediaType,
}
```

The axum host resolves a resource ID to an authenticated HTTP route. The Tauri host resolves it to an `updraft://` URI. A resource revision is immutable and is published only after its bytes are available. MapLibre consumes the resolved URL as a normal vector-tile, raster-tile, or GeoJSON source.

## Snapshots and Resume

Configuration and resumable flight state are ordinary versioned documents, not an event store. A flight snapshot contains only durable in-flight state plus the active dataset identities and the last durable flight-log sequence. The runtime writes snapshots atomically through a native effect.

On startup, the runtime loads the latest valid snapshot and reconciles it with the existing IGC file before accepting live inputs. Imported datasets remain separate files and are reopened by identifier.

## Open Questions

- **Track resource updates:** how the ever-growing own-track resource is served without refetching its full history on every change.
- **Durability policy:** which snapshot and IGC transitions require an `fsync`-equivalent operation on each supported platform.
