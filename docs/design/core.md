# The Rust Core

The core is a plain Rust library that owns all application state and computation. It is a single-owner state machine, written **sans-IO**: the core performs no I/O, spawns no threads, and reads no clocks — it is driven entirely through a poll-style API by a runtime layer that owns all of those. This document covers the core, its computation model, and the message protocol through which everything else interacts with it.

The shape is deliberately boring and well-trodden: journaled inputs into a single-writer state machine (as in the LMAX Disruptor), a sans-IO surface (as in `quinn-proto`, `rustls`, or `str0m`), and determinism as an architectural constraint so that simulation testing pays off (see [testing.md](testing.md)). It is also a deliberate departure from XCSoar's blackboard chain, which copies state snapshots between device, merge, and calculation threads. Alternatives considered and rejected are listed at the [end](#alternatives-considered).

## Crate Layout

The dependency direction ("shells depend on the core's protocol, the core depends on nothing above it", see [README.md](README.md)) is enforced by cargo, not convention:

- **`updraft_protocol`** — the serde + ts-rs types only: commands, queries, errors, topic payloads, and the mutating/read-only tagging. No logic. TypeScript generation runs against this crate, and it is the only crate transports need for plumbing.
- **`updraft_core`** — the sans-IO state machine. Depends on `updraft_protocol` and the domain libs (`updraft_geo`, `updraft_polar`, …).
- **`updraft_runtime`** — everything the core must not own: channels, the rayon worker pool, tokio for I/O adapters, the recording writer, the persistence thread, and the real clock. Depends on `updraft_core`.

`server` and `tauri` depend on `updraft_runtime`; the frontend depends only on the generated output of `updraft_protocol`. Besides making the boundary structural, the split keeps protocol edits from rebuilding domain logic and vice versa.

## State

One state struct owns everything the application knows: current fix, sensor values (vario, airspeed, altitude sources), the traffic table, loaded airspace/waypoint/terrain datasets, the active task, computed values (wind, MacCready, final glide), settings, and device connection states. It is mutated **only** through the core's single input entry point, with no shared mutable state across threads.

The struct is composed from **per-domain modules** (traffic, task, airspace, devices, settings, …), each owning its sub-state, the inputs it consumes, its reducer, and its topic projections, composed Elm-style at the top level — so the input dispatch never becomes one giant match. Mode-ful subdomains (flight modes, the warning lifecycle, device connection lifecycles) are explicit little state machines — an enum with transition methods — never loose booleans. Cross-domain derived values (final glide needs wind, MacCready, polar, position, terrain) are nodes in the computation graph (see below), not cross-module calls.

## The Sans-IO Surface

```rust
impl Core {
    /// Apply one input; the only mutation entry point.
    fn handle_input(&mut self, input: Input);
    /// Drain pending effects after handling inputs.
    fn poll_effect(&mut self) -> Option<Effect>;
    /// Next timer deadline, for the host to arm.
    fn next_deadline(&self) -> Option<Instant>;
}

enum Effect {
    Publish(TopicUpdate),    // subscription output (see Outputs)
    Compute(ComputeRequest), // pure CPU job; result re-enters as an Input
    Io(IoRequest),           // snapshot write, OGN poll, …; result re-enters as an Input
}
```

The runtime pumps inputs into `handle_input`, executes `Compute` requests on the rayon pool and `Io` requests on tokio or the persistence thread, arms a single timer from `next_deadline`, and feeds every result back in as an input. This makes the two load-bearing rules structural rather than conventional: the core **cannot block** on work it only describes as data, and results **must re-enter as inputs** because no other path exists. It also means whole-flight integration tests are a plain loop over `handle_input`/`poll_effect` — no async runtime, no sleeps, no flakiness surface — and replay executes the same `ComputeRequest::run()` the production pool does (see _Determinism & Replay_).

## Inputs

Everything enters as messages through `handle_input`:

- parsed device sentences (from connection threads, see [devices.md](devices.md)),
- user commands (from the transport layer),
- timer ticks,
- completed computation and I/O results (see _The Computation Graph_ below).

Device connections run on their own threads/tasks in the runtime and are dumb pipes: bytes in, parsed messages out, into the ingress. The core is therefore a **pure function of its input sequence**: record the input stream and any bug is replayable deterministically. This is the foundation of the testing story (see [testing.md](testing.md)).

Ingress is structured rather than a single clever channel. The runtime drains three plain parts in a fixed, deterministic order:

