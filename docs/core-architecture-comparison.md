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
limitations, SSE/WebSocket behavior) against specs, bug trackers, and upstream docs, and
ran local experiments where possible (std-vs-`libm` bit divergence, clone-vs-`imbl`
micro-benchmark). `codex-core` was additionally built and tested empirically in this
environment. **Maintainer decision:** the `codex-core` code itself is set aside — the
implementation will be built from scratch against the synthesized design below. The code
is treated here purely as an experiment: evidence about feasibility and a source of
pitfalls, not a candidate for merging. The assessment below is adjusted accordingly.

## What the codex-core experiment established (code set aside)

Even without merging a line of it, running the branch settled real unknowns that a
from-scratch build inherits as validated assumptions:

- **The e2e strategy works exactly as testing.md predicted, on the first try**: a
  `cargo run` server with a simulation seam, real headless Chromium, MapLibre rendering
  via software GL with no special setup, assertions on map state (`getSource()`,
  `queryRenderedFeatures` after an awaited `idle`) rather than pixels. The
  walking-skeleton test layer is de-risked for whatever implementation is built.
- **The ts-rs committed-bindings + CI drift-check workflow functions as designed**
  (regeneration produced zero drift; the `TS_RS_EXPORT_DIR` + `git diff --exit-code`
  wiring is worth copying as a pattern).
- **The `handle(Input) -> Update` shape is buildable and stays small**: a complete
  position-to-map vertical slice (core, runtime, SSE server, typed client, store, e2e)
  fit in ~600 lines of ordinary code. That says the interface shape carries no hidden
  framework tax — though a position slice exercises none of the hard parts (workers,
  warnings, replay), so it validates the shape, not the architecture.
- **The code review produced a pitfall checklist** that the fresh implementation should
  treat as requirements (see _Suggested sequencing_): runtime-task supervision, SSE
  keep-alive, injected time at the simulation seam, slow-subscriber observability,
  client-side stream error/staleness handling, and CI layout for the e2e job.

## Where all four proposals agree (treat as settled)

- Single-owner state machine; every mutation enters as a message/input; results of
  background work re-enter as inputs; no shared mutable state.
- Time advances via inputs, never read by the core. GPS time stays separate from
  scheduling time.
- Effects leave the core as data and are executed by host adapters.
- Warnings are computed synchronously in the update path, natively.
- Verbatim recording of external I/O results; replayability as the foundation of testing.
- ts-rs-generated TypeScript, committed, with a CI drift check; JSON as first encoding.
  (Fact-check aside: the alternative, tauri-specta, only pays off when Tauri `invoke` is
  the primary API — under an HTTP/SSE transport plain ts-rs is the lower-risk fit, and
  tauri-specta v2 is still a release candidate.)
- Bulk geodata never crosses the message channel — served by URL reference + version.
- Playwright against the axum server is the flagship test layer.

The four proposals are variations on main's architecture, not rejections of it. Every one
of them keeps main's core identity ("pure function of its input sequence") and sharpens or
simplifies specific mechanisms.

## The six real decisions

### 1. Core API shape — adopt codex-core's, it is the same sans-IO idea with less ceremony

All four converge on "core performs no I/O, spawns no threads, owns no channel". The
differences are surface syntax:

- codex-core: `App::handle(&mut self, Input) -> Update { changes, effects }`
  (demonstrated buildable in its experiment)
- fbqjgl: "pure update function" returning publishes + effects — semantically identical to
  a `&mut self` method in Rust; the "pure function" label is branding
