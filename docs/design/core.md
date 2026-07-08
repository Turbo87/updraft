# The Rust Core

The core is a plain Rust library (no UI, no networking assumptions) that owns all application state and computation. It is a single-owner state machine. This document covers the core itself and the message protocol through which everything else interacts with it.

## State

One state struct owns everything the application knows: current fix, sensor values (vario, airspeed, altitude sources), the traffic table, loaded airspace/waypoint/terrain datasets, the active task, computed values (wind, MacCready, final glide), settings, and device connection states. It is mutated **only** by the core's own update loop, with no shared mutable state across threads.

## Inputs

Everything enters as messages on a single channel:

- parsed device sentences (from connection threads, see [devices.md](devices.md)),
- user commands (from the transport layer),
- timer ticks,
- completed async computation results (see _Computation Pipeline_ below).

Device connections run on their own threads/tasks and are dumb pipes: bytes in, parsed messages out, onto the channel. The core is therefore a **pure function of its input sequence**: record the input stream and any bug is replayable deterministically. This is the foundation of the testing story (see [testing.md](testing.md)).

The channel is not a plain FIFO. User commands are prioritized ahead of sensor traffic, so an acknowledgement never queues behind a burst of fixes, and producers coalesce messages that supersede each other (a newer update for the same traffic target replaces a queued older one). Recording for replay captures messages as the core dequeues them, so the recorded sequence is exactly what the core saw.

## Time Is an Input

The core never reads the wall clock directly. Time is injected via a `Clock` trait and advances via messages. Replay at 100x real time is a unit-test primitive, not a special mode. Tests should be able to run a "four-hour flight" in seconds.

**Clock vs. GPS time.** The injected clock drives scheduling only (timers, debouncing, warning lookahead). It is never conflated with fix timestamps: IGC records and all flight data use GPS time carried in the fixes themselves. Replay therefore reproduces original GPS timestamps regardless of playback speed.

## Computation Pipeline

After each batch of input messages the core runs a staged pipeline whose stages update at different cadences:

- **Every fix**: ground speed, track, GPS/pressure altitude fusion, AGL lookup
- **Every vario update**: speed-to-fly
- **~1 Hz** (rate-limited, always against the newest fix): airspace-proximity lookahead, nearest-waypoint ranking, final glide
- **Debounced / async**: glide-reach polygon, task optimization, wind-estimation refinement

Expensive, CPU-bound stages run on a rayon worker pool and must never block the state machine. I/O-bound work from the outside world, such as pulling OGN traffic, is handled by async tokio tasks in the adapter layer. A worker posts its result back **as another input message**. Staleness is judged per worker kind, not by a global input counter: a slow worker always finishes several inputs behind the current state, so its result is discarded only when an input that actually invalidates it arrived in the meantime (for the reach polygon: position moved beyond a threshold, or the MacCready/wind/task inputs changed).

**Warnings are synchronous.** Warning generation (computed airspace proximity, relayed FLARM collision alarms, see [traffic.md](traffic.md)) runs inside the update loop, never on the async worker path.

## Determinism & Replay

Recording and replaying an input sequence reproduces the exact same state evolution, which is the foundation for simulation mode, IGC replay, demo mode, and regression testing alike. Worker results posted back as inputs make this subtle, and the rule is: **record only what is genuinely nondeterministic.**

- **External I/O results** (OGN responses, weather fetches) are recorded verbatim. They come from outside and are inputs like any other.
- **Pure CPU worker results** (reach polygon, task optimization, wind refinement) are recorded only as a completion marker: which worker finished, and where in the input sequence its result landed. Replay recomputes the payload and injects it at the recorded position, so the state evolution keeps its original ordering while the log stays proportional to the flight instead of being dominated by derived geometry. It also means every replayed test exercises the worker code, not just an opt-in verification mode.

This requires the workers themselves to be deterministic:

- **Parallel map, sequential fold.** rayon's `collect` preserves index order, so the parallel phase is fine, but float accumulation must happen sequentially in a fixed order. Never `par_iter().sum()` or `.reduce()` over floats in a worker whose output must replay identically.
- **No iteration-order-dependent output.** Anything that influences a result uses ordered maps or a fixed hasher. `HashMap` iteration order varies between process runs and would silently break replay.

**Timers are core state.** Debounce and scheduling timers live in the core as a priority queue of (deadline, timer id), armed by the update loop and drained deterministically as the injected clock advances, with a fixed tie-break against same-tick inputs. Adapters only ever deliver clock advancement, in production from a real ticking source and in replay from the recorded timeline. There is no second scheduler implementation that replay has to keep in lockstep.

**Resume is a new baseline.** After a mid-flight restart the core seeds its state from a snapshot (see _Snapshots & Resume_ below), so replay tooling supports "seed from snapshot X, then replay the log from position N" as a first-class mode alongside replay-from-empty. When input recording is enabled it is written incrementally like the IGC log, so a crash leaves behind exactly the artifact needed to reproduce it.

