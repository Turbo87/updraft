# The Rust Core

The core is a plain Rust library that owns all application state and computation. It is a single-owner state machine that performs **no I/O, spawns no threads, and reads no clocks**: the host feeds it one input at a time, and it returns the resulting state changes plus **effect requests** as data. A thin **runtime** — shared by every embedding: the Tauri shell, the axum server, a unit test — owns the input queue, the worker threads, and all I/O; it executes the effects and feeds their results back as inputs.

The shape is deliberately boring and well-trodden: journaled inputs into a single-writer state machine (as in the LMAX architecture), a sans-IO surface (as in `quinn-proto`, `rustls`, or `str0m`), and determinism as an architectural constraint so that simulation testing pays off (see [testing.md](testing.md)). It is also a deliberate departure from XCSoar's blackboard chain, which copies state snapshots between device, merge, and calculation threads. Alternatives considered and rejected are listed at the [end](#alternatives-considered); the full decision record with claim verification lives in [core-architecture-comparison.md](../core-architecture-comparison.md).

## The Application Interface

```rust
impl App {
    /// Apply one input; the only mutation entry point.
    pub fn handle(&mut self, input: Input) -> Update;
    /// Read-only queries against current state.
    pub fn query(&self, query: Query) -> QueryResult;
    /// Full state for a newly subscribing client.
    pub fn snapshot(&self) -> Snapshot;
}

pub struct Update {
    pub changes: Vec<Change>,                  // client-visible state changes
    pub effects: Vec<Effect>,                  // requests to the outside world
    pub next_deadline: Option<MonotonicTime>,  // earliest pending timer (lands with core-time)
}
```

Inputs, changes, and effects are enums grouped by domain, not one flat list. The core crate never depends on tokio, rayon, or any I/O library — the runtime lives outside it, so the "no threads in the core" rule is enforced by the dependency graph, not by convention. Whole-flight integration tests are a plain loop over `handle()`: no async runtime, no sleeps, no flakiness surface.

The runtime pumps the input queue into `handle()`, fans `changes` out to subscribers, executes `effects` on worker threads / the I/O thread / async tasks, and feeds every result back in as an input. Hosts contribute only transport bindings and platform adapters on top of it.

## State

One state struct owns everything the application knows: current fix, sensor values (vario, airspeed, altitude sources), the traffic table, the active task, computed values (wind, MacCready, final glide), settings, and device connection states. It is mutated **only** through `handle()`, with no shared mutable state across threads.

"One struct" is an ownership rule, not a monolith: the struct is composed from **per-domain modules** (flight, navigation, traffic, devices, settings, …), each owning its sub-state, the inputs it consumes, its reducer, and its change payloads. Mode-ful subdomains (flight modes, the warning lifecycle, device connection lifecycles) are explicit little state machines — an enum with transition methods — never loose booleans.

The core owns **authoritative domain state**; clients own presentation state (map viewport, open dialogs, layout). A secondary client observes the same domain state without sharing its presentation state with the primary (see [multi-client.md](multi-client.md)).

**Reference data is not state.** Loaded datasets — airspace, waypoints, terrain — are immutable once loaded and held as cheap shared handles (`Arc`). Loading a file builds a new dataset and swaps the handle; the swap is the state change, the dataset itself never mutates. Workers on any thread can hold a dataset without locking.

## Inputs

Everything enters as messages through `handle()`:

- user commands (from the transport layer),
- normalized sensor observations (from connection threads, see [devices.md](devices.md)) carrying source, monotonic observation time, typed value, and GPS timestamp where applicable, delivered at adapter-bounded rates,
- clock advancement,
- results of completed effects: I/O completions and computation results (see below).

Device connections run on their own threads/tasks in the runtime and are dumb pipes: bytes in, parsed messages out, onto the queue. The core is therefore a **pure function of its input sequence**: record the input stream and any bug is replayable deterministically. This is the foundation of the testing story (see [testing.md](testing.md)).

