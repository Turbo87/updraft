# Core Architecture: Comparison of the Four Proposals

Comparison of the four competing core-architecture proposals against `main` and against
each other, with claim verification and empirical testing. Evaluation criteria, in order:
avoid unnecessary complexity; first-class automated testing; easy to comprehend; server
and Tauri as thin shells sharing the same code.

| Branch | Form | One-line thrust |
| --- | --- | --- |
| `claude/core-architecture-review-rnyrkn` | docs | One transport: Tauri embeds the axum server on loopback; SSE; verbatim replay; snapshots for readers |
| `claude/core-architecture-review-qk989e` | docs | Sans-IO poll API; protocol/core/runtime crate split; static computation graph; hardened recompute-replay |
| `claude/core-architecture-review-fbqjgl` | docs | Sans-IO pure reducer; plain bounded FIFO; dirty-flag+epoch workers; verbatim replay |
| `codex-core` | docs + code | "Deliberately ordinary" `App::handle(Input) -> Update`; sync-first computation; snapshot+changes stream; working walking skeleton |

**Method.** Each proposal was reviewed adversarially by an independent subagent; a fifth
subagent fact-checked the load-bearing external claims (float determinism, Tauri/wry
limitations, SSE/WebSocket behavior) against specs, bug trackers, and upstream docs.
`codex-core` was additionally built and tested empirically in this environment.

## Empirical results (codex-core)

The only proposal with code, and it holds up:

- `cargo test -p updraft_core -p updraft_server`: **25 tests pass** (app reducer, runtime
  FIFO/atomic-subscribe/slow-subscriber semantics, server routes, ts-rs binding exports).
- Regenerating ts-rs bindings produces **zero drift** against the committed files — the
  golden-file CI check works as designed.
- Frontend: **10 vitest tests pass**.
- **The Playwright e2e walking skeleton passes** (~6 s): `cargo run`-built server with
  `--simulation`, real headless Chromium, MapLibre rendering via software GL, positions
  POSTed to `/api/simulation/position` asserted via `getSource()` coordinates and
  `queryRenderedFeatures`. This validates testing.md's "WebGL in CI" prediction and the
  whole walking-skeleton test approach on the first try.

## Where all four proposals agree (treat as settled)

- Single-owner state machine; every mutation enters as a message/input; results of
  background work re-enter as inputs; no shared mutable state.
- Time advances via inputs, never read by the core. GPS time stays separate from
  scheduling time.
- Effects leave the core as data and are executed by host adapters.
- Warnings are computed synchronously in the update path, natively.
- Verbatim recording of external I/O results; replayability as the foundation of testing.
- ts-rs-generated TypeScript, committed, with a CI drift check; JSON as first encoding.
- Bulk geodata never crosses the message channel — served by URL reference + version.
- Playwright against the axum server is the flagship test layer.

The four proposals are variations on main's architecture, not rejections of it. Every one
of them keeps main's core identity ("pure function of its input sequence") and sharpens or
simplifies specific mechanisms.

## The six real decisions

### 1. Core API shape — adopt codex-core's, it is the same sans-IO idea with less ceremony

All four converge on "core performs no I/O, spawns no threads, owns no channel". The
differences are surface syntax:

- codex-core: `App::handle(&mut self, Input) -> Update { changes, effects }` (implemented, tested)
- fbqjgl: "pure update function" returning publishes + effects — semantically identical to
  a `&mut self` method in Rust; the "pure function" label is branding
