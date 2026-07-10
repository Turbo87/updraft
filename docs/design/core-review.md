# Core Architecture Review: Options & Recommendations

A review of the architecture proposed in [core.md](core.md), written before the
`core-state` / `core-time` / `core-subscriptions` / `core-workers` milestones
land — i.e. while changing course is still free. It surveys the realistic
alternatives, says why most of them lose, and recommends a set of refinements
to the current proposal. Where a recommendation is accepted it should be folded
into [core.md](core.md) (and this file trimmed or deleted); this document is an
analysis, not a second source of truth.

## Verdict

The macro-architecture in core.md is sound and does not need replacing. A
single-writer state machine fed by a journaled input sequence, with derived
results re-entering as inputs, is a proven shape with strong prior art:

- **LMAX Disruptor**: journal inputs, run all business logic single-threaded,
  reproduce any state by replay. Same core idea, battle-tested in a domain
  (exchange matching) with far harsher latency requirements than a vario.
- **sans-IO protocol libraries** (`quinn-proto`, `rustls`'s `Connection`,
  [str0m](https://github.com/algesten/str0m)): pure state machines with
  injected time, driven by whatever runtime the host chooses. This is exactly
  the testing story core.md wants, and it is known to work.
- **Deterministic simulation testing** (TigerBeetle's VOPR, FoundationDB,
  [Firezone's connlib](https://www.firezone.dev/blog/sans-io)): determinism as
  an architectural constraint pays off precisely the way testing.md hopes.
- **Elm/Redux reducers**: message-driven single-owner state with derived views
  is the dominant pattern for exactly the "state + commands + computed values
  + a view" feature shape the roadmap describes.

It is also a direct, deliberate improvement over XCSoar's blackboard chain
(`DeviceBlackboard` → `MergeThread` → `CalculationThread` copying snapshots
between threads), which is worth stating in the doc as motivating context.

So the question is not "which architecture instead?" but "where does this
architecture rot as 200 roadmap items land on it, and what do we harden now?"
The recommendations below are, in order of leverage:

1. Make the core **literally sans-IO** — a poll-style API with effects as
   outputs, no tokio/rayon/channel types inside the core crate.
2. **Split crates**: `updraft_protocol` / `updraft_core` / `updraft_runtime`,
   making the documented dependency direction cargo-enforced.
3. Replace per-worker ad-hoc staleness with a **derived-value dependency
   graph** with per-node invalidation predicates.
4. Replace the exotic prioritized/coalescing channel with **structured
   ingress**: a command FIFO plus latest-value mailboxes plus the timer queue.
5. Derive topics from state as **projections with change detection**, so
   forgetting to publish is structurally impossible.
6. **Version recordings and snapshots** explicitly; recordings that recompute
   worker payloads are only faithful on the code version that wrote them.
7. Specify **slow-subscriber policy per topic kind** (conflation vs bounded
   buffer) — currently unspecified and it will bite on remote clients.
8. Define a **worker panic/supervision policy**.
9. Grow the testing story into **deterministic simulation testing** (proptest
   state-machine tests against a reference model, fuzzed schedules), and gate
   cross-platform float divergence in CI.

None of these change the architecture's shape; they turn prose guarantees into
compiler- and CI-enforced ones.

## Options Considered for the Macro-Architecture

For completeness, the realistic alternatives and why they lose:

### A. Status quo: single-writer reducer + message channel (baseline)

What core.md describes today. Sound, but several of its guarantees (core never
does I/O, workers never block the loop, every state change is published) exist
only as prose and reviewer vigilance. Options B below keeps the shape and makes
them structural.

### B. Sans-IO hardening of A — **recommended**

Same architecture, but the core crate exposes a pure poll-style API and owns
zero I/O, zero threads, zero runtime types. Detailed below. This is a
refinement, not a rewrite, and everything else in this review assumes it.

### C. Actor framework (`actix`, `ractor`, one-actor-per-device/subsystem)

Buys supervision trees and per-actor mailboxes. Loses the single totally
ordered input sequence, which is the foundation of the replay story: with N
actors there are N interleaved histories, and reproducing a bug means
reproducing a schedule, not a log. Soaring state is also tightly coupled
(final glide needs wind, MacCready, polar, position, terrain), so actor
boundaries would be chatty and arbitrary. The design already has exactly one
actor with many dumb producers, which is the degenerate case where actor
frameworks add ceremony and subtract determinism. **Rejected.**

### D. ECS (`bevy_ecs` headless, or similar)

ECS earns its complexity with large numbers of homogeneous entities iterated
by many independent systems. Updraft's state is heterogeneous singletons
(position, wind, task, settings) plus one keyed table (traffic) that will
rarely exceed a few hundred rows. System schedulers are also parallel and
order-flexible by design, which fights replay determinism. **Rejected** —
though the *discipline* of systems declaring reads/writes reappears in the
dependency graph below.

### E. Demand-driven incremental computation (`salsa`)

Salsa memoizes query results with automatic dependency tracking — the right
tool when consumers *pull* values at unpredictable times over a large input
database (rust-analyzer). Updraft's pipeline is the opposite shape: fixed
cadences, push-driven, tiny hot state, and the expensive results (reach
polygon) are invalidated by *semantic* thresholds ("moved > X m"), which
memoization frameworks cannot express — they invalidate on any input change,
which for a value that changes every fix means always. **Rejected**, but the
core idea (declared dependencies, automatic dirty propagation) is adopted in
hand-rolled form below.

### F. Full CQRS / event sourcing (explicit command → event → projection)

The design is already event-sourced where it matters: the input sequence *is*
the event log, and state is its fold. Introducing a second, explicit
domain-event layer between commands and state would add schema/versioning
burden for every event type and buy nothing replay doesn't already provide.
**Rejected as a whole**; adopted piecemeal — topics-as-projections (below) is
the read-model half of CQRS, and it's the half that pays.

### G. Reactive signal graphs (`futures-signals`, leptos-style runtimes)

These frameworks solve UI-driven recomputation with dynamic subscriber sets,
and bring their own scheduling that would have to be held in lockstep with
replay. **Rejected as a dependency**; the concept — a static dependency graph
with dirty marking — is exactly recommendation 3.

### H. CRDTs / distributed state for multi-client

Only relevant if secondary clients were peers. They are not: multi-client.md
correctly makes the primary's core the single source of truth and secondaries
mere views with permissions. Conflict-free merging is the wrong problem.
**Rejected.**

## Recommended Changes

### 1. Make the core literally sans-IO

core.md already says the core is "a plain Rust library (no UI, no networking
assumptions)" — but it still implies the core *spawns* rayon jobs, *owns* an
input channel, and is *pumped* by something unspecified. Tighten this into the
str0m/quinn-proto shape:

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
    Publish(TopicUpdate),          // subscription output
    Compute(ComputeRequest),       // run on the worker pool, result re-enters as Input
    Io(IoRequest),                 // snapshot write, OGN poll, … result re-enters as Input
}
```

The core crate contains **no tokio, no rayon, no channels, no threads, no
clocks**. The runtime layer (see crate split) owns all of them: it executes
`Compute` requests on rayon, `Io` requests on tokio or the dedicated
persistence thread, arms one timer for `next_deadline`, and feeds every result
back through `handle_input`.

Why this is worth the ceremony:

- **The "never block the loop" and "workers post results as inputs" rules stop
  being conventions.** A `ComputeRequest` is data; the core *cannot* block on
  it, and its result *must* re-enter as an input, because there is no other
  path.
- **Replay gets simpler, not just possible.** `ComputeRequest` carries a pure
  `run()` function; production executes it on rayon, replay executes the same
  `run()` synchronously and injects the result at the recorded position. One
  implementation, no scheduler to keep in lockstep — this is core.md's timer
  argument ("no second scheduler implementation") extended to all effects.
- **Tests need no runtime at all.** A whole-flight integration test is a loop
  over `handle_input`/`poll_effect` — no `#[tokio::test]`, no sleeps, no
  flakiness surface, and it runs single-threaded under `miri` if ever needed.
- **The Firezone-style reference-model test** (recommendation 9) becomes
  possible: proptest drives arbitrary input interleavings straight into the
  state machine.

This should be decided **before `core-state` lands**, because it dictates the
signature of `apply()` and the shape of the worker milestone.

### 2. Split `updraft_protocol` / `updraft_core` / `updraft_runtime`

README.md fixes the dependency direction in prose ("`frontend`, `server`, and
`tauri` depend on `core`'s message protocol, while `core` depends on nothing
above it"). Make cargo enforce it:

- **`updraft_protocol`** — the serde + ts-rs types only: commands, queries,
  errors, topic payloads, the mutating/read-only tagging. No logic. This is
  the crate ts-rs generation runs against and the only crate hosts need for
  transport plumbing.
- **`updraft_core`** — the sans-IO state machine. Depends on `updraft_protocol`
  and the domain libs (`updraft_geo`, `updraft_polar`, …).
- **`updraft_runtime`** — channels, rayon/tokio executors, the recording
  writer, the persistence thread, the real clock. Depends on `updraft_core`.
- `server` and `tauri` depend on `updraft_runtime`; the frontend depends only
  on the generated output of `updraft_protocol`.

Besides boundary enforcement this pays in compile times (protocol edits don't
rebuild domain logic and vice versa; hosts rebuild less), which matters for
the LLM-agent iteration loop testing.md cares about.

### 3. Formalize the computation pipeline as a dependency graph

core.md's staged pipeline ("every fix / every vario / ~1 Hz / debounced-async")
with per-worker-kind staleness is correct but described as four special cases
plus per-worker invalidation reasoning. As computed values multiply (the
roadmap adds arrival heights, alternates, task ETAs, thermal stats, VNAV, …),
ad-hoc staleness logic is where subtle bugs will accumulate.

Replace the prose with one mechanism: a static graph of **derived-value
nodes**, each declaring

- its **inputs** (state fields and/or other nodes),
- its **cadence policy** (every-change, rate-limited at N Hz, debounced-async),
- for async nodes, an **invalidation predicate** over (inputs at spawn, inputs
  now) — default is "any input changed", and the reach polygon overrides it
  with its semantic thresholds ("position moved beyond X, or MacCready / wind
  / task changed"), exactly the rule core.md already states.

The update loop marks nodes dirty from the inputs applied this cycle,
recomputes synchronous nodes in topological order, and emits `Compute` effects
for dirty async nodes; a completion is accepted iff its node's predicate says
no invalidating input arrived since spawn. This is the same behavior core.md
specifies, but as a table instead of scattered code: adding "arrival heights"
means adding a node declaration, not re-deriving the staleness argument by
hand. It also gives free instrumentation (per-node recompute counts and
timings in devmode) and makes "warnings are synchronous" checkable: warning
nodes are simply forbidden from being async.

### 4. Structure the ingress instead of building an exotic channel

core.md specifies a channel that is priority-ordered *and* coalesces
superseded messages in-queue. A single MPSC with scan-and-replace semantics is
awkward to build and to reason about. The same semantics fall out of three
plain parts, drained by the pump in a fixed, deterministic order:

- a **command FIFO** (drained first — this *is* the prioritization),
- **latest-value mailboxes** for sensor streams (one slot per stream, keyed
  per target for traffic; writing over an unconsumed slot *is* the
  coalescing — no queue scanning, and bursts can never build a backlog by
  construction),
- the existing **timer queue**.

Recording still captures messages at dequeue, so the replay log remains
exactly what the core saw, in order. This is an implementation note more than
a design change, but it is worth writing down before `core-state` builds the
clever channel.

One consequence to make explicit: mailbox coalescing means *inputs* can be
dropped (superseded) before the core sees them, which is fine for last-value
sensor data but must never apply to commands, device connection events, or
FLARM alarm transitions — the taxonomy of which input kinds coalesce belongs
in the doc.

### 5. Topics as projections with change detection

core.md's four topic kinds are right. What is unspecified is how publishing
stays correct: with manual `publish()` calls sprinkled through the reducer,
"forgot to publish after mutating X" becomes a recurring bug class invisible
to tests that assert on state.

Instead, derive: after each update cycle, run each topic's **projection
function** over the state, compare with the last published value (cheap
equality at these payload sizes; per-key for keyed collections), and publish
if changed. A topic can then never be stale relative to state, by
construction, and the projection is the single place a topic's payload shape
is defined.

The exception is genuinely edge-triggered output: warning raised/escalated/
cleared/acknowledged events are transitions, not state, and stay explicit
emissions from the reducer (the *active set* half of that topic is still a
projection). This split — events emitted, values projected — is worth stating
in core.md.

### 6. Version the recording and snapshot formats explicitly

Two consequences of the (good) decision to record pure worker results as
completion markers and recompute on replay:

- **A recording is only faithful on the code version that wrote it.** Any
  change to worker logic changes the recomputed payload and therefore the
  state evolution. That's acceptable for a debug artifact, but it must be
  detectable: give the log a header (app version, git hash, protocol version)
  and refuse — or loudly warn — on mismatch, and record a **payload hash**
  with each completion marker so replay divergence is caught at the first
  diverging worker result ("diverged at input #N, node reach") instead of as
  a mysteriously different final state. A devmode flag for verbatim payload
  recording covers cross-version forensics.
- **Cross-architecture float determinism is a real hazard.** The same version
  on x86 CI and an ARM phone can differ through libm implementations and FMA
  contraction. The doc's rayon rules are necessary but not sufficient.
  Mitigate cheaply: avoid `mul_add` and platform-varying transcendentals in
  worker code where results feed replay (the `libm` crate gives portable
  results where needed), and empirically gate it — CI replays the golden
  corpus on both an x86 and an ARM runner and diffs the payload hashes.

Snapshots are a second compatibility surface: a resumed flight loads state
written by a possibly-older binary. Give snapshots a schema version; on
mismatch, degrade gracefully (fresh state + still resume IGC logging from the
incremental log) rather than attempting migration or crashing. The IGC log is
the flight record; the snapshot is merely convenience state and can be
discarded.

### 7. Specify slow-subscriber policy per topic kind

"Every topic delivers current state on subscribe, reconnect is resubscribe" is
excellent. Unspecified: what happens when a subscriber (a browser tab over
WebSocket on flaky cockpit Wi-Fi, per multi-client.md) consumes slower than
the core publishes. Without a policy, the answer becomes "unbounded buffer in
the transport layer" by default — a slow-motion memory leak (see also
maplibre-gl-js#6154 for the same failure shape on the map side).

The topic taxonomy already contains the answer; state it:

- **Last-value** and **reference** topics: conflate — a slow consumer skips
  intermediate values and always gets the newest. Bounded memory by
  construction.
- **Keyed collections**: conflate per key.
- **Event topics**: must not conflate (transitions matter). Bounded buffer;
  on overflow, drop the *subscription* and force a resubscribe — which is safe
  and cheap precisely because subscribe delivers the current active set.

The in-process native audio subscriber is exempt from transport concerns but
gets the same contract; warning events are low-rate, so its bound is never hit
in practice — the point is that the invariant is stated.

This is conflation as practiced by market-data distribution systems, and it
composes with the mailbox ingress (rec. 4): the same "latest wins per key"
idea on both edges of the core.

### 8. Define the worker failure policy

A panic in a rayon worker must not kill the app, and a wedged worker must not
silently never deliver. Catch unwinds at the effect-executor boundary and
convert them into an ordinary input (`ComputeFailed { node, error }`), which
is also what the recording sees — so a crash-in-worker bug is replayable like
any other. The dependency graph decides per node whether to retry, degrade
(keep showing the stale reach polygon with an age indicator), or surface a
devmode diagnostic. Same shape for `Io` effects (snapshot write failed →
input message → user-visible warning), which core.md already implies for
persistence but should state generally.

### 9. Grow determinism into deterministic simulation testing

The design pays for determinism; harvest it beyond replaying recorded flights:

- **Reference-model tests** (the [Firezone pattern](https://www.firezone.dev/blog/sans-io)):
  a small, obviously-correct model of a subsystem (source priority, warning
  lifecycle, timer queue) plus proptest's state-machine testing driving
  thousands of generated input interleavings per CI run against both model
  and core, comparing state. This finds the bugs hand-written scenarios miss,
  and the sans-IO API (rec. 1) is what makes it cheap.
- **Golden replay corpus in CI**: real recorded flights replayed on every PR,
  diffing state-evolution hashes — the regression net for "this refactor
  changed behavior", and (run cross-arch) the empirical float-determinism
  gate from rec. 6.

Neither requires new architecture — only that recs. 1 and 6 land first.

## Answering the Open Question: the Growing Track Resource

core.md leaves open how the ever-growing own-track resource is served without
full refetches. Concrete recommendation:

- The core assigns each appended track segment a **monotonic sequence number**
  and stable feature ID; the `track` topic (reference kind) publishes
  `{version: seq, url}`.
- The track URL accepts `?since=<seq>` and returns only the segments after it
  (plus the current head), as a FeatureCollection with IDs.
- The frontend applies increments with MapLibre's
  [`GeoJSONSource.updateData`](https://maplibre.org/maplibre-gl-js/docs/API/classes/GeoJSONSource/),
  which takes a [`GeoJSONSourceDiff`](https://maplibre.org/maplibre-gl-js/docs/API/type-aliases/GeoJSONSourceDiff/)
  (add/update/remove by feature ID) precisely to avoid re-parsing large
  sources for small changes. Note `updateData` requires every feature in the
  source to carry a unique ID — which the sequence numbering provides.
- A fresh or reconnecting subscriber fetches without `since` and gets the
  consolidated track (the core may simplify old geometry server-side at that
  point); after that, tail fetches only.

This keeps the "no geometry through the message channel" rule, adds no new
protocol machinery (it's still version + URL), and bounds per-update work on
both sides.

## Smaller Notes

- **WebSocket vs SSE** (server.md's open question): lean WebSocket. SSE is
  text-only (blocking the per-topic binary-encoding option core.md reserves)
  and shares the browser's 6-connections-per-host HTTP/1.1 limit with the
  bulk-geodata fetches on the same local server; a single multiplexed
  WebSocket sidesteps both, and the protocol's request IDs already make it
  duplex-capable. Commands can stay on REST regardless (their
  retry-with-request-ID semantics map cleanly to POST).
- **Core input shape** (devices.md's open question): the middle ground —
  semantic messages carrying provenance (source device ID + raw-sentence
  reference) — fits the architecture best: the reducer and the dependency
  graph stay vendor-agnostic, while devmode diagnostics and the capability
  observer retain full fidelity. The wire-faithful → semantic normalization
  point already exists in the adapter layer; keep vendor shapes from crossing
  into `updraft_core`.
- **Traffic dead reckoning** (traffic.md's open question): with topics
  conflating (rec. 7) the IPC-cost argument for core-side extrapolation
  weakens further — recommend raw states + timestamps over IPC once, frontend
  extrapolates at render time. Warning-relevant computation stays FLARM-side
  anyway per traffic.md, so nothing safety-critical depends on frontend
  smoothing.
- **Statecharts for mode-ful subdomains**: flight modes (cruise/circling,
  takeoff/landing), warning lifecycle (raised → escalated → acknowledged →
  cleared), and device connection lifecycle are little state machines; model
  them as explicit enum-with-transition-methods types inside their reducer
  modules rather than as loose booleans. No framework needed.
- **God-struct hygiene**: with ~200 roadmap features all shaped "state +
  commands + computed values + view", keep `apply()` from becoming one giant
  match by composing per-domain modules (traffic, task, airspace, devices,
  settings…), each owning its sub-state, its input subset, its reducer, and
  its projections, composed at the top level Elm-style. The dependency graph
  (rec. 3) is what lets cross-domain derived values (final glide) remain
  declarative instead of turning into cross-module calls.

## Suggested Sequencing

1. **Before `core-state`**: adopt recs. 1, 2, 4 — they dictate the crate
   layout, the `apply()`/effect signatures, and the ingress structure.
2. **With `core-time`**: nothing new — the timer queue design in core.md
   stands; `next_deadline()` is its sans-IO face.
3. **With `core-subscriptions`**: rec. 5 (projections) and rec. 7
   (slow-subscriber policy).
4. **With `core-workers`**: rec. 3 (dependency graph) and rec. 8 (failure
   policy).
5. **With `input-recording`**: rec. 6 (versioning, payload hashes).
6. **After core integration tests exist**: rec. 9 (reference models, golden
   corpus, cross-arch gate).
