# The Rust Core

The core is a plain Rust library (no UI, no networking, no threads) that owns all application state and computation. It is a single-owner state machine exposed as a **pure, synchronous update function**: the host feeds it one input at a time, and it returns topic publishes plus **effect requests** as data — spawn this computation, write this snapshot, wake me at time T. A thin **host runtime** — one per embedding: the Tauri shell, the axum server, a unit test — owns the input channel, the worker threads, and all I/O; it executes the effects and feeds their completions back as inputs. This sans-IO shape (as used by e.g. `quinn-proto`) keeps the deterministic surface to a single function: threads, channels, and timing exist only in the host. This document covers the core itself and the message protocol through which everything else interacts with it.

## State

One state struct owns everything the application knows: current fix, sensor values (vario, airspeed, altitude sources), the traffic table, loaded airspace/waypoint/terrain datasets, the active task, computed values (wind, MacCready, final glide), settings, and device connection states. It is mutated **only** through the update function, with no shared mutable state across threads.

## Inputs

Everything enters as messages on a single channel:

- parsed device sentences (from connection threads, see [devices.md](devices.md)),
- user commands (from the transport layer),
- timer ticks,
- completed async computation results (see _Computation Pipeline_ below).

Device connections run on their own threads/tasks and are dumb pipes: bytes in, parsed messages out, onto the channel. The core is therefore a **pure function of its input sequence**: record the input stream and any bug is replayable deterministically. This is the foundation of the testing story (see [testing.md](testing.md)).

The channel is a plain bounded FIFO, owned by the host and drained in batches. There are no priority lanes and no coalescing: per-input update cost is microseconds, so even a thousand-message sensor burst delays a user command by about a millisecond, and applying a superseded traffic update is a cheap state overwrite — the state itself is the coalescer. Priority could not improve the real latency bound anyway, which is the slowest synchronous pipeline stage, not queue position. The bounded capacity is the overload safety valve. (Should command latency ever measurably matter, a second command channel with a biased poll is the escape hatch — a host-side change that leaves the core and replay untouched.) Recording for replay captures inputs at the point the host feeds them to the update function, so the recorded sequence is exactly what the core saw.

## Time Is an Input

The core never reads the wall clock directly. Time is injected via a `Clock` trait and advances via messages. Replay at 100x real time is a unit-test primitive, not a special mode. Tests should be able to run a "four-hour flight" in seconds.

**Clock vs. GPS time.** The injected clock drives scheduling only (timers, debouncing, warning lookahead). It is never conflated with fix timestamps: IGC records and all flight data use GPS time carried in the fixes themselves. Replay therefore reproduces original GPS timestamps regardless of playback speed.

## Computation Pipeline

After each batch of input messages the core runs a staged pipeline whose stages update at different cadences:

- **Every fix**: ground speed, track, GPS/pressure altitude fusion, AGL lookup
- **Every vario update**: speed-to-fly
- **~1 Hz** (rate-limited, always against the newest fix): airspace-proximity lookahead, nearest-waypoint ranking, final glide
- **Debounced / async**: glide-reach polygon, task optimization, wind-estimation refinement

Expensive, CPU-bound stages must never block the state machine, and the core never runs them itself: it emits a **spawn effect** carrying a snapshot of the inputs the computation needs, and the host runs the computation on a worker thread. One plain thread per worker kind is expected to suffice on phone-class hardware; whether a worker parallelizes internally is a host/worker implementation detail the core never sees. I/O-bound work from the outside world, such as pulling OGN traffic, is handled by async tasks in the host/adapter layer. Either way, the result comes back **as another input message**.

Scheduling per worker kind is a **dirty flag plus at most one job in flight**. Relevant input changes set the flag — this is where "position moved beyond a threshold" style gates live, deciding whether a recompute is worth scheduling at all. When the worker is idle and the flag is set, the core snapshots the inputs, emits the spawn effect, and clears the flag. A completed result is **always applied**, never discarded for staleness: discarding a slightly-stale result only leaves an even staler previous one on screen, and if the flag was set again while the job ran, the next job is spawned immediately, so the display converges. The one genuine invalidation case — a result that is semantically wrong under the new state, such as a reach polygon for a task that was since replaced — is handled by an **epoch counter** per worker kind: semantically breaking changes bump the epoch, and a result stamped with an old epoch is dropped. One integer comparison instead of per-kind invalidation predicates.

**Warnings are synchronous.** Warning generation (computed airspace proximity, relayed FLARM collision alarms, see [traffic.md](traffic.md)) runs inside the update loop, never on the async worker path.

## Determinism & Replay

Recording and replaying an input sequence reproduces the exact same state evolution, which is the foundation for simulation mode, IGC replay, demo mode, and regression testing alike. Because the core is a pure function, replay is literally a fold over the recorded inputs — no threads, no channel, no timing are involved.

**All worker results are recorded verbatim**, external I/O (OGN responses, weather fetches) and pure CPU results (reach polygon, task optimization, wind refinement) alike. One rule, no special cases: replay just reads the next input and never re-runs a worker. The alternative — recording pure results as completion markers and recomputing them during replay — was considered and rejected:

- Recompute-on-replay requires bit-exact determinism of every worker, forever, across platforms. That is harsher than it sounds: `f64::sin`/`cos`/`atan2` dispatch to the platform libm, whose results differ between glibc, musl, bionic, and Apple's implementation, so a recording captured on an Android device could silently replay differently on a Linux CI machine.
- It ties recordings to the algorithm version: improve the reach algorithm and every previously recorded flight replays into a different state evolution — the field recording that reproduced a bug stops reproducing it after the next release. Verbatim recordings stay valid forever.
- It makes replay infrastructure heavier: replay would have to run the worker pool and coordinate injecting results at recorded positions, instead of just reading inputs.

The cost is log size — a reach polygon every few seconds over a four-hour flight is tens of MB raw — which compression handles: the recording is written compressed, and derived geometry compresses extremely well. Worker regressions are covered by direct golden tests on worker inputs → outputs, and an opt-in **verify mode** re-runs the workers during replay and diffs their output against the recorded payloads. Determinism hygiene inside workers (sequential float folds, ordered maps or fixed hashers instead of bare `HashMap` iteration) remains a guideline because it keeps golden tests stable across runs, but it is no longer a load-bearing replay invariant.

**Determinism scope.** Bit-exact replay is guaranteed for the same build on the same platform. Cross-platform replay is input-exact: the state evolution replays identically wherever it depends only on recorded inputs, and test assertions on computed floats use tolerances.

**Timers are core state.** Debounce and scheduling timers live in the core as a priority queue of (deadline, timer id), armed by the update loop and drained deterministically as the injected clock advances, with a fixed tie-break against same-tick inputs. The host only ever delivers clock advancement, in production from a real ticking source and in replay from the recorded timeline; the update output carries the earliest pending deadline so a host can sleep precisely instead of ticking blindly. There is no second scheduler implementation that replay has to keep in lockstep.

**Resume is a new baseline.** After a mid-flight restart the core seeds its state from a snapshot (see _Snapshots & Resume_ below), so replay tooling supports "seed from snapshot X, then replay the log from position N" as a first-class mode alongside replay-from-empty. When input recording is enabled it is written incrementally like the IGC log, so a crash leaves behind exactly the artifact needed to reproduce it.

## Outputs

After each update cycle, the core publishes state changes to named topics. Publishes are part of the update function's output, and the host's transport layer fans them out to subscribers. The update function knows from the input it just processed which topics are affected, so publishing is driven by that knowledge directly — there is no after-the-fact state diffing. Subscribers receive only the topics they subscribed to, so UIs can render reactively instead of polling and IPC traffic stays minimal by construction. **Every topic delivers its current state immediately on subscribe**, so a late or reconnecting subscriber never needs history: reconnect is just resubscribe. There is no replay buffer and no generic diff/patch format (JSON Patch and friends buy nothing at these payload sizes).

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

The protocol types (inputs, commands, queries, errors, topic payloads) live in their own crate, `updraft_protocol`, so the transports and the TypeScript codegen depend on the message surface without pulling in core internals — the "shells depend on the core's message protocol" dependency direction becomes literal.

### Encoding

The encoding is chosen per interaction by shape and frequency, while the contract stays transport-agnostic.

**Requests** are low-frequency and latency-insensitive (load a file, change a setting, connect a device, "what's at this point?"). These use plain JSON, defined as Rust types with `serde`. The matching TypeScript types are **generated via `ts-rs`**, so the two sides cannot drift. Generated types are committed, and CI fails if a regeneration would change them (golden-file check).

**Subscription streams** are per-topic. Plain JSON is the starting point for every topic. The topic abstraction allows choosing a different encoding per topic later without touching consumers, for example a compact binary frame for a high-rate stream such as the live vario signal.

## The Bulk Geodata Path

**Bulk geodata never travels through the message channel.** Pushing map tiles, airspace geometry, the glide-reach polygon, or the flight track through IPC and into MapLibre is the biggest serialization trap, so the core exposes that data as ordinary HTTP-style resources in _both_ hosts: native routes in the axum server, and a custom URI scheme (`updraft://tiles/…`, `updraft://geojson/…`) in the Tauri shell that streams raw bytes without JSON encoding. MapLibre consumes these as normal sources (vector/PMTiles basemap and terrain, GeoJSON overlays).

For this geodata the frontend therefore only ever handles **references**: source URLs plus version counters. When the reach polygon is recomputed, the core bumps a version on the `reach` topic and the frontend calls `source.setData(url)`. No geometry crosses the message channel.

**The own track** is the one ever-growing resource, and it is never re-served in full: the core serves it as an immutable log with a monotonic sequence number. The `track` topic carries the latest sequence, the frontend fetches `updraft://track?since=<seq>` for just the new points, and appends them into a local GeoJSON source it feeds to MapLibre. The core serves an immutable tail; the frontend accumulates.

## Snapshots & Resume

The core persists flight-critical state continuously: periodic snapshots of in-flight state with atomic writes, alongside incremental IGC logging. On startup the core detects an interrupted flight and resumes logging automatically. Storage details live in [data.md](data.md).

The snapshot is a small, explicitly versioned struct of flight-critical state (active task, logging status, device configuration) — never a serde dump of the whole state struct, so the state can evolve freely without snapshot-migration pain.

Snapshot and IGC writes never run inline on the update loop: the core emits write effects, a dedicated host I/O thread performs them and posts completion or failure back as an input message, so a slow flash write can never stall the warning pipeline.