- qk989e: `handle_input` / `poll_effect` / `next_deadline` poll surface — str0m's shape,
  but with an unspecified cycle boundary and no gain over returning `Update` at 10 Hz
  input rates (str0m's reason, allocation on a packet hot path, does not apply here)

Notably, qk989e/fbqjgl's own cited precedent (quinn-proto) uses `&mut self` methods, not a
pure function. There is **zero testability difference** between the shapes; the
testability win — shared by all four — is evicting threads/channels/rayon/tokio from the
core, which main's "core's own update loop" phrasing left ambiguous.

**Adopt:** the `handle() -> Update` shape. Add the earliest pending timer deadline to the
update output (fbqjgl) when timers land, so hosts can sleep precisely. **Drop** main's
`Clock` trait language — a trait the core calls is a hidden read; a timestamp in the
input is data in the replay log (codex-core is right here, and the sibling docs still
carrying "Clock trait" text alongside "time advances via messages" were internally
inconsistent).

**One lesson from the experiment to bake into the fresh build:** codex-core placed its
tokio-based runtime *inside* the core crate, giving the domain crate a tokio dependency.
The from-scratch layout should keep the domain core dependency-free from day one — the
runtime (channel pump, effect executor, subscriber fan-out) lives outside it, in a small
shared `updraft_runtime` crate (or module) used by both hosts. That is cheap discipline
at greenfield cost and what keeps whole-flight tests a plain loop. The separate
`updraft_protocol` crate (fbqjgl/qk989e) remains a mechanical refactor for whenever
compile times or codegen hygiene ask for it; not required on day one.

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
  is a dumb pipe. A bounded async channel with an awaited `send` gives this for free —
  but it must be stated as a rule, with a generous capacity, so nobody "optimizes" it
  into a lossy `try_send` later.
- Keep the escape hatch documented (second command channel with biased poll, a host-side
  change) so the deferred decision doesn't become a forgotten one.

### 3. Computation & workers — sync by default, but the async seam is part of the skeleton; reject the computation graph

- **Reject qk989e's static computation graph.** It is framework-building for what its own
  cadence table shows is ~8 synchronous values (cheaper to recompute unconditionally than
  to track dirtiness for) plus a handful of async jobs (reach, task optimization/live
  scoring, wind refinement). Its hidden cost — per-state-field change detection — is
  never priced. Keep its *vocabulary* (cadence classes, per-node invalidation reasoning,
  per-worker devmode stats) as convention, not machinery.
- **Sync by default, but build the async seam from the start.** codex-core's "background
  work only after profiling demonstrates a need" is too strong: WeGlide live-score
  calculation is *known* to be slow, and other task calculations are expected to follow —
  these are day-one tenants of the async path, not a hypothetical escalation. So the
  seam (spawn effect carrying an input snapshot, result re-entering as an input) belongs
  in the core skeleton, while each individual calculation still starts synchronous and
  moves behind it per known cost or measurement. The codex-core review's amendment
  stands: queue-depth/handler-duration instrumentation lands **with the first real sensor
  adapter**, and the docs name a warning-latency budget (e.g. PFLAU-to-audio). Main's
  cadence/cost table (every-fix / every-vario / ~1 Hz / debounced-async) should be
  restored as the domain's cost model — codex-core deleted the rationale along with the
  mechanism.
- **For async jobs, use fbqjgl's scheme**, which subsumes both main's per-worker-kind
  invalidation predicates and codex-core's generation ID: dirty flag + at most one job in
  flight per kind + results applied by default + a per-kind **epoch counter** bumped by
  semantically breaking changes (task replaced, position discontinuity). Two amendments:
  (a) a worker panic must be converted by the runtime into a `JobFailed { kind, epoch }`
  input — with one-in-flight bookkeeping, a lost completion otherwise wedges that worker
  kind for the rest of the flight; (b) "results are always applied" needs the
  discontinuity exception spelled out, because teleports are a *supported interaction*
  here (simulator drag, replay seek).
- **Stateful workers are expected and fit the scheme.** Live scoring wants to retain
  acceleration state between rounds (incremental structures over the growing trace)
  rather than re-optimizing from scratch. Design: the runtime owns one persistent worker
  per kind with `run(&mut self, inputs) -> result`; one-in-flight already serializes all
  access to that state, and an **epoch bump doubles as the reset-state signal** (task
  replaced ⇒ optimizer caches are garbage). Worker state is a host-side cache — never
  core state, never snapshotted; after restart/resume it is cold and the first round is
  slower, which resume must tolerate anyway. Replay is unaffected because results are
  recorded verbatim (decision 4) — and statefulness is one more argument for that
  default, since recomputing a stateful worker requires replaying its whole invocation
  sequence in order (possible, thanks to the per-kind total order, but exactly the kind
  of fragility verbatim recording avoids; the CI verify mode does replay the sequence).
  The growing-trace input pairs naturally with the `track` sequence numbering: a job's
  input snapshot can be "points since seq N" instead of the full trace.

### 4. Replay of worker results — record verbatim (rnyrkn/fbqjgl/codex-core); reject recompute-on-replay

The clearest verdict of the review. Main (and qk989e, hardened) record pure worker
results as completion markers and recompute them during replay. Three of four proposals
independently rejected that, and the evidence is on their side:

- Rust guarantees bit-exact cross-platform floats only for basic ops (RFC 3514); `sin`/
  `cos`/`atan2` route to platform libm and are documented "non-deterministic … varies by
  platform". **Demonstrated empirically during this review on one x86_64 machine:** std
  (glibc) `sin` vs the `libm` crate (musl port) disagree on 6,165 of 200,000 inputs, and
  `atan2` on 22,494 of 100,000 — two libms, one machine, different bits. glibc ↔ bionic
  will differ likewise, so a recording made in the cockpit (ARM/bionic) cannot be trusted
  to replay bit-exactly on an x86 dev machine.
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

For completeness: recompute-on-replay *is* technically salvageable — but only by pinning
every transcendental in the core and workers to a version-pinned `libm` crate (Rapier's
`enhanced-determinism` approach, or Factorio's hand-written trig) plus a dual-architecture
CI fingerprint test. That is permanent discipline imposed on every future contributor to
buy a smaller log file, and it still leaves recordings version-bound. Verbatim recording
needs none of it.

Also **refuted** during fact-checking, for the record: qk989e's `mul_add` ban.
`f64::mul_add` is IEEE-754 `fusedMultiplyAdd`, correctly rounded and deterministic —
one of the few math functions that *is* (empirically 0/100,000 mismatches against
`libm::fma`) — and Rust never contracts `a*b+c` implicitly. The real portability hazard
is exclusively the libm transcendentals.

### 5. Outputs — codex-core's snapshot+changes stream now; keep main's topic taxonomy as vocabulary; audio via effects

codex-core replaces main's four-kind topic taxonomy with one stream: snapshot on
subscribe, then FIFO batches of `Change` values; reconnect = fresh snapshot; slow
subscribers are dropped and recover by reconnecting. It is the simplest design that
satisfies "every topic delivers current state on subscribe", and the `Change` enum is
grouped by domain, which is a coarse topic key — per-client filtering can be added at the
host later without touching the core. Full disclosure: of the six decisions this is the
one whose recommendation leaned most on codex-core's working implementation; with the
code set aside it still stands, but on reasoning alone (no consumer needs per-topic
filtering on day one — the local UI wants nearly everything, audio moves to effects, and
secondary clients are full displays), so the fresh implementation must carry its
invariants explicitly rather than inheriting them:

- **Atomic subscribe**: subscription registration and snapshot capture happen together
  inside the runtime loop, so no change can fall between them — this needs a test in the
  fresh implementation, not just a sentence (a late subscriber asserting its snapshot
  already contains earlier submissions is the cheap way to pin it).
- **Reconnect contract**: a dropped or reconnecting subscriber starts over with a fresh
  snapshot; there is no replay buffer, no sequence bookkeeping.

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
  response bodies (wry #1404, open since 2024, no linked PR — responses are fully-buffered
  `Cow<'static, [u8]>`; SSE over the custom scheme is outright impossible). Android
  System WebView breaks HTTP Range requests against custom-scheme responses — the first
  ranged read succeeds, subsequent ones fail with `net::ERR_FAILED` (tauri-apps
  discussion #12243, upstream Chromium bug; intermittent, which is worse than a clean
  failure) — breaking PMTiles range reads **on the primary platform**. The community's
  converged workaround is exactly an embedded loopback HTTP server (a hyper-based
  fallback in that same discussion; Tauri's own warned-about localhost plugin). For a
  map-centric app, the custom-scheme bulk path main and codex-core assume is not viable
  on Android today.
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
   process can load the page and receive the token. Related detail for both hostings:
   `EventSource` cannot set request headers, so the SSE stream authenticates via a
   query-param token or a cookie scoped to the loopback origin.
3. **Port strategy: an ephemeral port is fine.** An earlier draft argued for a fixed
   port to keep browser-side caches (origin-keyed storage) alive across launches;
   maintainer correction accepted: all durable caching lives on the Rust side and the
   frontend only ever loads from loopback, where a refetch is nearly as cheap as a cache
   hit. With the explicit rule that nothing durable lives in origin-keyed browser
   storage (durable state belongs to the core), binding port 0 is *simpler* than a fixed
   port — the collision policy disappears and the shell hands the bound origin plus the
   session nonce to the webview at startup. The standalone server keeps its fixed
   default port for dev/browser convenience.
4. **Asset serving in the Tauri hosting has two workable shapes** — pick one in the
   spike: (a) *single origin*: the webview loads everything from the embedded server
   (rnyrkn as written); the built frontend must then be embedded in the binary
   (`rust-embed`/`include_dir`), because APK assets are not filesystem paths `ServeDir`
   can serve; no CORS anywhere, and the Tauri and server hostings stay byte-identical.
   (b) *hybrid*: Tauri keeps serving the static frontend through its normal asset
   handler — fine over the custom scheme, since app assets are small and need no Range
   requests — and the page talks cross-origin to the embedded axum server for API + SSE
   + bulk geodata only; no embedding and no change to Tauri's asset pipeline, at the
   cost of CORS/`Origin` allowances for the webview origin and an asset path that
   differs between hostings (a delta Playwright does not cover). Either way the
   *protocol and bulk-data* surface stays the single shared axum transport, which is
   what the testing argument rests on; `ServeDir` itself remains relevant only to the
   standalone server.

Plus one honest de-scoping: "no IPC at all" is overstated. File import works as plain
HTML file input + POST (identical in browser mode — a genuine simplification), and device
plugins are driven core→effect→shell rather than frontend→invoke. But a residue of
shell-mediated interactions (permission prompts, keep-awake, share-intent ingress) stays
on Tauri plugins/IPC. That residue is small and UI-independent; it does not undermine the
single *protocol* transport.

Platform facts established by the fact-check, which shrink the footwork rnyrkn listed:

- **iOS needs no ATS exemption at all** — ATS explicitly does not apply to IP-literal
  hosts, so `http://127.0.0.1:<port>` works without configuration (rnyrkn's hedged "ATS
  exemption if needed" resolves to *not needed*).
- **Android** blocks cleartext in release builds below API 37 (Android 17 finally exempts
  loopback implicitly); until then it is one `network-security-config` file scoping
  `cleartextTrafficPermitted` to `127.0.0.1`.
- **ws:// to loopback is not blocked** by mixed-content rules in Chromium or Firefox
  (loopback is a potentially-trustworthy origin), and Tauri's iOS custom-scheme origin
  isn't subject to mixed-content blocking anyway — rnyrkn's ws-secure-context argument
  for SSE is a myth. SSE is still the right call, but on its merits: auto-reconnect
  composing with snapshot-on-subscribe, and no hand-rolled heartbeat/backoff code.
- **Tauri IPC itself is no longer a serialization bogeyman** — v2 has a raw-ArrayBuffer
  response path and channels for streaming. The case against the dual-transport design
  rests on the custom-scheme bulk path (wry #1404, Android Range bug) and on code
  sharing, not on IPC performance.

**Risk management:** rnyrkn's own "validation spike" is correctly identified and must run
at walking-skeleton time, covering specifically **iOS suspend/resume socket teardown**
(documented killer: iOS invalidates listener sockets on backgrounding) and **Android
doze** alongside the foreground service. Nothing blocks on it: the walking skeleton
(axum server + browser, Tauri shell untouched) is transport-final either way, so it is
built first and the spike runs before any Tauri protocol bridge exists. If the spike
fails on iOS, the fallback is scoped: keep the embedded server on Android/desktop (where
the custom scheme is broken) and accept custom-scheme + buffered GeoJSON + no
PMTiles-by-range on iOS only.

SSE itself is the right starting choice (auto-reconnect composes with
snapshot-on-subscribe into zero reconnect code — confirmed working in the codex-core
experiment; binary framing is a hypothetical need, and qk989e's six-connection argument
dissolves at one multiplexed stream). Revisit WebSocket only on a measured need.

## What to adopt from each branch

**codex-core — adopt the design decisions; the code is set aside (built from scratch instead):**
- Effects-as-data; time-as-input without a Clock trait; snapshot+changes stream (with the
  invariants from decision 5 carried explicitly); host-owned command dedup;
  authoritative-vs-presentation state split; OGN area-of-interest from the primary's
  viewport; operation-ID pattern for long-running ops.
- The walking-skeleton *pattern* it demonstrated — a position-to-map vertical slice with
  a simulation POST seam as the first e2e test — is the right first milestone for the
  fresh build, and its e2e/ts-rs wiring is worth copying as a pattern even where the code
  is not.
- Its docs alone are not the spec: the branch deleted main's rationale (cadence/cost
  table, prioritization reasoning, effects-never-block rule, timer determinism) and
  contradicts multi-client.md on the transport. The synthesized design — main's docs
  edited per decisions 1–6 — is the spec the fresh implementation builds against.

**rnyrkn — adopt the transport direction and two design pieces:**
- Embedded-server single transport, amended as above, gated on the lifecycle spike.
- Kinematic state vectors (position/traffic topics carry position + track + speed + turn
  rate + climb + timestamp) with frontend dead-reckoning — resolves main's open question
  the way every FLARM display works, one message per update instead of per-frame traffic.
- Versioned persistence (snapshot schema version; discard, don't migrate; IGC resume
  unaffected).
- Defer: per-cycle Arc snapshots for readers and queries-off-the-loop (premature at this
  state size — codex-core's query-through-runtime is fine until measured); the wasm32
  core build (YAGNI, and its "sequential workers" contradict "never block the loop" on
  one thread); the panic-supervision *ladder* (keep catch/reseed/quarantine as the
  eventual crash story; stage-bisection safe mode is speculative, and `catch_unwind`
  conflicts with `panic = "abort"` release profiles — a constraint the doc must state).
- Reject `imbl`: benchmarked during this review at traffic-table scale (48 entries),
  plain `Vec` clone+update is ~4× faster (43 ns vs 158 ns per op) and dependency-free;
  persistent structures pay off at 10^4+ elements or many live snapshots, neither of
  which applies.

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

With the code set aside, codex-core's actionability 9 needs an asterisk of its own: it
was earned almost entirely by the working skeleton. Its core.md alone is the thinnest of
the four documents (~1.3k words vs main's ~1.7k, fbqjgl's ~2.4k, qk989e's ~3.4k) and
would not stand alone as a build spec — which is why the sequencing below makes the
design-doc fold-in the first step, not an afterthought.

## Suggested sequencing

1. **Write the synthesized design first.** Fold decisions 1–6 into `docs/design/` —
   one edit reconciling core.md/server.md/tauri.md/testing.md/multi-client.md/roadmap.md:
   the `handle() -> Update` surface and dependency-free core crate; plain bounded FIFO
   with block-on-full; the async worker seam with stateful workers and epochs; verbatim
   recording with the CI verify mode; the snapshot+changes stream with its two invariants
   spelled out; the embedded-server transport with the amendments (one multiplexed SSE
   stream, shell-injected token, ephemeral port, asset-shape options). Restore the
   rationale main already had and codex-core's docs dropped (cadence/cost table,
   prioritization reasoning, effects-never-block rule, timer determinism), and resolve
   the multi-client contradiction. Since the implementation is built from scratch, these
   docs *are* the spec — under-specification here becomes improvisation later.
2. **Build the walking skeleton fresh against that spec**, replicating the slice the
   codex-core experiment proved viable (core → runtime → SSE → store → map → Playwright)
   and treating the experiment's pitfall list as requirements, not fixes: the runtime
   task is supervised (a swallowed panic must not leave clients frozen on a dead stream —
   EventSource treats a non-200 subscribe as permanently fatal); the SSE response sends
   keep-alives (axum's default is none); the simulation seam accepts injected
   `observed_at` so e2e time is genuinely simulated; slow-subscriber drops are observable
   (a `Full` drop is not a `Closed` cleanup); the frontend stream client handles errors
   and surfaces data staleness; e2e runs in its own CI job with a cached/prebuilt server
   binary; the dev loop proxies `/api` so `pnpm dev` works against a running server.
3. **Run the rnyrkn lifecycle spike** (embedded server across Android
   suspend/resume/doze + foreground service, iOS backgrounding) before building any
   Tauri protocol bridge. Its outcome picks between "webview speaks HTTP/SSE to the
   embedded server everywhere" and the scoped iOS fallback, and settles the
   asset-serving shape (single-origin vs hybrid, decision 6 item 4).
4. **Grow features inside that shape.** The async worker seam (spawn effect,
   dirty/one-in-flight/epoch, `JobFailed`) is part of the core skeleton — WeGlide live
   scoring and task optimization are its known first tenants, stateful workers included —
   while everything else stays synchronous until the instrumentation (landing with the
   first sensor adapter) says otherwise.

## Key verified claims (references)

- **wry #1404** — custom protocol responses cannot stream (fully buffered);
  github.com/tauri-apps/wry/issues/1404
- **Android WebView Range failures over custom schemes** breaking PMTiles;
  github.com/orgs/tauri-apps/discussions/12243 (independent reports, incl. hyper-based
  localhost fallback); official tauri-plugin-localhost exists with documented risks
- **RFC 3514 float semantics** — basic ops bit-exact, no implicit FMA contraction; NaN
  payloads excluded; rust-lang.github.io/rfcs/3514-float-semantics.html
- **std transcendentals non-deterministic** across platforms (documented in `f64` docs);
  empirically reproduced in this review: std-glibc vs `libm`-crate `sin` differ on
  6,165/200,000 inputs (1 ulp each), `atan2` on 22,494/100,000, on a single x86_64
  machine; `mul_add` correctly rounded per IEEE 754, 0/100,000 mismatches vs `libm::fma`
  (deterministic — qk989e's ban refuted)
- **iOS ATS does not apply to IP-literal hosts** — no exemption needed for
  `http://127.0.0.1` (Apple Info.plist key reference); **Android** requires a
  network-security-config for loopback cleartext below API 37, implicit exemption from
  Android 17
- **Mixed-content rules exempt loopback** (potentially-trustworthy origin) in Chromium
  and Firefox for both `http://` and `ws://` — "webviews block ws:// to localhost"
  refuted (W3C Secure Contexts, crbug 40386732)
- **imbl** maintained (7.0.0, 2026) but benchmarked ~4× slower than plain clone at
  traffic-table scale (48 × 72 B entries: 43 ns vs 158 ns per clone+update)
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
