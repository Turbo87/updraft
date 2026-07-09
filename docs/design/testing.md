# Testing & Simulation

Testability is an explicit architectural goal: the system's boundaries exist so that each layer can be tested and refactored independently, and so that LLM agents can safely iterate on the codebase. Determinism is what makes this cheap: time as an input, normalized observations, and one application interface (see [core.md](core.md)) mean tests are fast and non-flaky, and field inputs can become replayable test cases.

## Testing Pyramid

1. **Rust unit tests** (the bulk). Parsers, geodesy, airspace geometry, terrain sampling, and detection heuristics are pure functions. Tests use `insta` snapshots against a `testdata/` corpus of recorded captures from real devices, a deliberately cultivated shared asset. **Property-based fuzzing via `proptest`:** Bluetooth/TCP input is untrusted, so every parser crate, including `updraft_nmea` and its vendor families, OpenAir, CUP, and IGC, carries proptest suites asserting no-panic on arbitrary input plus round-trip properties where applicable. These run in normal CI.
2. **Domain-module tests.** Tests exercise each module through its public behavior rather than reaching into private helpers.
3. **Application scenario tests.** Feed commands, normalized observations, monotonic time, and effect results into `App::handle()`, then assert on snapshots, changes, and effects. Whole-flight scenarios such as takeoff detection, airspace warning sequences, device loss, and reconnect run at simulated time.
4. **Host contract tests.** Tauri and axum expose equivalent commands, queries, snapshots, and changes. Every protocol schema round-trips between Rust and generated TypeScript, and CI fails if committed types drift.
5. **Playwright e2e, the flagship layer.** Tests launch `updraft-server` with a **simulation profile**: normalized replay inputs, fixed datasets, and simulated monotonic time. Playwright drives the real frontend in a real browser: _"load this flight, tap the map here, expect the airspace dialog to list TMA Langen."_ No Tauri, no device, no real-time flakiness. Visual regression on key screens is optional later. Since the axum-served system and the Tauri app share the frontend, application, and protocol, these tests cover the vast majority of shipping behavior without requiring native automation on five platforms. **WebGL in CI:** stock headless Playwright Chromium renders MapLibre via its built-in software GL with no special setup. Map assertions target map state (`getStyle`, `getLayer`, `queryRenderedFeatures`) rather than pixels, so they hold across GL rasterizers. The frontend exposes a `testMode` flag that disables map easing and other nondeterminism, and tests await explicit map-idle and resource-rendered signals rather than timeouts.
6. **Thin Tauri smoke tests.** A small manual or emulator checklist covers plugin permissions, foreground-service lifecycle, native audio, and storage adapters. Everything above it is already covered.

## Simulation as a Feature, Not Scaffolding

The same normalized input path supports user-facing IGC replay and a simple simulator for ground training and demos. Dragging conflicts with normal tap-to-query and pan gestures, so simulator mode uses a **distinct interaction**, such as a dedicated drag handle or explicit "move aircraft" mode. The TCP transport doubles as the Condor interface and a test input adapter.

## CI

GitHub Actions: Rust tests + clippy, frontend check/lint, Playwright suite, contract-drift check, release builds per platform. The workflows themselves are audited by [zizmor](https://docs.zizmor.sh/).
