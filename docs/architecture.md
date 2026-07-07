# System Architecture

This document describes the high-level architecture of the project. It is
deliberately light on individual features and instead focuses on the overall
structure of the system: the major components, how they communicate, and the
principles that guide their design.

## Goals & Scope

- **Primary audience:** glider pilots. The system is first and foremost a
  soaring flight computer (glide computer, tasks, thermals, final glide,
  FLARM/OGN traffic, IGC logging, …).
- **Flexible enough for adjacent domains:** paragliding and general aviation
  (GA) use should be possible without architectural changes. This mostly means
  that domain concepts (polars, task rules, navigation routes, alert types)
  are modeled as data and pluggable modules rather than hard-coded
  assumptions.
- **Platforms:** Android, iOS, Linux, macOS, and Windows as native apps from a
  single codebase, plus any modern browser via the server.
- **Offline-first and privacy-friendly:** the system must be fully functional
  in flight without connectivity. Online services (weather, live traffic,
  contest upload, live tracking) are optional enhancements.
- **License:** dual-licensed under MIT / Apache-2.0.

## Guiding Principles

1. **One source of truth.** All application state lives in a single Rust core.
   Every user interface (Tauri app, browser, secondary device) is a *view* of
   that state, never an owner of it.
2. **Message-driven state.** The core is updated exclusively through messages
   (commands) and observed exclusively through queries and state-change
   notifications. There is no back door.
3. **Thin shells, thick core.** Platform integrations (Tauri, axum, device
   I/O) are kept as thin as possible. Business logic never lives in a shell
   or in the frontend, unless critical for performance or security.
4. **Testability first.** Every layer can be exercised in isolation with fake
   inputs (simulated time, replayed sensor data, scripted user actions), so
   that the system can be refactored with confidence.
5. **Safety-critical logic is native.** Warning generation, alert audio, and
   flight logging live on the native Rust side, never in the webview. Mobile
   platforms suspend webview JavaScript when the screen is off or the app is
   backgrounded, so the frontend is treated as pure presentation that may
   vanish at any moment without compromising safety (see *Safety &
   Resilience*).

## The Rust Core

The core is a plain Rust library (no UI, no networking assumptions) that owns
a single **state struct** describing everything the application knows: own
position and air data, computed values (wind, MacCready, final glide),
the active task, traffic, airspace warnings, configuration, and so on.

It is interacted with through a small, well-defined surface:

- **Commands** mutate state (`SetMacCready`, `AdvanceTaskTurnpoint`,
  `AcknowledgeAirspaceWarning`, sensor updates, …).
- **Queries** read state (either full snapshots or scoped selections).
- **Events / subscriptions** notify observers about state changes, so UIs can
  render reactively instead of polling.

This message-based design has several important consequences:

- The core is **transport-agnostic**: the same messages flow over Tauri IPC,
  WebSocket, or a direct function call in a unit test.
- The core is **deterministic and replayable**: a flight is just a sequence of
  input messages (including the results posted back by async workers, see
  *Computation Pipeline* below). Recording and replaying that sequence
  reproduces the exact same state evolution, which is the foundation for
  simulation mode, IGC replay, demo mode, and regression testing alike.
- **Time is an input.** The core never reads the wall clock directly. Time
  advances via messages. Tests can therefore run a "four hour flight" in
  milliseconds.

### Computation Pipeline

After each batch of input messages the core runs a staged pipeline whose
stages update at different cadences:

- **Every fix** — ground speed, track, GPS/pressure altitude fusion.
- **~1 Hz** — airspace-proximity lookahead, nearest-waypoint ranking.
- **Debounced / async** — glide-reach polygon, task optimization, and
  wind-estimation refinement.

Expensive, CPU-bound stages run on a rayon worker pool and must never block
the state machine. I/O-bound work from the outside world, such as pulling OGN
traffic, is handled by async tokio tasks in the adapter layer. A
worker posts its result back **as another input message**, tagged with the
input generation it was computed from, so the core can detect and discard
stale results.

This has a subtle but important consequence for determinism: **a recorded
input log must include the results posted back by async workers.** Replay then
re-injects those recorded results in their original order rather than
recomputing them, so replay is independent of worker timing and scheduling.
(Recomputing during replay is a separate, opt-in verification mode.) Note also
that the injected clock drives scheduling only. It is never conflated with fix
timestamps, which carry their own GPS time for logging and flight data.

### Adapters

Everything that touches the outside world, e.g. device I/O (serial, Bluetooth,
BLE, TCP/UDP) and online services (weather, OGN, contest servers, live
tracking), sits behind Rust traits. Production code plugs in real adapters.
Tests plug in fakes.