## Outputs

After each update cycle, the core publishes state changes to named topics. Subscribers receive only the topics they subscribed to, so UIs can render reactively instead of polling and IPC traffic stays minimal by construction. **Every topic delivers its current state immediately on subscribe**, so a late or reconnecting subscriber never needs history: reconnect is just resubscribe. There is no replay buffer and no generic diff/patch format (JSON Patch and friends buy nothing at these payload sizes).

Topics come in four kinds:

- **Last-value** (`position`, `wind`, `mac-cready`, `final-glide`, `vario`): each message is the complete current value and replaces the previous one.
- **Keyed collection** (`traffic`, `devices`): upserts and removals keyed by a stable ID (FLARM/OGN target, device), preceded by a full snapshot on subscribe.
- **Events plus active set** (`airspace-warnings`): edge-triggered events (raised, escalated, cleared, acknowledged), because audio and banners fire on transitions, plus the currently active set on subscribe.
- **Reference** (`reach`, `track`): a version counter and a URL, with the payload fetched through the bulk geodata path (see below).

Further topics (`task`, `logger`, `settings`, …) follow the same taxonomy. The native audio adapter is an in-process subscriber to the warning topics, wired up when the core is embedded, so warnings sound regardless of transport or webview state (see [tauri.md](tauri.md)).

## The Message Protocol

The core is interacted with through a small, well-defined surface:

- **Requests** are a single request/response mechanism. Every request returns a result or a typed error. _Commands_ are the mutating requests (`SetMacCready`, `AdvanceTaskTurnpoint`, `AcknowledgeAirspaceWarning`, …), _queries_ the read-only ones (either full snapshots or scoped selections, e.g. `query_at(lon, lat, radius)` for the map's "What's here?" interaction). A command is not fire-and-forget: loading a file can fail, a setting can be out of range, and the caller needs to know.
- **Subscriptions** deliver per-topic state-change notifications (see _Outputs_ above).

Commands and queries share one mechanism, one error type, and one correlation scheme, but every request type is tagged as mutating or read-only, because three things depend on the distinction:

1. **Replay:** commands are inputs and enter the recorded input log. Queries never mutate and are never recorded.
2. **Permissions:** client roles are checked per mutating request (see [multi-client.md](multi-client.md)).
3. **Transport mapping:** queries map to GET and are freely retryable. Commands map to POST and carry a client-generated request ID, so a retried command is applied at most once.

Errors are one shared type with stable, machine-readable codes so the frontend can localize and match on them, generated to TypeScript like every other protocol type.

The protocol is **transport-agnostic**: the same messages flow over Tauri IPC, WebSocket/SSE, or a direct function call in a unit test. Feature code never needs to know which transport is in use. On stream transports, responses carry the request ID they answer, so completion order does not matter (Tauri's `invoke` correlates natively).

### Encoding

The encoding is chosen per interaction by shape and frequency, while the contract stays transport-agnostic.

**Requests** are low-frequency and latency-insensitive (load a file, change a setting, connect a device, "what's at this point?"). These use plain JSON, defined as Rust types with `serde`. The matching TypeScript types are **generated via `ts-rs`**, so the two sides cannot drift. Generated types are committed, and CI fails if a regeneration would change them (golden-file check).

**Subscription streams** are per-topic. Plain JSON is the starting point for every topic. The topic abstraction allows choosing a different encoding per topic later without touching consumers, for example a compact binary frame for a high-rate stream such as the live vario signal.

## The Bulk Geodata Path

**Bulk geodata never travels through the message channel.** Pushing map tiles, airspace geometry, the glide-reach polygon, or the flight track through IPC and into MapLibre is the biggest serialization trap, so the core exposes that data as ordinary HTTP-style resources in _both_ hosts: native routes in the axum server, and a custom URI scheme (`updraft://tiles/…`, `updraft://geojson/…`) in the Tauri shell that streams raw bytes without JSON encoding. MapLibre consumes these as normal sources (vector/PMTiles basemap and terrain, GeoJSON overlays).

For this geodata the frontend therefore only ever handles **references**: source URLs plus version counters. When the reach polygon is recomputed, the core bumps a version on the `reach` topic and the frontend calls `source.setData(url)`. No geometry crosses the message channel.

## Snapshots & Resume

The core persists flight-critical state continuously: periodic snapshots of in-flight state (active task, logging status, device configuration) with atomic writes, alongside incremental IGC logging. On startup the core detects an interrupted flight and resumes logging automatically. Storage details live in [data.md](data.md).

Snapshot and IGC writes never run inline on the update loop: a dedicated I/O thread performs them and posts completion or failure back as an input message, so a slow flash write can never stall the warning pipeline.

## Open Questions

- **Track resource updates:** how the ever-growing own-track resource is served without refetching the full history on every change (bounded tail refetch, appendable format, …).