The input queue is a **plain bounded FIFO** owned by the runtime, and its full-queue policy is **block the producer, never drop**: FLARM collision alarms share this queue, kernel socket buffers absorb the backpressure, and the reader threads are dumb pipes that can wait. There are no priority lanes and no coalescing — real input rates (~10 Hz GPS, baud-limited FLARM bursts) are a percent-level duty cycle for microsecond-scale handlers, so queues never build and a command waits at most milliseconds behind a burst; priority could not improve the real latency bound (the slowest synchronous stage) anyway, and lane designs add a bug class — every future message kind must be correctly classified coalescable-or-not, or alarm edges get silently dropped. A superseded sensor value is a cheap state overwrite: the state itself is the coalescer. Should command latency ever measurably matter, the escape hatch is a second command queue with a biased poll — a runtime-side change that leaves the core and the recording format untouched. Recording captures inputs in the order the runtime feeds them to `handle()`, so the recorded sequence is exactly what the core saw.

## Time Is an Input

The core never reads a clock. Adapters stamp inputs with **monotonic timestamps** from a single process-wide epoch; tests and replay provide timestamps directly. There is no `Clock` trait — a trait the core calls is a hidden read, while a timestamp in the input is data in the replay log. Replay at 100x real time is a unit-test primitive, not a special mode: a four-hour flight runs in seconds.

**Monotonic time vs. GPS time.** Monotonic time drives scheduling only (timers, debouncing, warning lookahead). It is never conflated with fix timestamps: IGC records and all flight data use GPS time carried in the fixes themselves. Replay therefore reproduces original GPS timestamps regardless of playback speed.

**Timers are core state.** Debounce and scheduling timers live in the core as a priority queue of (deadline, timer id), armed by the update loop and drained deterministically as injected time advances, with a fixed tie-break against same-tick inputs. `Update.next_deadline` carries the earliest pending deadline so the host arms one timer and sleeps precisely instead of ticking blindly. There is no second scheduler implementation that replay has to keep in lockstep.

## Computation

Derived values are computed **synchronously** inside `handle()`, at cadences matched to their cost:

- **Every fix**: ground speed, track, GPS/pressure altitude fusion, AGL lookup
- **Every vario update**: speed-to-fly
- **~1 Hz** (rate-limited via timers, always against the newest state): airspace-proximity lookahead, nearest-waypoint ranking, final glide
- **Async workers**: live scoring / task optimization, glide-reach polygon, wind-estimation refinement

This table is the domain's cost model: the first three classes are cheap enough that recomputing them is cheaper than tracking whether they need recomputing. **Warnings are synchronous** — warning generation (computed airspace proximity, relayed FLARM collision alarms, see [traffic.md](traffic.md)) runs inside `handle()`, never on the async path. The working latency budget is **a warning input becomes an audible alert in under 100 ms**; the runtime instruments handler duration and queue depth from the first sensor adapter onward, and a calculation that threatens the budget moves to a worker.

**The async seam exists from day one**, because some calculations are known to be too slow for the update path (live score optimization over the growing trace is the canonical case). The core emits a **compute effect** carrying a snapshot of the inputs the job needs; the runtime runs it on a worker thread and the result comes back as an input.

Scheduling per worker kind is a **dirty flag plus at most one job in flight**. Relevant input changes set the flag; when the worker is idle and the flag is set, the core snapshots the inputs, emits the effect, and clears the flag. A completed result is applied by default — discarding a slightly-stale result only leaves an even staler one on screen — and if the flag was set again while the job ran, the next job spawns immediately, so the display converges. Semantic invalidation is an **epoch counter** per worker kind: breaking changes (task replaced, position discontinuity beyond a threshold — teleports are a supported interaction via simulator drag and replay seek) bump the epoch, a result stamped with an old epoch is dropped, and the UI shows the affected value as recomputing. A worker panic is caught by the runtime and converted into a `JobFailed { kind, epoch }` input — with one-in-flight bookkeeping, a lost completion would otherwise wedge that worker kind for the rest of the flight.

**Workers may retain state.** The runtime owns one persistent worker per kind, `run(&mut self, inputs) -> result`, so an optimizer can keep incremental structures over the growing trace instead of re-solving from scratch. One-in-flight already serializes all access to that state, and an epoch bump doubles as the reset signal. Worker state is a host-side acceleration cache — never core state, never snapshotted; after a restart it is cold and the first round is slower, which resume must tolerate anyway. A job's input snapshot can be incremental too ("track points since sequence N", see _The Bulk Geodata Path_).

I/O-bound work from the outside world, such as pulling OGN traffic, is an I/O effect executed by async tasks in the runtime; results re-enter as inputs like everything else. Whether a worker parallelizes internally is its own business — the core never sees it.

