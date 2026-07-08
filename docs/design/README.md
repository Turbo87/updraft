# Design Documentation

**Updraft** is a multi-platform soaring flight computer with a modern UI/UX in the spirit of WeGlide Copilot and Enroute Flight Navigation. It targets the full XCSoar feature envelope long-term, but ships incrementally, starting with rock-solid situational awareness.

**Architecture in one sentence:** a Rust core that owns all state, computation, and device I/O, exposed through a transport-agnostic API consumed by a SvelteKit + MapLibre GL JS frontend, packaged via Tauri on mobile and desktop, but equally runnable as a plain HTTP/WebSocket/SSE server serving the same frontend in a browser.

These documents describe the target design and architecture. The [roadmap.md](../roadmap.md) file tracks the implementation status.

## Goals & Scope

- **Primary audience: glider pilots.** The system is first and foremost a soaring flight computer (glide computer, tasks, thermals, final glide, FLARM/OGN traffic, IGC logging, and so on).
- **Flexible enough for adjacent domains:** paragliding and general aviation (GA) use should be possible without architectural changes. This mostly means that domain concepts (polars, task rules, navigation routes, alert types) are modeled as data and pluggable modules rather than hard-coded assumptions.
- **Platforms:** Android, iOS, Linux, macOS, and Windows as native apps from a single codebase, plus any modern browser via the server. Android is prioritized over iOS for the practical reason that the primary developer uses Android and has no iOS device to test with.
- **Offline-first and privacy-friendly:** the system must be fully functional in flight without connectivity. Online services (weather, live traffic, contest upload, live tracking) are optional enhancements.
- **Testability as a first-class constraint:** deterministic simulation and replay inputs plus a browser-driven e2e suite, so both humans and LLM agents can safely iterate on the codebase (see [testing.md](testing.md)).
- **License:** dual-licensed under MIT / Apache-2.0.

## Non-Goals (initially)

- **Own backend infrastructure.** Users import standard files (OpenAir, CUP, IGC). Built-in downloads from public sources (openAIP et al.) come in a later phase.
- **Post-flight scoring.** OLC/XC scoring remains the domain of the separate `score-rs` project and online platforms. Updraft's in-flight optimization needs will be served by a purpose-built real-time engine.
- **E-paper / e-reader devices** (Kobo etc., an XCSoar niche). Updraft targets phones, tablets, and desktops.
- **Feature parity with XCSoar in v1.** Parity is a long-term ambition, phased deliberately (see [roadmap.md](../roadmap.md)).

## Guiding Principles

1. **One source of truth.** All application state lives in a single Rust core. Every user interface (Tauri app, browser, secondary client) is a _view_ of that state, never an owner of it.
2. **Message-driven state.** The core is updated exclusively through messages (commands) and observed exclusively through queries and state-change notifications. There is no back door.
3. **Thin shells, thick core.** Platform integrations (Tauri, axum, device I/O) are kept as thin as possible. Business logic never lives in a shell or in the frontend, unless critical for performance or security.
4. **Testability first.** Every layer can be exercised in isolation with fake inputs (simulated time, replayed sensor data, scripted user actions), so that the system can be refactored with confidence.
5. **Safety-critical logic is native.** Warning generation, alert audio, and flight logging live on the native Rust side, never in the webview. Mobile platforms suspend webview JavaScript when the screen is off or the app is backgrounded, so the frontend is treated as pure presentation that may vanish at any moment without compromising safety (see [tauri.md](tauri.md)).

## Repository Shape

```
core/        Rust core library: state, messages, domain logic
libs/        Rust libraries (e.g. NMEA parsing, geodesy, units, …)
server/      axum server (REST + state stream, optional static hosting)
tauri/       Tauri shell for Android/iOS/Linux/macOS/Windows
frontend/    SvelteKit + maplibre-gl-js application
e2e/         Playwright test suite and replay fixtures
docs/        Documentation
```

The exact crate/package layout may evolve, but the dependency direction is fixed: `frontend`, `server`, and `tauri` depend on `core`'s message protocol, while `core` depends on nothing above it.

**Crate policy:** before writing any of the small parser/geometry crates, evaluate existing crates.io options and prefer contributing upstream over forking. Own crates live in `libs/updraft_<name>` directories.

## Documents

- [core.md](core.md): the Rust core, its message protocol, and the bulk geodata path
- [server.md](server.md): the axum host, headless mode, and its security model
- [tauri.md](tauri.md): the Tauri shell, mobile plugins, platform risks, and native safety constraints
- [frontend.md](frontend.md): the SvelteKit app, map, interaction model, and platform behaviors
- [devices.md](devices.md): device I/O, the parser stack, and auto-detection
- [simulator.md](simulator.md): simulator mode and IGC replay
- [traffic.md](traffic.md): FLARM and OGN traffic handling, merging, and warnings
- [data.md](data.md): offline data (basemap, terrain, aviation data), storage layout, and crash-safe persistence
- [multi-client.md](multi-client.md): primary/secondary operation
- [testing.md](testing.md): test strategy, simulation, and CI
- [devmode.md](devmode.md): hidden developer mode and debugging options
- [distribution.md](distribution.md): release channels and operations

Each document carries its own open questions inline. Cross-cutting phasing lives in [roadmap.md](../roadmap.md), terminology in [glossary.md](../glossary.md).
