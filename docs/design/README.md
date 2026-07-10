# Design Documentation

**Updraft** is a soaring flight computer for several platforms. Its long-term goal is to cover the main XCSoar features. It will ship in small steps, starting with reliable situational awareness.

**Architecture summary:** a deterministic Rust core owns trusted flight state and decisions. A shared runtime owns clocks, I/O, workers, subscriptions, and resources. A Svelte frontend owns presentation. Clients use the same HTTP, SSE, and resource URLs in standalone and Tauri hosting.

These documents describe the target design and architecture. The [roadmap.md](../roadmap.md) file tracks the implementation status.

## Goals & Scope

- **Primary audience: glider pilots.** The system is first and foremost a soaring flight computer (glide computer, tasks, thermals, final glide, FLARM/OGN traffic, IGC logging, and so on).
- **Flexible enough for related flight types:** shared types should avoid glider-only assumptions when this is simple. Paragliding and general-aviation features may add or change their own modules later. The initial design does not need a plugin system for them.
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

1. **One trusted source for flight state.** Shared flight state lives in one Rust `App`. Each client owns its temporary presentation state. Saved display configuration belongs to one display and is not broadcast as flight state.
2. **A small typed flow.** Inputs mutate the core, queries read it, changes update clients, effects request outside work, and resources carry bulk data. There is no generic message bus.
3. **Thin hosts, explicit adapters.** Tauri and axum provide transport and platform bindings. Device, storage, network, audio, and worker adapters live in the shared Rust application layer without leaking I/O into the core.
4. **Testability first.** Every layer can be exercised in isolation with fake inputs (simulated time, replayed sensor data, scripted user actions), so that the system can be refactored with confidence.
5. **Safety-critical logic is native.** Warning generation, alert audio, and flight logging run in native Rust, never in the webview. Mobile systems may pause webview JavaScript when the screen is off or the app is in the background. Warnings and logging must still work when that happens (see [tauri.md](tauri.md)).

## Where a Feature Belongs

- Does it change shared flight decisions, safety behavior, or trusted state? → A core domain module
- Does it touch a clock, filesystem, network, device, platform API, or expensive worker? → A runtime or host adapter, requested through an effect when the core owns the decision
- Is it a large dataset, geometry, image, or growing history? → The resource path, with only identity and version in shared state
- Is it layout, formatting, animation, gesture handling, or estimating movement for rendering? → The frontend
- Is it durable but specific to one display? → Rust-side display-profile storage, outside the shared flight snapshot

These boundaries are defaults, not a framework. A feature should use the smallest path that meets its current needs.

## Repository Shape

```
libs/        Rust crates: updraft_core (state machine, no I/O, no threads),
             updraft_runtime (shared host runtime: input queue, effect
             executors, state-stream delivery), parsers, geodesy, units, …
server/      axum server: the single transport (REST + SSE + bulk data), standalone or embedded
tauri/       Tauri shell for Android/iOS/Linux/macOS/Windows (embeds the server)
frontend/    SvelteKit + maplibre-gl-js application
e2e/         Playwright test suite and replay fixtures
docs/        Documentation
```

The exact crate and package layout may change. Two rules stay fixed. `frontend`, `server`, and `tauri` depend on the core protocol, while the core depends on nothing above it. The `updraft_core` crate never depends on tokio, rayon, or an I/O library. Threads and I/O live in `updraft_runtime` and the hosts (see [core.md](core.md)).

**Crate policy:** before writing any of the small parser/geometry crates, evaluate existing crates.io options and prefer contributing upstream over forking. Own crates live in `libs/updraft_<name>` directories.

## Documents

- [core.md](core.md): the Rust core, its small typed flow, state ownership, and client contract
- [runtime.md](runtime.md): queues, timers, workers, effect execution, subscriptions, and resource storage
- [replay.md](replay.md): input recording, deterministic replay, CI recompute verification, and crash resume
- [server.md](server.md): the axum server (the single transport), headless mode, and its security model
- [tauri.md](tauri.md): the Tauri shell, the embedded server, mobile plugins, platform risks, and native safety constraints
- [frontend.md](frontend.md): the SvelteKit app, map, interaction model, and platform behaviors
- [devices.md](devices.md): device I/O, parsing, and auto-detection
- [simulator.md](simulator.md): simulator mode and IGC replay
- [traffic.md](traffic.md): FLARM and OGN traffic handling, merging, and warnings
- [data.md](data.md): offline data (basemap, terrain, aviation data), storage layout, and crash-safe persistence
- [aircraft.md](aircraft.md): aircraft profiles and built-in presets
- [multi-client.md](multi-client.md): primary/secondary operation
- [testing.md](testing.md): test strategy, simulation, and CI
- [devmode.md](devmode.md): hidden developer mode and debugging options
- [distribution.md](distribution.md): release channels and operations

Each document carries its own open questions inline. Cross-cutting phasing lives in [roadmap.md](../roadmap.md), terminology in [glossary.md](../glossary.md).