## Effects

Handling an input returns effect requests as plain data, executed by a small `match` in the runtime — not a generic effect framework:

```rust
Effect::PlayWarning(...)     // fire-and-observe: native audio adapter
Effect::WriteFlightLog(...)  // dedicated I/O thread
Effect::SaveSnapshot(...)    // dedicated I/O thread
Effect::Compute(...)         // per-kind worker (see Computation)
Effect::FetchOgnTraffic(...) // async task
Effect::SendToDevice(...)    // connection writer
Effect::PublishResource(...) // runtime resource store (see The Bulk Geodata Path)
```

Completion-sensitive effects deliver a typed result back as an input; fire-and-observe effects (audio) do not travel through client subscriptions at all, so the safety-critical warning→audio path never shares machinery or drop policies with UI streaming — and is assertable in a unit test with no transport. **Effect execution never blocks the runtime loop**: file writes run on a dedicated I/O thread, so a slow flash write can never stall the warning pipeline. Long-running operations return an operation ID immediately and report progress and completion as ordinary changes instead of holding a transport request open.

## Determinism & Replay

Recording and replaying an input sequence reproduces the exact same state evolution, which is the foundation for simulation mode, IGC replay, demo mode, and regression testing alike. Because worker results re-enter as inputs, **everything is recorded verbatim** — external I/O results (OGN responses, weather fetches) and worker payloads alike. One rule, no special cases: replay is a fold over the log; it never re-runs a worker.