- a **command FIFO**, drained first — this is the prioritization: an acknowledgement never queues behind a burst of fixes;
- **latest-value mailboxes** for sensor streams (one slot per stream, keyed per target for traffic) — overwriting an unconsumed slot _is_ the coalescing, so sensor bursts can never build a backlog by construction;
- the **timer queue** (see _Time Is an Input_).

Coalescing means superseded inputs are dropped before the core ever sees them. That is correct only for last-value sensor data: commands, device connection events, and FLARM alarm transitions are never coalesced. Recording for replay captures messages as they are dequeued, so the recorded sequence is exactly what the core saw.

## Time Is an Input

The core never reads the wall clock directly. Time is injected via a `Clock` trait and advances via messages; the host arms one timer from `next_deadline`. Replay at 100x real time is a unit-test primitive, not a special mode. Tests should be able to run a "four-hour flight" in seconds.

**Clock vs. GPS time.** The injected clock drives scheduling only (timers, debouncing, warning lookahead). It is never conflated with fix timestamps: IGC records and all flight data use GPS time carried in the fixes themselves. Replay therefore reproduces original GPS timestamps regardless of playback speed.

## The Computation Graph

Derived values form a **static dependency graph**. Each node declares:

- its **inputs**: state fields and/or other nodes,
- its **cadence policy**: every-change, rate-limited (always against the newest inputs), or debounced-async,
- for async nodes, an **invalidation predicate** over (inputs at spawn, inputs now). The default is "any input changed"; nodes with cheaper semantics override it — the reach polygon is invalidated only when the position moved beyond a threshold or the MacCready/wind/task inputs changed.

Typical cadences:

- **Every fix**: ground speed, track, GPS/pressure altitude fusion, AGL lookup
- **Every vario update**: speed-to-fly
- **~1 Hz** (rate-limited): airspace-proximity lookahead, nearest-waypoint ranking, final glide
- **Debounced / async**: glide-reach polygon, task optimization, wind-estimation refinement

After each batch of inputs the update loop marks dirty nodes, recomputes synchronous nodes in topological order, and emits `Compute` effects for dirty async nodes. A completion re-enters through `handle_input` and is accepted iff its node's predicate says no invalidating input arrived since it was spawned — staleness is judged per node, never by a global input counter, because a slow worker always finishes several inputs behind the current state. Adding a computed value means adding a node declaration, not re-deriving a staleness argument by hand; devmode gets per-node recompute counts and timings for free.

`Compute` effects are pure CPU jobs the runtime executes on the rayon pool. I/O-bound work from the outside world, such as pulling OGN traffic, is an `Io` effect executed by async tokio tasks in the runtime; results re-enter as inputs like everything else.

**Warnings are synchronous.** Warning generation (computed airspace proximity, relayed FLARM collision alarms, see [traffic.md](traffic.md)) runs inside the update loop; warning nodes are forbidden from being async.

**Worker failure is an input.** The runtime catches panics at the effect-executor boundary and converts them into an ordinary input (`ComputeFailed { node, error }`), so a crash in a worker is recorded and replayable like any other event. The graph decides per node whether to retry, degrade (keep showing the stale reach polygon with an age indicator), or surface a devmode diagnostic. `Io` effects report failure the same way (a failed snapshot write becomes a user-visible warning, see _Snapshots & Resume_).

## Determinism & Replay

Recording and replaying an input sequence reproduces the exact same state evolution, which is the foundation for simulation mode, IGC replay, demo mode, and regression testing alike. Worker results posted back as inputs make this subtle, and the rule is: **record only what is genuinely nondeterministic.**

- **External I/O results** (OGN responses, weather fetches) are recorded verbatim. They come from outside and are inputs like any other.
- **Pure CPU worker results** (reach polygon, task optimization, wind refinement) are recorded only as a completion marker: which node finished, where in the input sequence its result landed, and a **hash of the payload**. Replay recomputes the payload and injects it at the recorded position, so the state evolution keeps its original ordering while the log stays proportional to the flight instead of being dominated by derived geometry. It also means every replayed test exercises the worker code, not just an opt-in verification mode.

**Recordings are version-bound artifacts.** Because replay recomputes pure worker payloads, a recording is only faithful on the code version that wrote it. The log carries a header (app version, git hash, protocol version) and replay refuses — or loudly warns — on mismatch. The per-completion payload hash catches divergence at the first differing worker result ("diverged at input #N, node `reach`") instead of as a mysteriously different final state. A devmode flag records payloads verbatim for cross-version forensics.

This requires the workers themselves to be deterministic:

