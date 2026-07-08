# Testing & Simulation

Testability is an explicit architectural goal: the system's layering exists so that each layer can be tested and refactored independently, and so that LLM agents can safely iterate on the codebase. Determinism is what makes this cheap: injected time, replayable inputs, and a message-only core API (see [core.md](core.md)) mean tests are fast and non-flaky, and any bug found in the field can be turned into a replayable test case.

## Testing Pyramid

1. **Rust unit tests** (the bulk). Parsers, geodesy, airspace geometry, terrain sampling, and detection heuristics are pure functions. Tests use `insta` snapshots against a `testdata/` corpus of recorded captures from real devices, a deliberately cultivated shared asset. **Property-based fuzzing via `proptest`:** Bluetooth/TCP input is untrusted, so every parser crate (NMEA, FLARM, LXNav, OpenAir, CUP, IGC) carries proptest suites asserting no-panic on arbitrary input plus round-trip properties where applicable. These run in normal CI.
2. **Core integration tests.** The deterministic state machine pays off: feed a scripted input sequence (fixes, sentences, commands, injected clock) and assert on emitted topic updates. Whole-flight scenarios (takeoff detection, airspace warning sequence, device loss and reconnect) run in milliseconds at simulated time.
3. **Contract tests.** Every command/topic schema round-trips Rust ↔ generated TypeScript. CI fails if generated types drift from committed ones.
4. **Playwright e2e, the flagship layer.** Tests launch `updraft-server` with a **simulation profile**: replay transport playing an IGC/NMEA file at N× speed, fixed datasets, injected clock. Playwright drives the real frontend in a real browser: _"load this flight, tap the map here, expect the airspace dialog to list TMA Langen."_ No Tauri, no device, no real-time flakiness. Visual regression on key screens optional later. Since the axum-served system and the Tauri app share the frontend, the core, and the message protocol, these tests cover the vast majority of shipping behavior without requiring native automation on five platforms. **WebGL in CI:** stock headless Playwright Chromium renders MapLibre via its built-in software GL with no special setup. Map assertions target map state (`getStyle`, `getLayer`, `queryRenderedFeatures`) rather than pixels, so they hold across GL rasterizers. Screenshot comparison stays optional polish. The frontend exposes a `testMode` flag that disables map easing/animations and other nondeterminism sources, and tests await explicit "map idle" / "data version N rendered" signals rather than timeouts.
5. **Thin Tauri smoke tests.** The shell is kept so thin that a small manual/emulator checklist (plugin permissions, foreground-service lifecycle) suffices. Everything above it is already covered.

## Simulation as a Feature, Not Scaffolding

The same replay machinery ships to users: IGC replay mode and a simple simulator (set position/speed/track, fly with buttons or by dragging the ownship symbol) for ground training and demos. Dragging conflicts with the normal tap-to-query and pan gestures, so simulator mode uses a **distinct interaction**, a dedicated drag handle on the ownship symbol (or an explicit "move aircraft" mode) rather than overloading the plain drag gesture. The TCP transport doubles as the Condor interface _and_ a test injection point.

## CI

GitHub Actions: Rust tests + clippy, frontend check/lint, Playwright suite, contract-drift check, release builds per platform. The workflows themselves are audited by [zizmor](https://docs.zizmor.sh/).
