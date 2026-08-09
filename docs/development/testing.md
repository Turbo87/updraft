# Testing

Status: Current development guide

Tests stay at the layer that owns the behavior. Use a boundary test only when
the behavior crosses that boundary. The CI workflow in `.github/workflows/ci.yml`
is the source of truth for the complete validation set.

## Rust

Rust unit tests cover parsers, domain modules, resources, transport helpers, and
Tauri integration. `updraft_core` scenario tests exercise ordered inputs,
effects, persistence, and topic output through its public application boundary.

Run the non-Tauri workspace checks with:

```console
cargo fmt --all --check
cargo clippy --workspace --exclude updraft_tauri --exclude tauri-plugin-updraft --all-targets --all-features -- -D warnings
cargo test --workspace --exclude updraft_tauri --exclude tauri-plugin-updraft --all-features
cargo doc --workspace --exclude updraft_tauri --exclude tauri-plugin-updraft --no-deps --all-features
```

Run focused Tauri and plugin checks with:

```console
cargo clippy -p updraft_tauri --all-targets --all-features -- -D warnings
cargo test -p updraft_tauri --all-features
cargo clippy -p tauri-plugin-updraft --all-targets --all-features -- -D warnings
cargo test -p tauri-plugin-updraft --all-features
```

Use direct assertions for small deterministic values. Use Insta snapshots for
structured, multiline, serialized, or scenario output. Keep parser fixtures in
`testdata/` byte-exact when their line endings, checksums, signatures, or
extensions are part of the behavior.

## Frontend

Vitest covers TypeScript modules, Svelte stores, and Svelte components. Browser
component tests use Playwright through Vitest. Map component tests use a
controlled MapLibre style and data sources.

Run the frontend checks with:

```console
pnpm lint
pnpm build
pnpm build:storybook
pnpm check
pnpm test
```

Storybook presents isolated component states. It does not replace a component
or end-to-end behavior test.

## End-to-end tests

The Playwright suite builds and serves the frontend in browser mode. The root
layout selects `FakeClient` outside Tauri. `?testMode=1` exposes the fake client
and application state to tests and disables map transition duration.

These tests verify complete frontend flows, including settings routes, live
topic updates, MapLibre layers, follow mode, and map inspection. They do not run
the Rust core, Tauri commands, or Android plugins.

Run them with:

```console
pnpm build
pnpm test:e2e
```

## Android

Android JVM tests cover the Kotlin plugin classes. CI also compiles the Rust
Tauri packages for Android and builds a debug APK. The generated Android project
runs its unit tests with:

```console
cd tauri/gen/android
./gradlew :tauri-plugin-updraft:testDebugUnitTest
```

Physical-device behavior that JVM tests cannot prove belongs in a verification
record. The record must identify the tested behavior, environment, result, and
limitations without host-local paths or unique device identifiers.
