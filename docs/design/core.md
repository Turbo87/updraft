# The Rust Core

The core is a plain Rust library (no UI, no networking assumptions) that owns all application state and computation. It is a single-owner state machine. This document covers the core itself and the message protocol through which everything else interacts with it.

**The core crate has no direct tokio or rayon dependency.** Async I/O belongs to the adapter layer, and CPU-bound workers run behind a small trait with a threaded implementation on native hosts and a sequential fallback (see _Computation Pipeline_ below). This keeps the deterministic path free of threading by construction, and it keeps a single-threaded `wasm32` build of the core viable — a browser-only demo/training mode and frontend tests against the real core come nearly for free, and the constraint guarantees the core never quietly grows platform dependencies. (Threaded WASM is explicitly not pursued: it requires nightly toolchains and rebuilt std, and tokio's sync primitives panic on the browser main thread.)

## State

One state struct owns everything the application knows: current fix, sensor values (vario, airspeed, altitude sources), the traffic table, the active task, computed values (wind, MacCready, final glide), settings, and device connection states. It is mutated **only** by the core's own update loop, with no shared mutable state across threads.

"One struct" is an ownership rule, not a monolith: the struct is a tree of per-domain sub-states (position & sensors, traffic, task, glide settings, devices, warnings, …), each owned by its own module together with its update logic.

**Reference data is not state.** Loaded datasets — airspace, waypoints, terrain — are immutable once loaded and held as cheap shared handles (`Arc`). Loading a file builds a new dataset and swaps the handle; the swap is the state change, the dataset itself never mutates. Readers on any thread can hold a dataset without locking.

**Readers get snapshots.** After each update cycle the loop publishes an immutable snapshot of the state: sub-states that changed are cloned into it, unchanged ones are shared from the previous snapshot via `Arc` (collections with per-cycle churn, like the traffic table, can use persistent structures — `imbl` — where structural sharing pays off). Everything that reads state — queries, topic publishing, async workers — reads a snapshot and never touches the live struct. The loop stays the sole writer; readers are consistent, lock-free, and off the hot path. XCSoar's copy-based blackboards between its merge and calculation threads are the same pattern; immutable sharing replaces the locks and copies.

## Inputs

Everything enters through a two-lane input front:

- **Commands** — user commands from the transport, completed worker results, I/O completions, clock advancement — arrive on a bounded FIFO channel. Commands are never dropped; a full queue pushes back on the sender.
- **Sensor data** — parsed device messages (from connection threads, see [devices.md](devices.md)), internal sensors, simulator/replay — lands in latest-wins slots, keyed per category (and per target within keyed categories such as traffic). A newer value replaces an unread older one, so a sensor burst can never queue up behind a slow cycle and backpressure exists by construction.

The update loop drains both lanes at the start of each cycle, commands first, so an acknowledgement never queues behind a burst of fixes. Coalescing therefore happens at the drain point, inside the core, where message semantics live: device connections stay dumb pipes — bytes in, parsed messages out — and no producer needs to know which messages supersede which. Recording for replay captures messages exactly as drained, so the recorded sequence is exactly what the core saw and the core is a **pure function of its input sequence**.

## Time Is an Input

The core never reads the wall clock directly. Time is injected via a `Clock` trait and advances via messages. Replay at 100x real time is a unit-test primitive, not a special mode. Tests should be able to run a "four-hour flight" in seconds.

**Clock vs. GPS time.** The injected clock drives scheduling only (timers, debouncing, warning lookahead). It is never conflated with fix timestamps: IGC records and all flight data use GPS time carried in the fixes themselves. Replay therefore reproduces original GPS timestamps regardless of playback speed.

## Computation Pipeline

After each batch of input messages the core runs a staged pipeline whose stages update at different cadences:

- **Every fix**: ground speed, track, GPS/pressure altitude fusion, AGL lookup
- **Every vario update**: speed-to-fly
- **~1 Hz** (rate-limited, always against the newest fix): airspace-proximity lookahead, nearest-waypoint ranking, final glide
- **Debounced / async**: glide-reach polygon, task optimization, wind-estimation refinement

Which stages run is decided by dirty flags derived from comparing consecutive snapshots — a hand-rolled dependency layer, deliberately not an incremental-computation framework: the graph is dozens of values, not a compiler's.

Expensive, CPU-bound stages run on the worker pool behind the core's worker trait (rayon-backed on native hosts, sequential in tests and on wasm32) and must never block the state machine. I/O-bound work from the outside world, such as pulling OGN traffic, is handled by async tokio tasks in the adapter layer. A worker posts its result back **as another input message**, tagged with the snapshot it computed from. Staleness is then a data question rather than per-worker bookkeeping: when the result arrives, the inputs it depended on (for the reach polygon: position beyond a movement threshold, MacCready, wind, task) are compared between that snapshot and the current state, and the result is discarded only if one of them actually changed in the meantime.

**Warnings are synchronous.** Warning generation (computed airspace proximity, relayed FLARM collision alarms, see [traffic.md](traffic.md)) runs inside the update loop, never on the async worker path.

## Determinism & Replay

Recording and replaying an input sequence reproduces the exact same state evolution, which is the foundation for simulation mode, IGC replay, demo mode, and regression testing alike. The rule is: **everything nondeterministic is recorded verbatim — including worker results.**

- **External I/O results** (OGN responses, weather fetches) are recorded verbatim. They come from outside and are inputs like any other.
- **Worker results** (reach polygon, task optimization, wind refinement) are recorded verbatim too. Replay is then pure log application: it demands no bit-exactness from worker code, and a log recorded in the cockpit on an ARM phone replays identically on an x86 dev machine. Worker payloads dominate log size, so they go to a compressed sidecar stream next to the primary input log; if storage is tight the sidecar can be dropped, at the cost of recomputing workers on replay (same-platform only, see the determinism budget below).

**Recompute-and-compare is a verification mode, not the replay mechanism.** In CI, recorded flights are replayed with the workers recomputing live, and their outputs are compared against the recorded payloads; a mismatch flags either a nondeterminism regression or a genuine behavior change. This keeps worker code exercised and honest without making bit-exact floating point load-bearing for field forensics. The inverse design — replay that silently depends on recomputation matching — has a well-documented failure mode: a decade of Factorio desync reports shows determinism bugs recurring release after release, with replay divergence as the only symptom.

For recomputation to hold, worker code follows a determinism budget grounded in what Rust actually guarantees (RFC 3514: basic float arithmetic is bit-exact IEEE 754, with no FMA contraction):

- **No parallel float reductions.** rayon's `reduce()`/`sum()` ordering is unspecified by design. Parallel map with order-preserving `collect` is fine; float folds happen sequentially in a fixed order.
- **No iteration-order-dependent output.** Anything that influences a result uses ordered maps or a fixed hasher. `HashMap` iteration order varies between process runs.
- **No NaN-bit inspection.** NaN payloads are non-deterministic by spec; NaNs are canonicalized or rejected at state boundaries.
- **Transcendentals are platform-dependent.** `sin`/`cos`/`atan2` route through platform libm and differ between architectures. Same-platform recomputation is unaffected. If cross-platform recompute verification ever proves valuable, transcendentals in worker code get pinned to a software implementation (the `libm` crate) at that point — not preemptively.

**Timers are core state.** Debounce and scheduling timers live in the core as a priority queue of (deadline, timer id), armed by the update loop and drained deterministically as the injected clock advances, with a fixed tie-break against same-tick inputs. Adapters only ever deliver clock advancement, in production from a real ticking source and in replay from the recorded timeline. There is no second scheduler implementation that replay has to keep in lockstep.

**Resume is a new baseline.** After a mid-flight restart the core seeds its state from a snapshot (see _Snapshots & Resume_ below), so replay tooling supports "seed from snapshot X, then replay the log from position N" as a first-class mode alongside replay-from-empty. When input recording is enabled it is written incrementally like the IGC log, so a crash leaves behind exactly the artifact needed to reproduce it.

## Outputs

After each update cycle, the core publishes state changes to named topics by diffing the new snapshot against the previous one. Subscribers receive only the topics they subscribed to, so UIs can render reactively instead of polling and transport traffic stays minimal by construction. **Every topic delivers its current state immediately on subscribe**, so a late or reconnecting subscriber never needs history: reconnect is just resubscribe. There is no replay buffer and no generic diff/patch format (JSON Patch and friends buy nothing at these payload sizes).

Topics come in four kinds:

- **Last-value** (`position`, `wind`, `mac-cready`, `final-glide`, `vario`): each message is the complete current value and replaces the previous one.
- **Keyed collection** (`traffic`, `devices`): upserts and removals keyed by a stable ID (FLARM/OGN target, device), preceded by a full snapshot on subscribe.
- **Events plus active set** (`airspace-warnings`): edge-triggered events (raised, escalated, cleared, acknowledged), because audio and banners fire on transitions, plus the currently active set on subscribe.
- **Reference** (`reach`, `track`): a version counter and a URL, with the payload fetched through the bulk geodata path (see below).

**Moving objects are published as kinematic state vectors.** The `position` and `traffic` topics carry position plus velocity — track, ground speed, turn rate, climb rate — and the GPS timestamp, not bare coordinates. Clients extrapolate to render time, so displays stay smooth at any frame rate from 1 Hz FLARM or 10 Hz GPS updates, every client extrapolates from identical states, and no per-frame traffic crosses the transport (see [traffic.md](traffic.md) and [frontend.md](frontend.md)).

A slow subscriber never buffers unboundedly: last-value and reference topics drop superseded values by construction, and a subscriber that falls too far behind on a keyed-collection or event topic is disconnected — which, by the subscribe contract, costs it exactly one resubscribe.

Further topics (`task`, `logger`, `settings`, …) follow the same taxonomy. The native audio adapter is an in-process subscriber to the warning topics, wired up when the core is embedded, so warnings sound regardless of transport or webview state (see [tauri.md](tauri.md)).

## The Message Protocol

The core is interacted with through a small, well-defined surface:

- **Requests** are a single request/response mechanism. Every request returns a result or a typed error. _Commands_ are the mutating requests (`SetMacCready`, `AdvanceTaskTurnpoint`, `AcknowledgeAirspaceWarning`, …), _queries_ the read-only ones (either full snapshots or scoped selections, e.g. `query_at(lon, lat, radius)` for the map's "What's here?" interaction). A command is not fire-and-forget: loading a file can fail, a setting can be out of range, and the caller needs to know.
- **Subscriptions** deliver per-topic state-change notifications (see _Outputs_ above).

**Queries never enter the update loop.** They execute against the most recent published snapshot on the transport side, so a heavy `query_at` over airspace geometry cannot add latency to the warning path.

Commands and queries share one mechanism, one error type, and one correlation scheme, but every request type is tagged as mutating or read-only, because three things depend on the distinction:

1. **Replay:** commands are inputs and enter the recorded input log. Queries never mutate and are never recorded.
2. **Permissions:** client roles are checked per mutating request (see [multi-client.md](multi-client.md)).
3. **Transport mapping:** queries map to GET and are freely retryable. Commands map to POST and carry a client-generated request ID, so a retried command is applied at most once.

Errors are one shared type with stable, machine-readable codes so the frontend can localize and match on them, generated to TypeScript like every other protocol type.

The protocol contract is **host-agnostic**: the same messages flow over HTTP or a direct function call in a unit test, and feature code never knows which. In production there is exactly **one transport** — the axum server, standalone or embedded in the Tauri shell (see [server.md](server.md) and [tauri.md](tauri.md)) — so the contract tests and the Playwright suite exercise the same path that ships on every platform.

### Encoding

The encoding is chosen per interaction by shape and frequency, while the contract stays host-agnostic.

**Requests** are low-frequency and latency-insensitive (load a file, change a setting, connect a device, "what's at this point?"). These use plain JSON, defined as Rust types with `serde`. The matching TypeScript types are **generated via `ts-rs`**, so the two sides cannot drift. Generated types are committed, and CI fails if a regeneration would change them (golden-file check).

**Subscription streams** are per-topic, delivered over SSE (see [server.md](server.md)). Plain JSON is the starting point for every topic. The topic abstraction allows choosing a different encoding or channel per topic later without touching consumers — for example a compact binary frame (postcard) over WebSocket for a high-rate stream such as the live vario signal, should one ever need it.

## The Bulk Geodata Path

**Bulk geodata never travels through the message channel.** Pushing map tiles, airspace geometry, the glide-reach polygon, or the flight track through the protocol and into MapLibre is the biggest serialization trap, so the core exposes that data as ordinary HTTP routes in the axum server — the same routes on every platform, since the Tauri shell embeds the server (see [tauri.md](tauri.md)). MapLibre consumes them as normal sources (vector/PMTiles basemap and terrain, GeoJSON overlays).

A webview custom URI scheme was considered for the native apps and rejected: wry's custom protocols cannot stream response bodies (wry #1404), and Android's webview fails byte-range requests against custom protocols (a Chromium webview bug), which breaks PMTiles exactly on the primary platform. Real HTTP over loopback has none of these problems.

For this geodata the frontend therefore only ever handles **references**: source URLs plus version counters. When the reach polygon is recomputed, the core bumps a version on the `reach` topic and the frontend calls `source.setData(url)`. No geometry crosses the message channel.

## Snapshots & Resume

The core persists flight-critical state continuously: periodic snapshots of in-flight state (active task, logging status, device configuration) with atomic writes, alongside incremental IGC logging. On startup the core detects an interrupted flight and resumes logging automatically. Storage details live in [data.md](data.md).

Snapshot and IGC writes never run inline on the update loop: a dedicated I/O thread performs them and posts completion or failure back as an input message, so a slow flash write can never stall the warning pipeline.

**Versioned persistence.** Snapshots and input logs carry a schema version. Resume accepts only its own version: after an app update, an incompatible snapshot is discarded (the IGC log still resumes — it is append-only and standard). Recorded logs kept as regression fixtures are re-recorded or migrated deliberately when the protocol changes; silent cross-version replay is not attempted.

## Failure & Supervision

A warning-generating instrument needs a defined crash story. The update loop runs under a supervisor: a panic is caught, state is re-seeded from the latest snapshot, and input processing resumes, with the panicking input batch quarantined into the crash report rather than re-fed. Determinism cuts both ways here — blindly replaying the same inputs into the same state reproduces the same panic — so if the core panics again immediately after a resume, it drops to a degraded safe mode (position, IGC logging, and relayed FLARM alarms from live inputs stay; computed stages are re-enabled one at a time to isolate the offender) instead of boot-looping. When input recording is enabled, the recording plus snapshot left behind is exactly the artifact needed to reproduce the crash offline.

## Open Questions

- **Track resource updates:** how the ever-growing own-track resource is served without refetching the full history on every change (bounded tail refetch, appendable format, …).
- **Snapshot sharing granularity:** which sub-states warrant persistent collections (`imbl`) versus plain clone-on-write — to be decided by profiling once the traffic table and waypoint indices exist.