The parsing of external data (NMEA sentences, vendor protocols, file formats)
lives in pure functions that are trivially unit-testable, separate from the
transport that carried the bytes.

A device *driver* is the device-specific knowledge — which sentences to parse
and which outbound personality to speak — layered on top of a shared
transport. Crucially it is deliberately *not* the owner of a connection. A
single byte stream is processed by a **framer** (splitting bytes into
sentences or frames) feeding a **dispatcher** that routes each sentence to
whichever registered parsers claim it — `$GP*`/`$GN*` to the generic NMEA
parser, `$PFLA*` to FLARM, `$LXWP*` to LXNav, and so on. A device such as an
LX9000 that emits NMEA, FLARM pass-through, and LXNav sentences on one port
simply has three parsers active on that one stream, all feeding the core.

New connections start in an **identification mode** with parsers enabled
promiscuously. After a short observation window (or on a signature sentence)
the stream is tagged with detected capabilities ("GPS", "FLARM", "Vario").
Those tags drive a data-priority scheme (e.g. an external FLARM GPS outranks
the internal GPS), decide which outbound *device personality* attaches for
task declaration and MacCready/ballast sync, and populate the device-manager UI.
Manual override remains available for unusual hardware. Framing, dispatch, and
detection are all pure functions over sentence sequences, tested against a
corpus of recorded captures.

## Protocol & Data Paths

The core's message protocol is not a single wire format. The encoding is
chosen per interaction by shape and frequency, while the *contract* stays
transport-agnostic.

- **Commands and queries** are low-frequency and latency-insensitive (load a
  file, change a setting, connect a device, "what's at this point?"). These
  use plain JSON. The Rust types are the source of truth, and the matching
  **TypeScript types are generated** from them, so the two sides cannot
  drift. Generated types are committed, and CI fails if a regeneration would
  change them (golden-file check).
- **Subscription streams** are per-topic, and each topic uses the encoding
  that fits its shape and rate (for example plain JSON for low-rate state
  slices, and a compact binary frame for a high-rate stream such as the live
  vario signal).

**Bulk geodata never travels through the message channel.** Pushing map
tiles, airspace geometry, the glide-reach polygon, or the flight track
through IPC and into MapLibre is the biggest serialization trap, so the core
exposes that data as ordinary HTTP-style resources in *both* hosts: native
routes in the axum server, and a custom URI scheme (`updraft://tiles/…`,
`updraft://geojson/…`) in the Tauri shell that streams raw bytes without JSON
encoding. MapLibre consumes these as normal sources (vector/PMTiles basemap
and terrain, GeoJSON overlays).

For this geodata the frontend therefore only ever handles **references** —
source URLs plus version counters. When the reach polygon is recomputed, the
core bumps a version on the `reach` topic and the frontend calls
`source.setData(url)`. No geometry crosses the command channel.

## Frontends & Shells

### Svelte 5 Frontend

The UI is a single **Svelte 5** application using **maplibre-gl-js** for the
map (vector tiles, terrain shading, and the many map overlays the feature
list calls for). It contains presentation logic only: it renders state
received from the core and translates user interactions into commands. It
does not compute domain values itself. The number shown in a data field is
computed in the core, so it is identical on every platform and every
connected device.

Because the frontend speaks only the core's message protocol, the exact same
build runs inside the Tauri shell, served by the axum server, or against a
mocked message layer in component tests.

A few interaction principles are fixed early because they are hard to
retrofit and matter for in-flight use:

- **Dialogs, not bottom sheets.** Every secondary surface is a dialog: a
  centered modal on large screens, automatically fullscreen on small
  screens, from one responsive component with a consistent header (title +
  back/close).
- **A structured settings tree.** Settings form a nested hierarchy (Flight /
  Map / Airspace / Devices / Units / System …) — fullscreen pages with back
  navigation on mobile, master-detail on wide screens.
- **Glove- and turbulence-friendly targets.** In-flight controls have a
  minimum touch-target size on the order of 48px, generous hit space on
  map symbols, no action available only via long-press.

### Tauri Shell (primary target)

The **Tauri** application is the primary shipping artifact for all five
platforms. It embeds the Rust core in-process, hosts the Svelte frontend in
the system webview, and bridges the two over Tauri's IPC using the shared
message protocol. It is also where platform-specific concerns live:
permissions (location, Bluetooth), background execution, screen-keep-awake,
and access to native device APIs.

### axum Server

The **axum** server exposes the same Rust core over HTTP:

- **REST** for request/response interactions (queries, one-shot commands,
  file management),
- **WebSocket** (or SSE) for the continuous state-change stream.

It can run **headless**, driven purely over REST/WebSocket/SSE with no user
interface of its own. In this mode the system is inspected and controlled
entirely through those transports, which suits machine-to-machine integration
and automated testing.

It can also optionally serve the frontend's static assets, so any browser
becomes its display. A soaring computer on a Raspberry-Pi-class device in the
panel can run this way, and during development it is the fastest loop: run the
server, open the frontend in a browser, no native build required.

Crucially, the Tauri IPC bridge and the axum server are two thin transports
around the *same* core API. Feature code never needs to know which one is in
use.

## Safety & Resilience

A soaring computer gives warnings the pilot relies on, so the architecture
makes safety independent of the most fragile layer — the webview.

- **Safety-critical logic runs natively.** Airspace and traffic warning
  generation runs in the Rust core, and warning audio and IGC logging are
  performed by native adapters driven directly by the core, not in
  JavaScript. Android suspends webview JavaScript when the screen
  is off or the app is backgrounded. A foreground service keeps the *core*
  alive, not the webview. The frontend is pure presentation. If it is suspended,
  warnings still sound and the flight is still logged.
- **Audio is native.** Warning sounds are played on the native side and
  driven directly from the core, so they survive webview suspension and
  respect platform audio focus/ducking. The frontend may trigger non-critical
  UI sounds, but nothing safety-relevant depends on it being alive.
- **Crash and kill resilience.** A mid-flight crash or OS process kill must
  not lose the flight. IGC logs are written incrementally and flushed per fix
  batch. The core snapshots in-flight state (active task, logging status,
  device configuration) periodically with atomic writes. On startup the app
  detects an interrupted flight and resumes logging automatically.
- **Not a certified navigation source.** A first-run disclaimer and
  about-screen text state this explicitly — both an honest safety message and
  a smoother path through app-store review.

## Multi-Device: Primary / Secondary Operation

The architecture supports a **primary/secondary setup**: the pilot's device runs
the core (primary), and a copilot can connect from a secondary device over
the local network via the axum server.

- The primary's core remains the single source of truth.
- A secondary is "just another frontend": it authenticates, subscribes to the
  state stream, and may send commands, exactly like the local UI does.
- Roles and permissions decide what a secondary may change (e.g. the copilot can
  edit the task and acknowledge warnings, but the pilot's device settings stay
  local).

Because every UI is already a message-driven client of the core, this feature
falls out of the architecture rather than being bolted on. The same mechanism
covers related feature-list items such as repeater displays and two-seat
operation.

## Testing Strategy

Testability is an explicit architectural goal: the layering above exists so
that each layer can be tested and refactored independently.

- **Core logic:** Pure Rust unit tests, property-based tests for calculations (polar, final glide, task rules), and replay of recorded flights as regression fixtures
- **Parsers & file formats:** Unit tests on pure functions with real-world sample data, snapshot-tested against a shared corpus of recorded captures. Because device input over Bluetooth/TCP is untrusted, every parser also carries property-based fuzz tests (`proptest`) asserting it never panics on arbitrary bytes, plus round-trip properties where applicable
- **Transports (Tauri IPC, REST/WebSocket):** Integration tests asserting both transports expose identical core behavior
- **Frontend components:** Component tests against a mocked message layer
- **Whole system:** Playwright end-to-end suite

The **Playwright suite** drives the real frontend against a real core (via
the axum server) and uses the replay/simulation capability as its fixture
mechanism: a test boots the system, feeds it a scripted flight, and asserts
on what the pilot would actually see. Since the axum-served system and the
Tauri app share the frontend, the core, and the message protocol, these tests
cover the vast majority of shipping behavior without requiring native
automation on five platforms.

Determinism is what makes this cheap: injected time, replayable inputs, and a
message-only core API mean e2e tests are fast and non-flaky, and any bug found
in the field can be turned into a replayable test case.

## Repository Shape (indicative)

```
core/        Rust core library: state, messages, domain logic
libs/        Rust libraries (e.g. NMEA parsing, Geodesy, units, …)
server/      axum server (REST + WebSocket, optional static hosting)
tauri/       Tauri shell for Android/iOS/Linux/macOS/Windows
frontend/    Svelte 5 + maplibre-gl-js application
e2e/         Playwright test suite and replay fixtures
docs/        Documentation
```

The exact crate/package layout may evolve, but the dependency direction is
fixed: `frontend`, `server`, and `tauri` depend on `core`'s message protocol,
while `core` depends on nothing above it.