- qk989e: `handle_input` / `poll_effect` / `next_deadline` poll surface — str0m's shape,
  but with an unspecified cycle boundary and no gain over returning `Update` at 10 Hz
  input rates (str0m's reason, allocation on a packet hot path, does not apply here)

Notably, qk989e/fbqjgl's own cited precedent (quinn-proto) uses `&mut self` methods, not a
pure function. There is **zero testability difference** between the shapes; the
testability win — shared by all four — is evicting threads/channels/rayon/tokio from the
core, which main's "core's own update loop" phrasing left ambiguous.

**Adopt:** codex-core's `handle() -> Update` as implemented. Add the earliest pending
timer deadline to the update output (fbqjgl) when timers land, so hosts can sleep
precisely. **Drop** main's `Clock` trait language — a trait the core calls is a hidden
read; a timestamp in the input is data in the replay log (codex-core is right here, and
the sibling docs still carrying "Clock trait" text alongside "time advances via messages"
were internally inconsistent).

**One structural fix to codex-core:** `CoreRuntime` (tokio) currently lives inside
`updraft_core`, so the domain crate carries a tokio dependency. Move the runtime out
(small `updraft_runtime` crate, or into the server crate only if Tauri ends up not needing
it — see decision 6). The domain core staying dependency-free is cheap discipline now and
what keeps whole-flight tests a plain loop. The separate `updraft_protocol` crate
(fbqjgl/qk989e) is a mechanical refactor that can happen whenever compile times or codegen
hygiene ask for it; not urgent at current size.

### 2. Input channel — plain bounded FIFO (fbqjgl/codex-core); reject priority lanes and coalescing for now

Main prescribes a "not a plain FIFO" channel (priority + coalescing). qk989e and rnyrkn
formalize that into command-FIFO + latest-value mailboxes. fbqjgl and codex-core argue a
plain bounded FIFO suffices, and the review supports them:

- Real input rates are ~50–150 msg/s (10 Hz GPS + baud-limited FLARM burst ≈ 50 PFLAA/s)
  against per-input costs of microseconds to tens of microseconds: ≈1–2 % duty cycle.
  Queues never build; a queued command waits ~ms even behind a full burst.
- Priority cannot improve the actual latency bound (the slowest synchronous stage).
- Lane/mailbox designs introduce a real bug class: every message kind must be classified
  coalescable-or-not, and misclassifying one future kind silently drops alarm edges. The
  FIFO cannot have that bug, and its recorded sequence is trivially exactly what the core saw.
- The LMAX Disruptor — cited by qk989e as precedent — is itself a bounded FIFO ring
  without priority lanes.

**Two amendments required** (from the fbqjgl review):

- **The full-queue policy must be block-the-producer, never drop.** FLARM collision
  alarms share the queue; kernel socket buffers absorb the backpressure and the BT reader
  is a dumb pipe. codex-core's `mpsc::channel(16).send().await` already does this;
  it needs to be stated as a rule, and the capacity deserves a more generous value.
- Keep the escape hatch documented (second command channel with biased poll, a host-side
  change) so the deferred decision doesn't become a forgotten one.

### 3. Computation & workers — sync-first (codex-core), with fbqjgl's scheme as the documented escalation path; reject the computation graph

- **Reject qk989e's static computation graph.** It is framework-building for what its own
  cadence table shows is ~8 synchronous values (cheaper to recompute unconditionally than
  to track dirtiness for) plus exactly **3 async jobs** (reach, task optimization, wind
  refinement). Its hidden cost — per-state-field change detection — is never priced. Keep
  its *vocabulary* (cadence classes, per-node invalidation reasoning, per-worker devmode
  stats) as convention, not machinery.
- **Adopt codex-core's default**: compute synchronously after relevant inputs; move a
  calculation to a background worker only when measured. Amendment (from the codex-core
  review): this deferral is only sound if the promised queue-depth/handler-duration
  instrumentation lands **with the first real sensor adapter**, and the docs should name a
  warning-latency budget (e.g. PFLAU-to-audio) that triggers escalation. Main's
  cadence/cost table (every-fix / every-vario / ~1 Hz / debounced-async) should be
  restored as the domain's cost model — codex-core deleted the rationale along with the
  mechanism.
- **When a job does go async, use fbqjgl's scheme**, which subsumes both main's
  per-worker-kind invalidation predicates and codex-core's generation ID: dirty flag + at
  most one job in flight per kind + results applied by default + a per-kind **epoch
  counter** bumped by semantically breaking changes (task replaced, position
  discontinuity). Two amendments: (a) a worker panic must be converted by the runtime
  into a `JobFailed { kind, epoch }` input — with one-in-flight bookkeeping, a lost
  completion otherwise wedges that worker kind for the rest of the flight; (b) "results
  are always applied" needs the discontinuity exception spelled out, because teleports are
  a *supported interaction* here (simulator drag, replay seek).

### 4. Replay of worker results — record verbatim (rnyrkn/fbqjgl/codex-core); reject recompute-on-replay

The clearest verdict of the review. Main (and qk989e, hardened) record pure worker
results as completion markers and recompute them during replay. Three of four proposals
independently rejected that, and the evidence is on their side:

- Rust guarantees bit-exact cross-platform floats only for basic ops (RFC 3514); `sin`/
  `cos`/`atan2` route to platform libm and are documented "non-deterministic … varies by
  platform" — glibc/musl/bionic/Apple differ at the ulp level, and real divergence bugs
  exist. A recording made in the cockpit (ARM/bionic) would not be trusted to replay on
  an x86 dev machine.
- Recompute-on-replay ties recordings to the algorithm version: qk989e concedes this by
  making recordings version-bound artifacts that replay *refuses* on mismatch — which
  destroys the primary use case (field bug recorded on release vX, debugged on HEAD) and
  prevents recordings from becoming lasting regression fixtures.
- Prior art agrees: rr records everything nondeterministic verbatim (and scopes
  determinism to same-machine); Factorio's recompute-style lockstep produced a decade of
  desync bug reports and ultimately required hand-written trig. qk989e's counter-measures
  (payload hashes, dual-architecture CI, pinned `libm` everywhere) are the cost of
  swimming against this current.
- Cost check: reach-polygon results ≈ 9–26 MB raw per 4-hour flight, zstd compresses
  coordinate JSON 3–8× (better with long windows on near-identical polygons), and the
  sensor input log itself is the same order of magnitude. Recording is opt-in anyway.

**Adopt:** record **all** worker results verbatim (fbqjgl), with rnyrkn's droppable
compressed sidecar as the storage shape and an explicit determinism scope statement
(bit-exact same build+platform; input-exact cross-platform). Keep recompute-and-compare
as a **CI verification mode** (all three proposals converge on this), and keep qk989e's
one genuinely good addition: divergence reporting by payload hash ("diverged at input #N,
worker `reach`") inside that CI mode. Float-determinism hygiene (sequential folds,
ordered maps) demotes from load-bearing replay invariant to golden-test stability
guideline. One honest scope note (from the fbqjgl review): verbatim recordings replay
the *recorded* behavior faithfully forever; as fixtures against evolved core logic they
are approximations — that is fine, but say it.

Also **refuted** during fact-checking, for the record: qk989e's `mul_add` ban.
`f64::mul_add` is IEEE-754 `fusedMultiplyAdd`, correctly rounded and deterministic —
one of the few math functions that *is* — and Rust never contracts `a*b+c` implicitly.
The real portability hazard is exclusively the libm transcendentals.

### 5. Outputs — codex-core's snapshot+changes stream now; keep main's topic taxonomy as vocabulary; audio via effects

codex-core replaces main's four-kind topic taxonomy with one stream: snapshot on
subscribe, then FIFO batches of `Change` values; reconnect = fresh snapshot; slow
subscribers are dropped and recover by reconnecting. This is implemented, its atomicity
guarantee (subscription + snapshot captured inside the runtime loop, no update can fall
between) is tested, and it is the simplest thing that satisfies "every topic delivers
current state on subscribe". The `Change` enum is grouped by domain, which is a coarse
topic key — per-client filtering can be added at the host later without touching the core.

**Adopt with amendments:**

- **Audio must not be a subscriber.** codex-core drives warning audio as
  `Effect::PlayWarning` instead of main's "in-process subscriber to the warning topics".
  This is strictly better: the safety-critical path stops sharing machinery (and drop
  policies) with UI streaming, and warning→audio becomes assertable in a unit test with
  no transport. Update tauri.md accordingly.
- Keep main's taxonomy (last-value / keyed / events+active-set / reference) in the docs
  as the **vocabulary for shaping Change payloads** — codex-core deleted it wholesale,
  losing the edge-triggered-warning-events reasoning and the reference-kind (version +
  URL) pattern that the bulk-geodata path still depends on. The taxonomy should describe
  payload semantics, not become per-topic stream machinery.
- State the slow-subscriber contract explicitly (drop + auto-reconnect + fresh snapshot)
  and make drops observable (log/counter). The 16-message buffer means a client stalled
  ~1.6 s at 10 Hz gets dropped — plausible for a copilot tablet on WiFi; the buffer and
  the reconnect livelock risk need one measured look before multi-client ships.

### 6. Transport — adopt the embedded-server direction (rnyrkn), staged behind the validation spike; reject a permanent dual-transport target

This is the decision with the most contradictory positions (rnyrkn: only the embedded
axum server, no Tauri IPC, no custom scheme; codex-core: "Tauri does not start a hidden
HTTP server"). The evidence collected favors rnyrkn's direction:

- **Both of rnyrkn's load-bearing claims verify.** wry cannot stream custom-protocol
  response bodies (wry #1404, open since 2024 — responses are fully-buffered
  `Cow<'static, [u8]>`; SSE over the custom scheme is outright impossible). Android
  System WebView fails HTTP Range requests against custom-scheme responses (multiple
  independent reports incl. tauri-apps discussion #12243), which breaks PMTiles range
  reads **on the primary platform**. The established workarounds are an embedded loopback
  HTTP server or buffering whole files — for a map-centric app, the custom-scheme bulk
  path main and codex-core assume is not viable on Android today.
- **The dual-transport position is internally contradictory anyway** — two reviewers
  converged on this independently: multi-client.md requires the *pilot's phone app* (the
  Tauri build) to host the axum server for copilot clients. The server ends up inside the
  Tauri app regardless; rnyrkn *deletes* a transport rather than adding a server. Once
  the server is in-process, a second Tauri-IPC protocol mapping + custom URI scheme + a
  second TypeScript client + build-time transport switching exist only to avoid using it.
- **Testing follows directly:** with one transport, Playwright exercises byte-for-byte
  the transport that ships on every platform — the strongest possible version of the
  "server and tauri mostly share the same code" requirement.

**But rnyrkn under-specified four mechanisms, all fixable** (and its two supporting
arguments about HTTP/2 and ws:// secure contexts are hollow under its own design — the
decision stands on the wry/Android facts and code-sharing, not on those):

1. **One multiplexed SSE stream, never per-topic streams.** Per-topic EventSources — as
   rnyrkn's own text proposes — collide with the ~6-connections-per-origin HTTP/1.1 limit
   (browsers never speak h2 over cleartext loopback; EventSource holds its connection
   forever; Chromium marks this Won't-Fix) while MapLibre fetches tiles from the same
   origin. codex-core's single `/api/state` stream is the right shape; this also defuses
   qk989e's main argument for WebSocket.
2. **Token bootstrap must be shell-injected** (initialization script or one-shot URL
   nonce). As written ("the served frontend receives the token at page load") any local
   process can load the page and receive the token.
3. **Fixed port + collision policy**, not an ephemeral port: the origin changes with the
   port, silently discarding localStorage/IndexedDB/OPFS map caches on every launch.
4. **Frontend assets on Android** are APK assets, not filesystem paths — embed the built
   frontend into the binary (`rust-embed`/`include_dir`) rather than `ServeDir`.

Plus one honest de-scoping: "no IPC at all" is overstated. File import works as plain
HTML file input + POST (identical in browser mode — a genuine simplification), and device
plugins are driven core→effect→shell rather than frontend→invoke. But a residue of
shell-mediated interactions (permission prompts, keep-awake, share-intent ingress) stays
on Tauri plugins/IPC. That residue is small and UI-independent; it does not undermine the
single *protocol* transport.

**Risk management:** rnyrkn's own "validation spike" is correctly identified and must run
at walking-skeleton time, covering specifically **iOS suspend/resume socket teardown**
(documented killer: iOS invalidates listener sockets on backgrounding) and **Android
doze** alongside the foreground service. Until the spike passes, codex-core's shipped
state (axum server + browser; Tauri shell untouched) loses nothing — the walking skeleton
is transport-final either way. If the spike fails on iOS, the fallback is scoped: keep
the embedded server on Android/desktop (where the custom scheme is broken) and accept
custom-scheme + buffered GeoJSON + no PMTiles-by-range on iOS only.

SSE itself is the right starting choice (auto-reconnect composes with
snapshot-on-subscribe into zero reconnect code — already demonstrated by codex-core's
implementation; binary framing is a hypothetical need, and qk989e's six-connection
argument dissolves at one multiplexed stream). Revisit WebSocket only on a measured need.

## What to adopt from each branch

**codex-core — adopt as the foundation (code and doc shape):**
- The walking skeleton, merged essentially as-is: `App`/`Flight`/`protocol`/`CoreRuntime`,
  server SSE stream, state client/stores, e2e pattern, ts-rs pipeline. It is clean,
  tested, and the only proposal that proved itself executable.
- Effects-as-data; time-as-input without a Clock trait; snapshot+changes stream;
  host-owned command dedup; authoritative-vs-presentation state split; OGN
  area-of-interest from the primary's viewport; operation-ID pattern for long-running ops.
- Required fixes before/at merge: supervise the runtime task (a panic currently freezes
  every client permanently — subscribe returns 503, which EventSource treats as fatal);
  add SSE keep-alive (axum default is none); make the simulation route accept an optional
  `observed_at` so e2e time is actually simulated; make slow-subscriber drops observable;
  add EventSource error/staleness handling in `state-client.ts`; give e2e its own CI job
  with a cached/prebuilt server; move `CoreRuntime` out of the domain crate; fix the
  map recentering on every fix; add a Vite dev proxy for `/api`.

**rnyrkn — adopt the transport direction and two design pieces:**
- Embedded-server single transport, amended as above, gated on the lifecycle spike.
- Kinematic state vectors (position/traffic topics carry position + track + speed + turn
  rate + climb + timestamp) with frontend dead-reckoning — resolves main's open question
  the way every FLARM display works, one message per update instead of per-frame traffic.
- Versioned persistence (snapshot schema version; discard, don't migrate; IGC resume
  unaffected).
- Defer: per-cycle Arc snapshots for readers, `imbl`, queries-off-the-loop (premature at
  this state size — codex-core's query-through-runtime is fine until measured); the
  wasm32 core build (YAGNI, and its "sequential workers" contradict "never block the
  loop" on one thread); the panic-supervision *ladder* (keep catch/reseed/quarantine as
  the eventual crash story; stage-bisection safe mode is speculative, and `catch_unwind`
  conflicts with `panic = "abort"` release profiles — a constraint the doc must state).

**fbqjgl — adopt the two hard-question answers:**
- Verbatim recording of all worker results + determinism-scope statement (decision 4).
- Plain bounded FIFO with block-on-full (decision 2), and the dirty-flag/one-in-flight/
  epoch worker scheme with the panic + discontinuity amendments (decision 3).
- `track?since=<seq>` tail serving — merged with qk989e's MapLibre `updateData`/
  feature-ID mechanics (verified against the MapLibre API docs) and the fresh-subscriber
  consolidation case.
- The small, explicitly versioned snapshot struct ("never a serde dump of the state").

**qk989e — adopt the writing, reject the machinery:**
- The "Alternatives Considered" section (actors, ECS, salsa, CQRS, signal graphs, CRDTs)
  — verified accurate, valuable for contributors, keep it in core.md.
- Topic-payload projection *discipline* (each payload has one definition site) and the
  hash-divergence reporting inside the CI verify mode.
- The decided open questions where its reasoning holds: device input shape (semantic
  messages with provenance); dead reckoning in the frontend. Its WebSocket lean is
  superseded by the single-multiplexed-SSE design.
- Reject: the computation-graph engine; the three-method poll surface; version-bound
  recordings; the `mul_add`/FMA float rules (factually wrong); the three-crate split as a
  day-one requirement (do `updraft_runtime` when the Tauri host lands; `updraft_protocol`
  when codegen hygiene asks).

## Scorecard

Subagent scores (1–10), for what they're worth as a summary:

| | soundness | simplicity | testability | thin shells | actionability |
| --- | --- | --- | --- | --- | --- |
| rnyrkn | 8 | 7 | 9 | 9 | 7 |
| qk989e | 8 | 5 | 9 | 9 | 7 |
| fbqjgl | 8 | 9 | 8 | 5* | 6 |
| codex-core | 7 | 9 | 8 | 6** | 9 |

\* fbqjgl's substance is thin-shell-friendly; it scored low only because it never names a
shared home for the host runtime — importing qk989e's shared-runtime idea fixes it.
\*\* codex-core's runtime *is* shared; the score reflects the dual-transport residue that
decision 6 removes.

## Suggested sequencing

1. **Merge codex-core's walking skeleton** with the listed fixes (runtime supervision,
   SSE keep-alive, simulated `observed_at`, drop observability, client error handling,
   CI e2e job, runtime out of the domain crate).
2. **Fold the doc decisions above into `docs/design/`** — one edit that reconciles
   core.md/server.md/tauri.md/testing.md/roadmap.md with decisions 1–6, restoring the
   rationale paragraphs codex-core dropped (cadence table, prioritization reasoning,
   effects-never-block rule, timer determinism) and updating multi-client.md's
   contradiction with the transport decision.
3. **Run the rnyrkn lifecycle spike** (embedded server across Android
   suspend/resume/doze + foreground service, iOS backgrounding) before building any
   Tauri protocol bridge. Its outcome picks between "webview speaks HTTP/SSE to the
   embedded server everywhere" and the scoped iOS fallback.
4. Grow features inside codex-core's shape, escalating to fbqjgl's worker scheme per
   calculation as measurements demand, with the instrumentation landing alongside the
   first sensor adapter.

## Key verified claims (references)

- **wry #1404** — custom protocol responses cannot stream (fully buffered);
  github.com/tauri-apps/wry/issues/1404
- **Android WebView Range failures over custom schemes** breaking PMTiles;
  github.com/orgs/tauri-apps/discussions/12243 (independent reports, incl. hyper-based
  localhost fallback); official tauri-plugin-localhost exists with documented risks
- **RFC 3514 float semantics** — basic ops bit-exact, no implicit FMA contraction; NaN
  payloads excluded; rust-lang.github.io/rfcs/3514-float-semantics.html
- **std transcendentals non-deterministic** across platforms (documented in `f64` docs);
  `mul_add` correctly rounded per IEEE 754 (deterministic — qk989e's ban refuted)
- **rr** records nondeterminism verbatim with same-machine determinism scope
  (rr-project.org); **Factorio** desync history incl. custom trig for lockstep
  (FFF-188, forums)
- **LMAX Disruptor** is a bounded FIFO ring buffer, no priority lanes
  (lmax-exchange.github.io/disruptor)
- **SSE**: UTF-8-only per WHATWG; EventSource auto-reconnect, but permanent failure on
  non-200; ~6 connections/origin on HTTP/1.1, Chromium Won't-Fix; browsers do not speak
  h2 over cleartext, so loopback is always HTTP/1.1; axum `Sse` sends **no keep-alive by
  default**
- **MapLibre `GeoJSONSource.updateData`** exists, requires stable feature IDs, avoids
  re-sending/re-parsing the source (maplibre.org API docs)
- **quinn-proto / str0m / rustls** are genuine sans-IO precedents; quinn-proto itself
  uses `&mut self` methods, not a pure function