The alternative — recording pure results as completion markers and recomputing them during replay — was considered and rejected: it makes bit-exact float determinism load-bearing forever (`sin`/`cos`/`atan2` route to the platform libm, whose results differ between glibc, musl, bionic, and Apple's implementation — a cockpit recording from an ARM phone would not replay faithfully on an x86 dev machine), it ties recordings to the algorithm version (improve the reach algorithm and every field recording that reproduced a bug stops reproducing it), and it makes the replayer itself heavier. Worker payloads dominate log size, so they go to a **compressed sidecar stream** next to the primary input log; derived geometry compresses extremely well, and if storage is tight the sidecar is droppable.

- **Determinism scope:** bit-exact replay is guaranteed for the same build on the same platform. Cross-platform replay is input-exact: the state evolution replays identically wherever it depends only on recorded inputs, and test assertions on computed floats use tolerances.
- **Recompute-and-compare is a CI verification mode, not the replay mechanism.** Recorded flights are replayed with the workers recomputing live (rebuilding stateful workers by re-running their invocation sequence in order — the per-kind total order makes this well-defined) and outputs diffed against the recorded payloads by hash, reporting divergence at the first differing result ("diverged at input #N, worker `reach`"). A mismatch flags a nondeterminism regression or a genuine behavior change.
- Determinism hygiene inside workers (sequential float folds, ordered maps or fixed hashers instead of bare `HashMap` iteration) remains a guideline because it keeps golden tests stable across runs — it is not a load-bearing replay invariant. `f64::mul_add` is IEEE-exact and fine; the only genuinely platform-dependent operations are the libm transcendentals.
- A recording replays the *recorded* behavior faithfully forever. As a fixture against evolved core logic it is an approximation — new code may spawn different jobs than the recorded results answer — which is fine for regression fixtures, but worth knowing.

**Resume is a new baseline.** After a mid-flight restart the core seeds its state from a snapshot (see _Snapshots & Resume_ below), so replay tooling supports "seed from snapshot X, then replay the log from position N" as a first-class mode alongside replay-from-empty. When input recording is enabled it is written incrementally like the IGC log, so a crash leaves behind exactly the artifact needed to reproduce it.

## Outputs

Clients observe the core through **one state stream**: on subscribe, a full `Snapshot`, then FIFO-ordered batches of `Change` values. Two invariants carry the design and must be pinned by tests in the runtime:

- **Atomic subscribe.** Subscription registration and snapshot capture happen together inside the runtime loop, so no change can fall between them — a late subscriber's snapshot already contains everything submitted before it.
- **Reconnect is resubscribe.** A reconnecting client starts over with a fresh snapshot. There is no replay buffer, no sequence bookkeeping, and no generic diff/patch format (JSON Patch and friends buy nothing at these payload sizes).

A slow subscriber never grows an unbounded buffer: when its bounded buffer overflows, the subscription is dropped — observably (logged and counted, distinct from normal disconnect) — and the client recovers by reconnecting, which is safe and cheap precisely because subscribe delivers a fresh snapshot. `Change` values are grouped by domain, which is a coarse topic key: per-client filtering (a vario-only repeater, a tablet without the PFD page) can be added host-side later without touching the core. For that key to stay useful, high-rate instrument changes (attitude, live vario) get groups of their own as they land, instead of sharing one with low-rate navigation state.

**The `Snapshot` stays small.** It carries current values and active sets only — never datasets, histories, or time series. The altitude trace, climb statistics, waypoint database, and their like are served as reference resources (see _The Bulk Geodata Path_) or answered by queries; a snapshot that grew with flight duration would break the cheap-reconnect contract that everything above relies on.

Change payloads follow a small taxonomy — this shapes payload semantics, it is not per-topic stream machinery:

- **Last-value** (`position`, `wind`, `mac-cready`, `final-glide`, `vario`): each change carries the complete current value and replaces the previous one.
- **Keyed collection** (`traffic`, `devices`): upserts and removals keyed by a stable ID.
- **Events plus active set** (`airspace-warnings`): edge-triggered events (raised, escalated, cleared, acknowledged), because banners fire on transitions, plus the currently active set in the snapshot.
- **Reference** (`reach`, `track`): a version counter and a URL, with the payload fetched through the bulk geodata path (see below).

Moving objects (`position`, `traffic`) are published as **kinematic state vectors** — position, track, ground speed, turn rate, climb rate, and GPS timestamp, not bare coordinates. Clients extrapolate to render time (see [frontend.md](frontend.md) and [traffic.md](traffic.md)), so displays stay smooth at any frame rate, every client extrapolates from identical states, and no per-frame traffic crosses the transport.

Warning **audio** is not a subscriber: it is driven by effects (see _Effects_ above), so it works regardless of transport or webview state (see [tauri.md](tauri.md)).

## The Message Protocol

The core is interacted with through a small, well-defined surface:

- **Requests** are a single request/response mechanism. Every request returns a result or a typed error. _Commands_ are the mutating requests (`SetMacCready`, `AdvanceTaskTurnpoint`, `AcknowledgeAirspaceWarning`, …), _queries_ the read-only ones (either full snapshots or scoped selections, e.g. `query_at(lon, lat, radius)` for the map's "What's here?" interaction). A command is not fire-and-forget: loading a file can fail, a setting can be out of range, and the caller needs to know.
- **The state stream** delivers the snapshot+changes sequence (see _Outputs_ above).

Every request type is tagged as mutating or read-only, because three things depend on the distinction:

1. **Replay:** commands are inputs and enter the recorded input log. Queries never mutate and are never recorded.
2. **Permissions:** client roles are checked per mutating request (see [multi-client.md](multi-client.md)).
3. **Transport mapping:** queries map to GET and are freely retryable. Commands map to POST; **retry deduplication is owned by the host** — a retrying transport attaches client/request IDs and deduplicates them itself, so the core protocol does not carry retry bookkeeping for transports that never retry.

Errors are one shared type with stable, machine-readable codes so the frontend can localize and match on them, generated to TypeScript like every other protocol type.

The protocol contract is **host-agnostic**: the same messages flow over HTTP or a direct function call in a unit test, and feature code never knows which. In production there is exactly **one transport** — the axum server, standalone or embedded in the Tauri shell (see [server.md](server.md) and [tauri.md](tauri.md)) — so the contract tests and the Playwright suite exercise the same path that ships on every platform.

### Encoding

**Requests** are low-frequency and latency-insensitive (load a file, change a setting, connect a device, "what's at this point?"). These use plain JSON, defined as Rust types with `serde`. The matching TypeScript types are **generated via `ts-rs`**, so the two sides cannot drift. Generated types are committed, and CI fails if a regeneration would change them (golden-file check).

**The state stream** is plain JSON over a single multiplexed SSE stream (see [server.md](server.md)). The change encoding can evolve per domain group later without touching consumers; a compact binary channel for a high-rate stream (live audio-vario signal) is a hypothetical to revisit only on measured need.

## The Bulk Geodata Path

**Bulk geodata never travels through the message channel.** Pushing map tiles, airspace geometry, the glide-reach polygon, or the flight track through the protocol and into MapLibre is the biggest serialization trap, so the core exposes that data as ordinary HTTP routes in the axum server — the same routes on every platform, since the Tauri shell embeds the server (see [tauri.md](tauri.md)). MapLibre consumes them as normal sources (vector/PMTiles basemap and terrain, GeoJSON overlays).

A webview custom URI scheme was considered for the native apps and rejected: wry's custom protocols cannot stream response bodies (wry #1404), and Android's system webview fails follow-up byte-range requests against custom-scheme responses, which breaks PMTiles exactly on the primary platform. Real HTTP over loopback has none of these problems.

For this geodata the frontend only ever handles **references**: source URLs plus version counters. When the reach polygon is recomputed, the core bumps the version in a `reach` change and the frontend refreshes the source. No geometry crosses the message channel.

**Resource bytes live in a runtime-owned resource store.** When the core (or one of its workers) produces a new payload — a reach polygon, a batch of track segments, an overlay — it emits a publish effect carrying the bytes behind a cheap shared handle plus their version; the runtime keeps them in the store, and the host's routes serve them from there directly. Serving bulk data therefore never enters the input loop or the query path, and the state stream still carries only `{version, url}`. A version is published only after its bytes are available.

**The own track** is the one ever-growing resource, and it is never re-served in full: the core serves it as an immutable log with a monotonic sequence number per appended segment and a stable feature ID. The `track` change carries the latest sequence; the frontend fetches `?since=<seq>` for just the new segments and applies them with MapLibre's `GeoJSONSource.updateData` diff (which requires exactly the stable feature IDs the sequence numbering provides). A fresh or reconnecting subscriber fetches without `since` and receives the consolidated track (old geometry may be simplified server-side at that point); after that, tail fetches only.

## Snapshots & Resume

The core persists flight-critical state continuously: periodic snapshots of in-flight state with atomic writes, alongside incremental IGC logging. On startup the core detects an interrupted flight and resumes logging automatically. Storage details live in [data.md](data.md).

The snapshot is a small, **explicitly versioned** struct of flight-critical state (active task, logging status, device configuration) — never a serde dump of the whole state struct, so the state can evolve freely without snapshot-migration pain. On version mismatch after an app update the core degrades gracefully — fresh state, with IGC logging still resumed from the incremental log — rather than attempting migration or crashing. The IGC log is the flight record; the snapshot is convenience state and can always be discarded.

Snapshot and IGC writes are effects on a dedicated I/O thread (see _Effects_); completion and failure come back as inputs, and a failed snapshot write becomes a user-visible warning.

## Alternatives Considered

Rejected macro-architectures, kept as rationale:

- **Actor frameworks** (actix, ractor): buys supervision and per-actor mailboxes, but destroys the single totally ordered input sequence — with N actors, reproducing a bug means reproducing a schedule, not a log. This design already is one actor with many dumb producers.
- **ECS** (headless bevy_ecs): earns its complexity with many homogeneous entities; this state is heterogeneous singletons plus one small keyed table, and parallel system schedulers fight replay determinism.
- **Demand-driven incremental computation** (salsa): the right tool when consumers _pull_ queries over a large input database (rust-analyzer). This pipeline is the opposite shape — push-driven, fixed cadences, tiny hot state.
- **A declared computation graph** (nodes with declared inputs, dirty propagation, topological recompute): framework weight for a handful of cheap synchronous values plus a few async jobs; its per-field change detection is the hidden cost. The dirty-flag/epoch scheme above covers the async jobs in a fraction of the machinery.
- **Full CQRS / event sourcing**: the input sequence already _is_ the event log and state is its fold; a second domain-event layer adds schema burden and buys nothing replay doesn't provide.
- **Reactive signal-graph frameworks** (futures-signals, leptos-style runtimes): solve UI-driven recomputation with dynamic subscriber sets and bring their own scheduling that would have to be held in lockstep with replay.
- **CRDTs for multi-client**: only relevant if secondary clients were peers. They are views with permissions of an authoritative primary (see [multi-client.md](multi-client.md)).
- **A prioritized + coalescing input channel** and **per-topic subscription streams**: see _Inputs_ and _Outputs_ — both rejected for machinery that solves problems the measured rates don't have.
- **Recompute-on-replay for worker results**: see _Determinism & Replay_.
