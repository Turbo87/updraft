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
  input messages. Recording and replaying that sequence reproduces the exact
  same state evolution, which is the foundation for simulation mode, IGC
  replay, demo mode, and regression testing alike.
- **Time is an input.** The core never reads the wall clock directly. Time
  advances via messages. Tests can therefore run a "four hour flight" in
  milliseconds.

### Adapters

Everything that touches the outside world, e.g. device I/O (serial, Bluetooth,
BLE, TCP/UDP) and online services (weather, OGN, contest servers, live
tracking), sits behind Rust traits. Production code plugs in real adapters.
Tests plug in fakes.

The parsing of external data (NMEA sentences, vendor protocols, file formats)
lives in pure functions that are trivially unit-testable, separate from the
transport that carried the bytes.

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
- **Parsers & file formats:** Unit tests on pure functions with real-world sample data
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