- **Parallel map, sequential fold.** rayon's `collect` preserves index order, so the parallel phase is fine, but float accumulation must happen sequentially in a fixed order. Never `par_iter().sum()` or `.reduce()` over floats in a worker whose output must replay identically.
- **No iteration-order-dependent output.** Anything that influences a result uses ordered maps or a fixed hasher. `HashMap` iteration order varies between process runs and would silently break replay.
- **No architecture-dependent floats.** The same version on x86 CI and an ARM phone can diverge through FMA contraction and platform libm differences. Worker code that feeds replay avoids `mul_add` and platform-varying transcendentals (the `libm` crate where portable results are needed), and CI replays the golden corpus on both an x86 and an ARM runner and diffs the payload hashes to catch violations empirically.

**Timers are core state.** Debounce and scheduling timers live in the core as a priority queue of (deadline, timer id), armed by the update loop and drained deterministically as the injected clock advances, with a fixed tie-break against same-tick inputs. Adapters only ever deliver clock advancement — in production from a real ticking source armed via `next_deadline`, in replay from the recorded timeline. There is no second scheduler implementation that replay has to keep in lockstep; the same principle is why replay executes `ComputeRequest::run()` directly instead of shadowing the worker pool.

**Resume is a new baseline.** After a mid-flight restart the core seeds its state from a snapshot (see _Snapshots & Resume_ below), so replay tooling supports "seed from snapshot X, then replay the log from position N" as a first-class mode alongside replay-from-empty. When input recording is enabled it is written incrementally like the IGC log, so a crash leaves behind exactly the artifact needed to reproduce it.

Determinism also feeds the test strategy beyond replaying recorded flights: the sans-IO surface makes **reference-model tests** cheap — proptest state-machine testing drives thousands of generated input interleavings per CI run against both a small, obviously-correct model of a subsystem (source priority, warning lifecycle, timer queue) and the real core, comparing state (see [testing.md](testing.md)).

## Outputs

After each update cycle, the core publishes state changes to named topics as `Publish` effects. Subscribers receive only the topics they subscribed to, so UIs can render reactively instead of polling and IPC traffic stays minimal by construction. **Every topic delivers its current state immediately on subscribe**, so a late or reconnecting subscriber never needs history: reconnect is just resubscribe. There is no replay buffer and no generic diff/patch format (JSON Patch and friends buy nothing at these payload sizes).

**Topics are projections.** After each cycle, each topic's projection function runs over the state and is equality-diffed against the last published value (per key for keyed collections); it is published iff changed. A topic can therefore never be stale relative to state — "forgot to publish after mutating X" is structurally impossible — and the projection is the single place a topic's payload shape is defined. The runtime retains each topic's last published value (for keyed collections, the full map) to serve the on-subscribe snapshot without involving the core. The exception is genuinely edge-triggered output: warning raised/escalated/cleared/acknowledged events are transitions, not state, and are emitted explicitly by the reducer; the _active set_ half of that topic is still a projection.

Topics come in four kinds:

- **Last-value** (`position`, `wind`, `mac-cready`, `final-glide`, `vario`): each message is the complete current value and replaces the previous one.
- **Keyed collection** (`traffic`, `devices`): upserts and removals keyed by a stable ID (FLARM/OGN target, device), preceded by a full snapshot on subscribe.
- **Events plus active set** (`airspace-warnings`): edge-triggered events (raised, escalated, cleared, acknowledged), because audio and banners fire on transitions, plus the currently active set on subscribe.
- **Reference** (`reach`, `track`): a version counter and a URL, with the payload fetched through the bulk geodata path (see below).

Further topics (`task`, `logger`, `settings`, …) follow the same taxonomy. The native audio adapter is an in-process subscriber to the warning topics, wired up when the core is embedded, so warnings sound regardless of transport or webview state (see [tauri.md](tauri.md)).

**Slow subscribers never grow unbounded buffers.** The policy follows the topic taxonomy: last-value and reference topics **conflate** (a slow consumer skips intermediate values and always gets the newest), keyed collections conflate **per key**, and event topics must not conflate — they get a bounded buffer, and on overflow the subscription is dropped, forcing a resubscribe, which is safe and cheap precisely because subscribe delivers the current active set. The in-process audio subscriber gets the same contract; warning events are low-rate, so its bound is never hit in practice — the point is that the invariant is stated. This composes with the ingress mailboxes: the same "latest wins" conflation guards both edges of the core.

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

**Requests** are low-frequency and latency-insensitive (load a file, change a setting, connect a device, "what's at this point?"). These use plain JSON, defined as Rust types with `serde` in `updraft_protocol`. The matching TypeScript types are **generated via `ts-rs`**, so the two sides cannot drift. Generated types are committed, and CI fails if a regeneration would change them (golden-file check).

**Subscription streams** are per-topic. Plain JSON is the starting point for every topic. The topic abstraction allows choosing a different encoding per topic later without touching consumers, for example a compact binary frame for a high-rate stream such as the live vario signal.

## The Bulk Geodata Path

**Bulk geodata never travels through the message channel.** Pushing map tiles, airspace geometry, the glide-reach polygon, or the flight track through IPC and into MapLibre is the biggest serialization trap, so the core exposes that data as ordinary HTTP-style resources in _both_ hosts: native routes in the axum server, and a custom URI scheme (`updraft://tiles/…`, `updraft://geojson/…`) in the Tauri shell that streams raw bytes without JSON encoding. MapLibre consumes these as normal sources (vector/PMTiles basemap and terrain, GeoJSON overlays).

For this geodata the frontend therefore only ever handles **references**: source URLs plus version counters. When the reach polygon is recomputed, the core bumps a version on the `reach` topic and the frontend calls `source.setData(url)`. No geometry crosses the message channel.

### The Growing Track Resource

The own track grows for the whole flight, so serving it naively would refetch the full history on every change. Instead, the core assigns each appended track segment a monotonic sequence number and a stable feature ID; the `track` topic (reference kind) publishes `{version: seq, url}`. The track URL accepts `?since=<seq>` and returns only the segments after that point (plus the current head) as a FeatureCollection. The frontend applies increments with MapLibre's `GeoJSONSource.updateData`, whose `GeoJSONSourceDiff` (add/update/remove by feature ID — unique IDs are required, which the sequence numbering provides) exists precisely to avoid re-parsing large sources for small changes. A fresh or reconnecting subscriber fetches without `since` and receives the consolidated track (old geometry may be simplified server-side at that point); after that, tail fetches only.

## Snapshots & Resume

The core persists flight-critical state continuously: periodic snapshots of in-flight state (active task, logging status, device configuration) with atomic writes, alongside incremental IGC logging. On startup the core detects an interrupted flight and resumes logging automatically. Storage details live in [data.md](data.md).

Snapshot and IGC writes never run inline on the update loop: they are `Io` effects performed by a dedicated I/O thread in the runtime, which posts completion or failure back as an input message, so a slow flash write can never stall the warning pipeline.

**Snapshots carry a schema version.** A resumed flight may load state written by an older binary; on version mismatch the core degrades gracefully — fresh state, with IGC logging still resumed from the incremental log — rather than attempting migration or crashing. The IGC log is the flight record; the snapshot is convenience state and can always be discarded.

## Alternatives Considered

Macro-architectures evaluated and rejected, kept here as rationale:

- **Actor frameworks** (actix, ractor; actor-per-device/subsystem): buys supervision and per-actor mailboxes, but destroys the single totally ordered input sequence — with N actors, reproducing a bug means reproducing a schedule, not a log. Soaring state is also tightly coupled, so actor boundaries would be chatty and arbitrary. The design already is one actor with many dumb producers.
- **ECS** (headless bevy_ecs): earns its complexity with many homogeneous entities; this state is heterogeneous singletons plus one small keyed table, and parallel system schedulers fight replay determinism. The useful discipline — declared reads/writes — survives in the computation graph.
- **Demand-driven incremental computation** (salsa): the right tool when consumers _pull_ queries over a large input database (rust-analyzer). This pipeline is the opposite shape — push-driven, fixed cadences, tiny hot state — and semantic invalidation thresholds ("moved > X m") are inexpressible in memoization frameworks. The core ideas (declared dependencies, dirty propagation) are adopted in hand-rolled form.
- **Full CQRS / explicit event sourcing**: the input sequence already _is_ the event log and state is its fold; a second domain-event layer adds schema burden and buys nothing replay doesn't provide. The read-model half — topics as projections — is adopted, because that half pays.
- **Reactive signal-graph frameworks** (futures-signals, leptos-style runtimes): solve UI-driven recomputation with dynamic subscriber sets and bring their own scheduling that would have to be held in lockstep with replay. The concept became the computation graph; the frameworks stayed out.
- **CRDTs for multi-client**: only relevant if secondary clients were peers. They are views with permissions of an authoritative primary (see [multi-client.md](multi-client.md)); conflict-free merging is the wrong problem.
