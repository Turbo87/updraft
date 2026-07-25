# Core and Shell Rewrite Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace `updraft_core`, `updraft_runtime` and the axum `server` with a functional core plus a Tauri shell, proven by a walking skeleton that carries an NMEA position fix from a TCP socket to the ownship symbol on the map.

**Architecture:** A pure `updraft_core` crate accepts `Input`s, updates state, and returns `Effect`s. It has no clock, no threads and no I/O. The `updraft_tauri` crate is the imperative shell: it drives a 10 Hz tick, owns the TCP transport, executes effects, and forwards emitted topics over a Tauri channel. The frontend receives topic messages through a single typed client with a hand-written fake for tests.

**Tech Stack:** Rust 2024, Tauri 2.11, `ts-rs` for TypeScript generation, `insta` for snapshot tests, Svelte 5 with runes, `svelte-maplibre-gl`, Vitest, Playwright.

This plan delivers milestone 1 of the six described in [the architecture spec](../specs/2026-07-25-app-architecture-design.md).

## Global Constraints

- The core crate must not depend on `tokio`, `tauri`, or any I/O crate. Its only dependencies are `updraft_geo`, `updraft_units`, `updraft_nmea`, `serde` and `ts-rs`.
- The core must never read a clock. Time enters as a `Timestamp` parameter.
- Rust edition is 2024, toolchain 1.97.1. Workspace lints in the root `Cargo.toml` apply to every crate.
- Dependency versions are pinned with `=` in this repository. Follow that convention for anything added.
- `ts-rs` stays behind a non-default `ts` feature so it never enters the shipping binary's dependency graph. CI runs `cargo test --workspace --all-features`, which picks the drift test up without anyone having to name the feature.
- Generated TypeScript lives in `frontend/src/lib/protocol/generated/` and is committed.
- Frontend uses `let` rather than `const` for local bindings (enforced by `eslint-plugin-prefer-let`).
- Run `pnpm lint` and `pnpm format` before committing frontend changes.
- Queries are deliberately **not** implemented in this milestone. Nothing in the walking skeleton needs one. They arrive in milestone 4 with "what's here".
- The previous `updraft_core` and `updraft_runtime` implementations are **not** a reference. Do not read them for guidance. They are deleted in task 1.

---

## File Structure

**Deleted**

- `server/` entirely
- `libs/updraft_runtime/` entirely
- `libs/updraft_core/src/{app,device,flight,job,protocol,time}.rs`
- `frontend/src/lib/protocol/{client,client.test,state.svelte,state.test}.ts`
- `e2e/tests/position.spec.ts`, whose map assertions are lifted into the replacement in task 13

**Core** (`libs/updraft_core/src/`)

| File | Responsibility |
| --- | --- |
| `lib.rs` | Crate docs, module wiring, re-exports |
| `time.rs` | `Timestamp` newtype |
| `connection.rs` | `ConnectionId`, `ConnectionSpec`, `ConnectionState` |
| `topic.rs` | `Topic` enum, `Instruments` and `LatLon` payloads |
| `input.rs` | `Input` enum |
| `effect.rs` | `Effect` enum |
| `decoder.rs` | Per-connection NMEA byte buffer driving `updraft_nmea::parse` |
| `core.rs` | `Core`, `CoreConfig`, `apply()` |
| `bindings.rs` | `ts-rs` generation and the up-to-date test |

`mod core;` is legal here because the crate is `updraft_core`, not `core`, and nothing in it reaches the `core` crate by path. Derive macros expand to `::core::…`, which a leading `::` keeps unambiguous.

Tasks below add modules to `lib.rs` one at a time. Keep the declarations grouped rather than interleaved: one alphabetical block of `mod` lines, a blank line, then one alphabetical block of `pub use` lines. By the end it reads:

```rust
mod bindings;
mod connection;
mod core;
mod decoder;
mod effect;
mod input;
mod time;
mod topic;

pub use connection::{ConnectionId, ConnectionSpec, ConnectionState};
pub use core::{Core, CoreConfig};
pub use decoder::Decoder;
pub use effect::Effect;
pub use input::Input;
pub use time::Timestamp;
pub use topic::{Instruments, LatLon, Topic};
```

(`bindings` is `#[cfg(feature = "ts")] pub mod bindings;` and has no re-export.)

**Shell** (`tauri/src/`)

| File | Responsibility |
| --- | --- |
| `lib.rs` | Tauri builder, tracing, wiring |
| `driver.rs` | Owns `Core`, consumes messages, drives the tick, executes effects, fans topics to subscribers |
| `transport/mod.rs` | Dispatch from `ConnectionSpec` to a transport |
| `transport/tcp.rs` | TCP client transport |
| `ipc.rs` | The `subscribe` Tauri command |

The shell's handle is `DriverHandle`, not `AppHandle`, because `tauri::AppHandle` is already in scope in `lib.rs` and two types with one name in a single crate is a trap.

**Frontend** (`frontend/src/lib/`)

| File | Responsibility |
| --- | --- |
| `client/index.ts` | `UpdraftClient` interface and topic message types |
| `client/tauri.ts` | Real client over the Tauri channel |
| `client/fake.ts` | Hand-written fake for tests and browser development |
| `stores/instruments.svelte.ts` | Reactive instruments store |

---

### Task 1: Remove the superseded crates and protocol layer

Deleting first, in its own commit, so the rewrite starts from a blank sheet and no reviewer has to guess which lines are new.

**Files:**
- Delete: `server/` (whole directory)
- Delete: `libs/updraft_runtime/` (whole directory)
- Delete: `libs/updraft_core/src/app.rs`, `device.rs`, `flight.rs`, `job.rs`, `protocol.rs`, `time.rs`
- Delete: `libs/updraft_core/tests/scenario.rs`
- Delete: `frontend/src/lib/protocol/client.ts`, `client.test.ts`, `state.svelte.ts`, `state.test.ts`
- Delete: `frontend/src/lib/protocol/generated/` (whole directory)
- Delete: `e2e/tests/position.spec.ts`
- Modify: `Cargo.toml`, `libs/updraft_core/src/lib.rs`, `libs/updraft_core/Cargo.toml`, `e2e/playwright.config.ts`, `e2e/package.json`, `.github/workflows/ci.yml`
- Modify: `frontend/src/routes/+layout.svelte`, `frontend/src/lib/flight-view/FlightView.svelte`, `frontend/src/lib/map/Map.svelte`, `frontend/src/lib/map/Map.stories.svelte`, `frontend/src/lib/map/Ownship.svelte`, `frontend/src/lib/map/ownship.ts`
- Create: `frontend/src/lib/gnss.ts`
- Create: `e2e/tests/.gitkeep`

**Interfaces:**
- Consumes: nothing
- Produces: an empty `updraft_core` crate that compiles, and a frontend that builds with the map temporarily driven by a local placeholder rather than the state stream

- [x] **Step 1: Delete the Rust crates and modules**

```bash
git rm -r server libs/updraft_runtime
git rm libs/updraft_core/src/app.rs libs/updraft_core/src/device.rs \
       libs/updraft_core/src/flight.rs libs/updraft_core/src/job.rs \
       libs/updraft_core/src/protocol.rs libs/updraft_core/src/time.rs \
       libs/updraft_core/tests/scenario.rs
```

- [x] **Step 2: Reduce the core crate to an empty shell**

Replace `libs/updraft_core/src/lib.rs` entirely:

```rust
//! The deterministic Updraft core.
//!
//! The core owns shared application state and the decisions based on it.
//! It performs no I/O, spawns no threads, and reads no clocks.
```

Replace `libs/updraft_core/Cargo.toml`:

```toml
[package]
name = "updraft_core"
version = "0.0.0"
edition = "2024"
license = "MIT OR Apache-2.0"

[lints]
workspace = true

[dependencies]

[dev-dependencies]
```

- [x] **Step 3: Drop the server from the workspace**

In `Cargo.toml`, change the members line:

```toml
members = ["libs/*", "tauri"]
```

- [x] **Step 4: Verify the workspace builds and tests**

Run: `cargo test --workspace`
Expected: success, no reference to `updraft_server` or `updraft_runtime`.

Use `cargo test`, not `cargo check`. `cargo check` does not build test targets, so it will not notice an integration test left behind referring to deleted types.

- [x] **Step 5: Delete the frontend state stream layer**

```bash
git rm frontend/src/lib/protocol/client.ts frontend/src/lib/protocol/client.test.ts \
       frontend/src/lib/protocol/state.svelte.ts frontend/src/lib/protocol/state.test.ts \
       e2e/tests/position.spec.ts
git rm -r frontend/src/lib/protocol/generated
```

The generated directory belongs to the deleted protocol. Task 8 regenerates it from the topic types and its drift test compares the whole directory listing, so anything stale left behind fails that test.

Three of those shapes still type the map components until task 12 moves them to `Instruments`. Create `frontend/src/lib/gnss.ts`:

```typescript
export type Availability<T> =
  { status: 'unavailable' } | { status: 'current'; value: T } | { status: 'lastKnown'; value: T };

/** A geographic latitude and longitude in degrees. */
export type LatLon = { latitudeDegrees: number; longitudeDegrees: number };

/** Selected GNSS components. */
export type GnssData = {
  position: Availability<LatLon>;
  altitudeMeters: Availability<number>;
  trackDegrees: Availability<number>;
  groundSpeedMetersPerSecond: Availability<number>;
};
```

and point the type imports in `FlightView.svelte`, `Map.svelte`, `Map.stories.svelte`, `Ownship.svelte` and `ownship.ts` at `$lib/gnss`. Task 12 deletes the module once the generated bindings replace it.

- [x] **Step 6: Point the layout at a placeholder position**

The layout, not the page, owns the subscription and renders the flight view. `+page.svelte` is empty. Replace the script block of `frontend/src/routes/+layout.svelte`:

```svelte
<script lang="ts">
  import '../app.css';
  import 'virtual:uno.css';

  import { page } from '$app/state';

  import favicon from '$lib/assets/favicon.svg';
  import FlightView from '$lib/flight-view/FlightView.svelte';
  import { getLocale } from '$lib/paraglide/runtime.js';
  import type { GnssData } from '$lib/gnss';

  let { children } = $props();

  const PLACEHOLDER: GnssData = {
    position: { status: 'unavailable' },
    altitudeMeters: { status: 'unavailable' },
    trackDegrees: { status: 'unavailable' },
    groundSpeedMetersPerSecond: { status: 'unavailable' },
  };
  const testMode = new URLSearchParams(window.location.search).get('testMode') === '1';

  $effect(() => {
    document.documentElement.lang = getLocale();
  });
</script>
```

and change the markup's flight view to `<FlightView gnss={PLACEHOLDER} {testMode} />`.

- [x] **Step 7: Remove the server from the Playwright config**

Replace `e2e/playwright.config.ts` entirely:

```typescript
import { defineConfig } from '@playwright/test';

const PORT = 4450;
const HOST = '127.0.0.1';
const BASE_URL = `http://${HOST}:${PORT}`;

export default defineConfig({
  testDir: './tests',
  use: {
    baseURL: BASE_URL,
    screenshot: 'only-on-failure',
    trace: 'retain-on-failure',
  },
  webServer: {
    // `--host` is required: vite preview otherwise binds ::1 only, which the
    // IPv4 `url` below can never reach.
    command: `pnpm --filter @updraft/frontend preview --port ${PORT} --strictPort --host ${HOST}`,
    cwd: '..',
    gracefulShutdown: { signal: 'SIGINT', timeout: 5_000 },
    reuseExistingServer: false,
    timeout: 120_000,
    url: BASE_URL,
  },
});
```

- [x] **Step 8: Keep CI and the empty e2e suite green**

Deleting the only spec leaves `e2e/tests/` untracked (git does not track empty directories) and leaves Playwright with nothing to run, and the `e2e` CI job still builds the deleted server. All three break the branch until task 13 restores a spec.

In `.github/workflows/ci.yml`, delete the `cargo build -p updraft_server` step from the `e2e` job, and delete that job's `Swatinem/rust-cache` step with it, since no cargo command remains in the job.

In `e2e/package.json`, change the `test` script to `playwright test --pass-with-no-tests`.

Create `e2e/tests/.gitkeep` explaining that the directory is tracked so `testDir` resolves, and that both it and the flag come out in task 13.

- [x] **Step 9: Verify the frontend builds and tests pass**

Run: `pnpm build && pnpm check && pnpm test && pnpm lint && pnpm test:e2e`
Expected: all pass, with the e2e run reporting no tests. `pnpm build` comes first because `pnpm check` needs the paraglide output that the build generates, and those files are git-ignored.

- [x] **Step 10: Commit**

```bash
git add -A
git commit -m "Remove superseded core, runtime, and server crates" \
  -m "The core and runtime are rewritten from scratch against the new architecture spec, and the axum server is replaced by Tauri IPC plus a custom URI scheme. The frontend state stream layer goes with it; the map runs on a placeholder position until the new client lands."
```

---

### Task 2: Core timestamp and connection identity

**Files:**
- Create: `libs/updraft_core/src/time.rs`
- Create: `libs/updraft_core/src/connection.rs`
- Modify: `libs/updraft_core/src/lib.rs`
- Modify: `libs/updraft_core/Cargo.toml`

**Interfaces:**
- Consumes: nothing
- Produces: `Timestamp` with `from_millis(u64) -> Timestamp`, `as_millis(self) -> u64`, `saturating_since(self, earlier: Timestamp) -> Duration`. `ConnectionId(pub u32)`. `ConnectionSpec::Tcp { host: String, port: u16 }` with constructor `ConnectionSpec::tcp(host, port)`. `ConnectionState::{Connecting, Connected, Disconnected}`.

- [x] **Step 1: Write the failing test**

This task needs no new dependency: the one test uses `assert_eq!`. Dev-dependencies are added by the task that first uses them, so an unused one never lands in a commit.

Create `libs/updraft_core/src/time.rs`:

```rust
use std::time::Duration;

/// Monotonic time since the shell started, supplied with every input.
///
/// The core never reads a clock. Time is always passed in, which is what
/// makes a scripted sequence of inputs reproduce exactly.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Timestamp(Duration);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn measures_elapsed_time_without_going_negative() {
        let earlier = Timestamp::from_millis(1_000);
        let later = Timestamp::from_millis(1_250);

        assert_eq!(later.saturating_since(earlier), Duration::from_millis(250));
        assert_eq!(earlier.saturating_since(later), Duration::ZERO);
    }
}
```

Add to `libs/updraft_core/src/lib.rs`:

```rust
mod time;

pub use time::Timestamp;
```

- [x] **Step 2: Run the test to verify it fails**

Run: `cargo test -p updraft_core`
Expected: FAIL, `no function or associated item named 'from_millis' found`.

- [x] **Step 3: Implement `Timestamp`**

Insert into `libs/updraft_core/src/time.rs` above the test module:

```rust
impl Timestamp {
    pub const fn from_millis(millis: u64) -> Self {
        Self(Duration::from_millis(millis))
    }

    pub const fn as_millis(self) -> u64 {
        self.0.as_millis() as u64
    }

    /// Time elapsed since `earlier`, clamped at zero so a late or
    /// out-of-order input can never produce a negative duration.
    pub fn saturating_since(self, earlier: Timestamp) -> Duration {
        self.0.saturating_sub(earlier.0)
    }
}
```

- [x] **Step 4: Run the test to verify it passes**

Run: `cargo test -p updraft_core`
Expected: PASS, 1 test.

- [x] **Step 5: Add the connection types**

No test here: these are data definitions with one trivial constructor, and a test would only restate the struct literal. They are exercised throughout tasks 6 onward.

Create `libs/updraft_core/src/connection.rs`:

```rust
/// Identifies one link to an external device.
///
/// The identity travels with every byte the link produces, because
/// position-source arbitration and failover need to know which device a
/// value came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ConnectionId(pub u32);

/// How the shell should reach a device.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConnectionSpec {
    /// A TCP client link. Used for flight simulators, WiFi-attached
    /// instruments, and any device exposing a TCP server, as well as for
    /// feeding recorded NMEA during development.
    Tcp { host: String, port: u16 },
}

impl ConnectionSpec {
    pub fn tcp(host: impl Into<String>, port: u16) -> Self {
        Self::Tcp {
            host: host.into(),
            port,
        }
    }
}

/// What the shell reports back about a link.
///
/// The shell owns reconnection and backoff between an open and a close
/// effect, so `Disconnected` describes the current situation rather than a
/// request for the core to do anything.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConnectionState {
    Connecting,
    Connected,
    Disconnected,
}
```

Add to `libs/updraft_core/src/lib.rs`:

```rust
mod connection;

pub use connection::{ConnectionId, ConnectionSpec, ConnectionState};
```

- [x] **Step 6: Verify it compiles**

Run: `cargo test -p updraft_core`
Expected: PASS, 1 test.

- [x] **Step 7: Commit**

```bash
git add libs/updraft_core
git commit -m "core: Add \`Timestamp\` and connection identity types"
```

---

### Task 3: The instruments topic

**Files:**
- Create: `libs/updraft_core/src/topic.rs`
- Modify: `libs/updraft_core/src/lib.rs`
- Modify: `libs/updraft_core/Cargo.toml`

**Interfaces:**
- Consumes: nothing
- Produces: `Instruments { position: Option<LatLon>, track_degrees: Option<f64>, ground_speed_meters_per_second: Option<f64>, altitude_msl_meters: Option<f64> }`, `LatLon { latitude_degrees: f64, longitude_degrees: f64 }`, and `Topic::Instruments(Instruments)`.

Values are SI and unit-suffixed in their names. Conversion and formatting are the frontend's job.

The wire `LatLon` is distinct from `updraft_geo::LatLon`, which stores `Angle` and would serialize as radians. The core converts at the topic boundary. They are never both in scope in the same module.

Derive `ts_rs::TS` without `#[ts(export)]`. That attribute generates a test that writes TypeScript to the crate's default `bindings/` directory as a side effect of `cargo test --all-features`, which is the wrong place and would be committed by accident. Task 8 exports deliberately, to `frontend/src/lib/protocol/generated/`, via `Topic::export_all(&config)` — a `TS` trait method that needs only the derive.

- [x] **Step 1: Add serde and ts-rs to the core crate**

In `libs/updraft_core/Cargo.toml`:

```toml
[dependencies]
serde = { version = "=1.0.229", features = ["derive"] }
ts-rs = { version = "=12.0.1", optional = true }

[features]
ts = ["dep:ts-rs"]

[dev-dependencies]
insta = { version = "=1.48.0", features = ["json"] }
```

- [x] **Step 2: Write the failing test**

Create `libs/updraft_core/src/topic.rs`:

```rust
use serde::Serialize;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn topic_serializes_to_tagged_camel_case_json() {
        let topic = Topic::Instruments(Instruments {
            position: Some(LatLon {
                latitude_degrees: 50.823,
                longitude_degrees: 6.186,
            }),
            track_degrees: Some(270.0),
            ground_speed_meters_per_second: Some(45.0),
            altitude_msl_meters: None,
        });

        insta::assert_json_snapshot!(topic);
    }
}
```

Add to `libs/updraft_core/src/lib.rs`:

```rust
mod topic;

pub use topic::{Instruments, LatLon, Topic};
```

- [x] **Step 3: Run the test to verify it fails**

Run: `cargo test -p updraft_core`
Expected: FAIL, `cannot find type 'Topic' in this scope`.

- [x] **Step 4: Implement the topic types**

Insert into `libs/updraft_core/src/topic.rs` between the `use` line and the test module:

```rust
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct LatLon {
    pub latitude_degrees: f64,
    pub longitude_degrees: f64,
}

/// Fast-changing instrument values.
///
/// Every field is SI and names its unit. Conversion to display units and
/// formatting belong to the frontend.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct Instruments {
    pub position: Option<LatLon>,
    pub track_degrees: Option<f64>,
    pub ground_speed_meters_per_second: Option<f64>,
    pub altitude_msl_meters: Option<f64>,
}

/// One group of client-visible state, sent whole rather than as a delta.
///
/// Topics are grouped by how often they change, so a fast instrument
/// update does not pay to re-serialize slow-changing state.
///
/// Adjacently tagged so the wire form is `{ topic, value }` in both JSON
/// and the generated TypeScript. An internally tagged enum would generate
/// an intersection type, which is awkward to narrow on in the frontend.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "topic", content = "value", rename_all = "camelCase")]
pub enum Topic {
    Instruments(Instruments),
}
```

- [x] **Step 4: Accept the snapshot**

Run: `cargo insta accept`

Then read `libs/updraft_core/src/snapshots/updraft_core__topic__tests__topic_serializes_to_tagged_camel_case_json.snap` and confirm it shows `topic: instruments`, a nested `value`, camelCase keys, and a null `altitudeMslMeters`.

- [x] **Step 6: Run the test to verify it passes**

Run: `cargo test -p updraft_core`
Expected: PASS, 2 tests.

- [x] **Step 7: Commit**

```bash
git add libs/updraft_core
git commit -m "core: Add the instruments topic"
```

---

### Task 4: Inputs and effects

**Files:**
- Create: `libs/updraft_core/src/input.rs`
- Create: `libs/updraft_core/src/effect.rs`
- Modify: `libs/updraft_core/src/lib.rs`

**Interfaces:**
- Consumes: `ConnectionId`, `ConnectionSpec`, `ConnectionState`, `Topic` from tasks 2 and 3
- Produces: `Input::{Start, Tick, Bytes { connection, data }, ConnectionChanged { connection, state }}` with constructors `Input::bytes(connection, data)` and `Input::connection_changed(connection, state)`, plus `Effect::{Emit(Topic), OpenConnection { connection, spec }, CloseConnection { connection }}` with constructors `Effect::emit(topic)`, `Effect::open(connection, spec)` and `Effect::close(connection)`

No tests in this task. These are data definitions whose constructors are one-line struct literals, and a test asserting that a variant round-trips through a pattern match verifies the compiler rather than the code. They are covered in anger from task 6 onward.

`Input::Location` and `Input::Command` are deliberately absent. Location arrives with the Android plugin in milestone 2 and commands with settings in milestone 5. Adding empty variants now would be speculative.

- [x] **Step 1: Add `Input`**

Create `libs/updraft_core/src/input.rs`:

```rust
use crate::connection::{ConnectionId, ConnectionState};

/// Anything that may change core state.
///
/// Input kinds stay distinct rather than being flattened into one record
/// type, because they differ in trust, in semantics, and in what the
/// domain needs to know about their origin.
#[derive(Clone, Debug, PartialEq)]
pub enum Input {
    /// The first input the shell sends. Produces the effects needed to
    /// bring configured connections up.
    Start,
    /// A periodic wake-up, for policy that depends on elapsed time rather
    /// than on new data. Nothing uses it yet.
    Tick,
    /// Raw bytes from a device link, tagged with which link produced them.
    Bytes {
        connection: ConnectionId,
        data: Vec<u8>,
    },
    /// The shell reporting what happened to a link it was asked to maintain.
    ConnectionChanged {
        connection: ConnectionId,
        state: ConnectionState,
    },
}

impl Input {
    pub fn bytes(connection: ConnectionId, data: impl Into<Vec<u8>>) -> Self {
        Self::Bytes {
            connection,
            data: data.into(),
        }
    }

    pub fn connection_changed(connection: ConnectionId, state: ConnectionState) -> Self {
        Self::ConnectionChanged { connection, state }
    }
}
```

Add to `libs/updraft_core/src/lib.rs`:

```rust
mod input;

pub use input::Input;
```

- [x] **Step 2: Add `Effect`**

Create `libs/updraft_core/src/effect.rs`:

```rust
use crate::connection::{ConnectionId, ConnectionSpec};
use crate::topic::Topic;

/// A request for work that crosses the process boundary.
///
/// Effects exist only for I/O. Pure derivation stays inline in
/// `Core::apply()`. The shell matches exhaustively, so
/// a new effect cannot be silently ignored.
#[derive(Clone, Debug, PartialEq)]
pub enum Effect {
    /// Publish a topic to the frontend.
    Emit(Topic),
    /// Bring up and keep up a link. The shell owns reconnection and
    /// backoff until a matching [`Effect::CloseConnection`].
    OpenConnection {
        connection: ConnectionId,
        spec: ConnectionSpec,
    },
    /// Tear a link down and stop reconnecting it.
    CloseConnection { connection: ConnectionId },
}

impl Effect {
    pub fn emit(topic: Topic) -> Self {
        Self::Emit(topic)
    }

    pub fn open(connection: ConnectionId, spec: ConnectionSpec) -> Self {
        Self::OpenConnection { connection, spec }
    }

    pub fn close(connection: ConnectionId) -> Self {
        Self::CloseConnection { connection }
    }
}
```

Add to `libs/updraft_core/src/lib.rs`:

```rust
mod effect;

pub use effect::Effect;
```

- [x] **Step 3: Verify it compiles**

Run: `cargo test -p updraft_core`
Expected: PASS, 2 tests.

- [x] **Step 4: Commit**

```bash
git add libs/updraft_core
git commit -m "core: Add \`Input\` and \`Effect\` enums"
```

---

### Task 5: NMEA decoding per connection

**Files:**
- Create: `libs/updraft_core/src/decoder.rs`
- Modify: `libs/updraft_core/src/lib.rs`
- Modify: `libs/updraft_core/Cargo.toml`

**Interfaces:**
- Consumes: nothing from earlier tasks
- Produces: `Decoder` with `Decoder::default()`, `push(&mut self, data: &[u8])`, and `next_message(&mut self) -> Option<Message>` where `Message` is `updraft_nmea::Message`

`updraft_nmea::parse` is a pure function over a caller-owned buffer that handles framing, checksums and resynchronisation. The decoder's only job is owning that buffer per connection and draining complete sentences from it.

The buffer needs no explicit size cap. `parse` rejects a start marker not followed by a delimiter within its 1 KB sentence horizon, so a device emitting garbage can never grow the buffer without bound.

- [x] **Step 1: Add the dependencies**

In `libs/updraft_core/Cargo.toml`, under `[dependencies]`:

```toml
updraft_nmea = { path = "../updraft_nmea" }
```

and under `[dev-dependencies]`, for the assertions this task's tests use:

```toml
claims = "=0.8.0"
```

- [x] **Step 2: Write the failing test**

Create `libs/updraft_core/src/decoder.rs`:

```rust
use updraft_nmea::{Message, Step, parse};

#[cfg(test)]
mod tests {
    use super::*;
    use claims::assert_none;
    use std::assert_matches;

    const RMC: &[u8] = b"$GPRMC,120000.00,A,5049.38,N,00611.16,E,45.0,270.0,010126,,,A\r\n";
    const GGA: &[u8] = b"$GPGGA,120000.00,5049.38,N,00611.16,E,1,08,1.0,200.0,M,47.0,M,,\r\n";

    #[test]
    fn decodes_a_sentence_split_across_two_pushes() {
        let mut decoder = Decoder::default();

        decoder.push(b"$GPRMC,120000.00,A,5049.38,N,0");
        assert_none!(decoder.next_message());

        decoder.push(b"0611.16,E,45.0,270.0,010126,,,A\r\n");
        assert_matches!(decoder.next_message(), Some(Message::Rmc(_)));
    }

    #[test]
    fn drains_every_buffered_sentence_before_returning_none() {
        let mut decoder = Decoder::default();
        decoder.push(&[GGA, RMC].concat());

        assert_matches!(decoder.next_message(), Some(Message::Gga(_)));
        assert_matches!(decoder.next_message(), Some(Message::Rmc(_)));
        assert_none!(decoder.next_message());
    }

    #[test]
    fn recovers_after_leading_noise() {
        let mut decoder = Decoder::default();
        decoder.push(&[b"garbage bytes\r\n".as_slice(), RMC].concat());

        assert_matches!(decoder.next_message(), Some(Message::Rmc(_)));
    }
}
```

These fixtures carry no `*HH` checksum. `updraft_nmea` accepts checksum-less sentences and ends them at the next newline, which keeps the fixtures readable.

Add to `libs/updraft_core/src/lib.rs`:

```rust
mod decoder;

pub use decoder::Decoder;
```

- [x] **Step 3: Run the tests to verify they fail**

Run: `cargo test -p updraft_core decoder`
Expected: FAIL, `cannot find type 'Decoder' in this scope`.

- [x] **Step 4: Implement `Decoder`**

Insert into `libs/updraft_core/src/decoder.rs` between the `use` line and the test module:

```rust
/// Owns the byte buffer for one connection and drains framed sentences.
///
/// Framing, checksum validation and resynchronisation all live in
/// [`updraft_nmea::parse`].
#[derive(Debug, Default)]
pub struct Decoder {
    buffer: Vec<u8>,
}

impl Decoder {
    pub fn push(&mut self, data: &[u8]) {
        self.buffer.extend_from_slice(data);
    }

    /// Pulls the next complete sentence, discarding anything unframeable
    /// in front of it. Returns `None` when the buffer holds only a partial
    /// sentence.
    ///
    /// Named `next_message` rather than `next` because `Decoder` is not an
    /// iterator: it is refillable, and `None` means "not yet", not "done".
    pub fn next_message(&mut self) -> Option<Message> {
        loop {
            let mut remaining: &[u8] = &self.buffer;
            let step = parse(&mut remaining);
            let consumed = self.buffer.len() - remaining.len();
            self.buffer.drain(..consumed);

            match step {
                Step::Incomplete => return None,
                Step::Frame(message) => return Some(message),
                Step::Rejected(_) => {}
            }
        }
    }
}
```

- [x] **Step 5: Run the tests to verify they pass**

Run: `cargo test -p updraft_core decoder`
Expected: PASS, 3 tests.

- [x] **Step 6: Commit**

```bash
git add libs/updraft_core
git commit -m "core: Add per-connection NMEA decoder"
```

---

### Task 6: The `Core` and `apply()`

**Files:**
- Create: `libs/updraft_core/src/core.rs`
- Modify: `libs/updraft_core/src/lib.rs`
- Modify: `libs/updraft_core/Cargo.toml`

**Interfaces:**
- Consumes: everything from tasks 2 to 5
- Produces: `Core::new(config: CoreConfig) -> Core`, `Core::apply(&mut self, input: Input, at: Timestamp) -> Vec<Effect>`, `Core::topics(&self) -> Vec<Topic>`, and `CoreConfig { connections: Vec<(ConnectionId, ConnectionSpec)> }`

Behaviour: `Input::Start` emits an `OpenConnection` per configured connection. `Input::Bytes` decodes sentences, updates instruments, and emits the topic as soon as a value actually changes. Repeated sentences carrying identical values emit nothing, because the comparison is against previous state rather than against the previous message.

`Input::Tick` produces nothing in this milestone. It exists for the time-based policy that arrives with reconnection and stale-fix handling, and a test pins that it is currently inert.

The invalid-fix test carries a fully populated sentence rather than an empty one. An empty `V` sentence would pass whether or not the status guard existed, since every field would parse as `None` anyway — it would be a test that asserts nothing.

`CoreConfig::connections` is temporary. Configured connections become runtime-mutable, changed through a core input driven by the settings UI, in milestone 5. Nothing should come to depend on the list being fixed at construction.

- [ ] **Step 1: Add the approx dependency**

In `libs/updraft_core/Cargo.toml`, under `[dev-dependencies]`:

```toml
approx = "=0.6.0-rc2"
```

`updraft_geo` and `updraft_units` are deliberately **not** added. The core calls inherent methods (`latitude()`, `as_degrees()`, `as_meters_per_second()`, `as_meters()`) on values whose types arrive through `updraft_nmea`'s public API, and Rust resolves those without a direct dependency. Declaring one the crate never names by path is dead weight. A later milestone that names these types directly adds it then.

- [ ] **Step 2: Write the failing tests**

Create `libs/updraft_core/src/core.rs`:

```rust
use crate::connection::{ConnectionId, ConnectionSpec};
use crate::decoder::Decoder;
use crate::effect::Effect;
use crate::input::Input;
use crate::time::Timestamp;
use crate::topic::{Instruments, LatLon, Topic};
use std::collections::BTreeMap;
use updraft_nmea::{Message, RmcStatus};

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_abs_diff_eq;
    use claims::assert_some;
    use std::assert_matches;

    const RMC: &[u8] = b"$GPRMC,120000.00,A,5049.38,N,00611.16,E,45.0,270.0,010126,,,A\r\n";
    const LINK: ConnectionId = ConnectionId(1);

    fn config() -> CoreConfig {
        CoreConfig {
            connections: vec![(LINK, ConnectionSpec::tcp("127.0.0.1", 4353))],
        }
    }

    fn at(millis: u64) -> Timestamp {
        Timestamp::from_millis(millis)
    }

    #[test]
    fn start_opens_every_configured_connection() {
        let mut core = Core::new(config());

        let effects = core.apply(Input::Start, at(0));

        assert_eq!(
            effects,
            vec![Effect::open(LINK, ConnectionSpec::tcp("127.0.0.1", 4353))]
        );
    }

    #[test]
    fn fix_emits_instruments_immediately() {
        let mut core = Core::new(config());

        let effects = core.apply(Input::bytes(LINK, RMC), at(100));

        assert_matches!(effects.as_slice(), [Effect::Emit(Topic::Instruments(_))]);
        let [Effect::Emit(Topic::Instruments(instruments))] = effects.as_slice() else {
            unreachable!()
        };
        let position = assert_some!(instruments.position);
        assert_abs_diff_eq!(position.latitude_degrees, 50.823, epsilon = 1e-3);
        assert_abs_diff_eq!(position.longitude_degrees, 6.186, epsilon = 1e-3);
        assert_eq!(instruments.track_degrees, Some(270.0));
    }

    #[test]
    fn repeated_identical_sentences_emit_only_once() {
        let mut core = Core::new(config());
        let mut emissions = 0;

        for _ in 0..5 {
            emissions += core.apply(Input::bytes(LINK, RMC), at(100)).len();
        }

        assert_eq!(emissions, 1, "only the first sentence changed any value");
    }

    #[test]
    fn tick_emits_nothing() {
        let mut core = Core::new(config());
        core.apply(Input::bytes(LINK, RMC), at(100));

        assert_eq!(core.apply(Input::Tick, at(200)), vec![]);
    }

    #[test]
    fn bytes_from_an_unknown_connection_are_ignored() {
        let mut core = Core::new(config());

        let effects = core.apply(Input::bytes(ConnectionId(99), RMC), at(100));

        assert_eq!(effects, vec![]);
    }

    #[test]
    fn invalid_fix_does_not_publish_a_position() {
        let mut core = Core::new(config());

        // Fields are populated exactly as in a valid fix, so only the `V`
        // status can be what suppresses the emission.
        let effects = core.apply(
            Input::bytes(
                LINK,
                b"$GPRMC,120000.00,V,5049.38,N,00611.16,E,45.0,270.0,010126,,,N\r\n".as_slice(),
            ),
            at(100),
        );

        assert_eq!(effects, vec![]);
    }
}
```

Add to `libs/updraft_core/src/lib.rs`:

```rust
mod core;

pub use core::{Core, CoreConfig};
```

- [ ] **Step 3: Run the tests to verify they fail**

Run: `cargo test -p updraft_core app`
Expected: FAIL, `cannot find type 'Core' in this scope`.

- [ ] **Step 4: Implement `Core`**

Insert into `libs/updraft_core/src/core.rs` between the `use` lines and the test module:

```rust
/// Static configuration the core is built with.
///
/// `connections` is temporary. It becomes runtime-mutable through a core
/// input driven by the settings UI in milestone 5.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CoreConfig {
    pub connections: Vec<(ConnectionId, ConnectionSpec)>,
}

/// The deterministic application core.
///
/// The same ordered inputs and timestamps always produce the same
/// effects, which is what makes whole-flight scenario tests a plain loop
/// with no runtime, sleeps or wall clock.
#[derive(Debug)]
pub struct Core {
    config: CoreConfig,
    decoders: BTreeMap<ConnectionId, Decoder>,
    instruments: Instruments,
}

impl Core {
    pub fn new(config: CoreConfig) -> Self {
        let decoders = config
            .connections
            .iter()
            .map(|(id, _)| (*id, Decoder::default()))
            .collect();

        Self {
            config,
            decoders,
            instruments: Instruments::default(),
        }
    }

    /// Applies one input and returns the work it requires.
    ///
    /// `at` is supplied by the shell rather than read, which is what keeps
    /// the core deterministic.
    pub fn apply(&mut self, input: Input, at: Timestamp) -> Vec<Effect> {
        let _ = at;

        match input {
            Input::Start => self
                .config
                .connections
                .iter()
                .map(|(connection, spec)| Effect::open(*connection, spec.clone()))
                .collect(),
            Input::Bytes { connection, data } => self.decode(connection, &data),
            Input::ConnectionChanged { .. } => Vec::new(),
            Input::Tick => Vec::new(),
        }
    }

    /// The current value of every topic, for a client that has just
    /// subscribed and holds no state yet.
    pub fn topics(&self) -> Vec<Topic> {
        vec![Topic::Instruments(self.instruments)]
    }

    fn decode(&mut self, connection: ConnectionId, data: &[u8]) -> Vec<Effect> {
        let Some(decoder) = self.decoders.get_mut(&connection) else {
            return Vec::new();
        };

        decoder.push(data);

        let mut messages = Vec::new();
        while let Some(message) = decoder.next_message() {
            messages.push(message);
        }

        let before = self.instruments;
        for message in messages {
            self.handle_message(message);
        }

        if self.instruments == before {
            return Vec::new();
        }

        vec![Effect::emit(Topic::Instruments(self.instruments))]
    }

    fn handle_message(&mut self, message: Message) {
        match message {
            Message::Rmc(rmc) if rmc.status == RmcStatus::Active => {
                if let Some(position) = rmc.position {
                    self.instruments.position = Some(LatLon {
                        latitude_degrees: position.latitude().as_degrees(),
                        longitude_degrees: position.longitude().as_degrees(),
                    });
                }
                if let Some(course) = rmc.course_over_ground {
                    self.instruments.track_degrees = Some(course.as_degrees());
                }
                if let Some(speed) = rmc.speed_over_ground {
                    self.instruments.ground_speed_meters_per_second =
                        Some(speed.as_meters_per_second());
                }
            }
            Message::Gga(gga) => {
                if let Some(altitude) = gga.altitude {
                    self.instruments.altitude_msl_meters = Some(altitude.as_meters());
                }
            }
            _ => {}
        }
    }
}
```

- [ ] **Step 5: Run the tests to verify they pass**

Run: `cargo test -p updraft_core`
Expected: PASS, all tests.

`Gga::altitude` is `Option<Length>` and documented as altitude above mean sea level, so `as_meters()` is the right accessor and no geoid correction is needed here.

- [ ] **Step 6: Commit**

```bash
git add libs/updraft_core
git commit -m "core: Add \`Core\` with change-driven topic emission"
```

---

### Task 7: Scenario snapshot tests

**Files:**
- Create: `libs/updraft_core/tests/scenario.rs`
- Create: `testdata/nmea/basic.nmea`
- Create: `testdata/nmea/ignored.nmea`

**Interfaces:**
- Consumes: `Core`, `CoreConfig`, `Input`, `Timestamp`, `Effect` from task 6
- Produces: the scenario harness later milestones extend

This is the regression net described in the spec. The effect stream is the core's complete observable behaviour, so snapshotting it captures exactly what the UI would have seen over time. Floats are rounded to quantity-appropriate precision so a change in optimisation level cannot produce a diff.

- [ ] **Step 1: Create the fixtures**

Create `testdata/nmea/basic.nmea` with exactly these four lines:

```text
$GPRMC,120000.00,A,5049.38,N,00611.16,E,45.0,270.0,010126,,,A
$GPGGA,120001.00,5049.38,N,00611.16,E,1,08,1.0,200.0,M,47.0,M,,
$GPRMC,120002.00,A,5049.40,N,00611.20,E,46.0,271.0,010126,,,A
$GPRMC,120003.00,A,5049.42,N,00611.24,E,47.0,272.0,010126,,,A
```

and `testdata/nmea/ignored.nmea` with exactly these two:

```text
$GPRMC,120004.00,A,5049.42,N,00611.24,E,47.0,272.0,010126,,,A
$GPRMC,120005.00,V,5049.50,N,00611.40,E,60.0,300.0,010126,,,N
```

The first repeats the final line of `basic.nmea` verbatim, so it changes nothing. The second carries plausible position, track and speed but a `V` status, so it must be discarded entirely.

- [ ] **Step 2: Write the failing test**

Create `libs/updraft_core/tests/scenario.rs`:

```rust
use updraft_core::{ConnectionId, ConnectionSpec, Core, CoreConfig, Effect, Input, Timestamp, Topic};

const LINK: ConnectionId = ConnectionId(1);
const FIXTURE: &str = include_str!("../../../testdata/nmea/basic.nmea");
/// Sentences the core must not act on: a verbatim repeat of the last line
/// of `basic.nmea`, then a `V`-status fix carrying plausible values.
const IGNORED: &str = include_str!("../../../testdata/nmea/ignored.nmea");

/// Rounds to a quantity-appropriate precision so snapshots record real
/// behaviour changes and not last-bit float differences.
fn describe(effect: &Effect) -> String {
    fn number(value: Option<f64>, decimals: usize) -> String {
        value.map_or_else(|| "none".to_owned(), |v| format!("{v:.decimals$}"))
    }

    match effect {
        Effect::Emit(Topic::Instruments(instruments)) => {
            let position = instruments.position.map_or_else(
                || "none".to_owned(),
                |p| format!("{:.5},{:.5}", p.latitude_degrees, p.longitude_degrees),
            );

            format!(
                "instruments pos={position} track={} gs={} alt={}",
                number(instruments.track_degrees, 2),
                number(instruments.ground_speed_meters_per_second, 2),
                number(instruments.altitude_msl_meters, 1),
            )
        }
        Effect::OpenConnection { connection, spec } => format!("open {connection:?} {spec:?}"),
        Effect::CloseConnection { connection } => format!("close {connection:?}"),
    }
}

/// Replays `sentences` through a fresh core and returns the whole effect
/// stream, rendered.
fn replay(sentences: &str) -> Vec<String> {
    let mut core = Core::new(CoreConfig {
        connections: vec![(LINK, ConnectionSpec::tcp("127.0.0.1", 4353))],
    });

    let mut log: Vec<String> = core
        .apply(Input::Start, Timestamp::from_millis(0))
        .iter()
        .map(describe)
        .collect();

    for (index, line) in sentences.lines().enumerate() {
        let at = Timestamp::from_millis(1_000 + index as u64 * 1_000);
        let sentence = format!("{line}\r\n");
        log.extend(
            core.apply(Input::bytes(LINK, sentence.into_bytes()), at)
                .iter()
                .map(describe),
        );
    }

    log
}

#[test]
fn replaying_a_flight_produces_a_stable_effect_stream() {
    insta::assert_snapshot!(replay(FIXTURE).join("\n"));
}

#[test]
fn same_inputs_produce_same_effects() {
    assert_eq!(replay(FIXTURE), replay(FIXTURE));
}

/// Pins that neither guard can be removed without the snapshot noticing:
/// a repeated sentence must not re-emit, and a `V`-status fix must not be
/// applied at all.
#[test]
fn sentences_the_core_ignores_produce_no_effects() {
    let combined = format!("{FIXTURE}{IGNORED}");
    let with_ignored = replay(&combined);

    assert_eq!(
        with_ignored,
        replay(FIXTURE),
        "the ignored sentences changed the effect stream"
    );
}
```

`basic.nmea` alone cannot catch a regression in either guard: every one of its lines is an accepted, value-changing fix, so deleting the change-detection guard or the RMC status guard leaves the snapshot byte-identical. `ignored.nmea` is what closes that, and it stays a separate file so `basic.nmea` remains a clean feed for the manual check in task 13.

- [ ] **Step 3: Run the tests to verify they fail**

Run: `cargo test -p updraft_core --test scenario`
Expected: FAIL. The determinism test passes, the snapshot test fails because no snapshot is recorded yet.

- [ ] **Step 5: Accept the snapshot**

Run: `cargo insta accept`

Then read `libs/updraft_core/tests/snapshots/scenario__replaying_a_flight_produces_a_stable_effect_stream.snap` and confirm it shows one open effect followed by instrument emissions whose position advances. The GGA line adds an altitude without moving the position, so its emission repeats the previous coordinates. That is correct.

- [ ] **Step 5: Run the tests to verify they pass**

Run: `cargo test -p updraft_core --test scenario`
Expected: PASS, 3 tests.

Then verify the guards are genuinely pinned: temporarily delete the `if self.instruments == before` early return in `core.rs`, confirm `sentences_the_core_ignores_produce_no_effects` fails, restore it, and repeat for the `rmc.status == RmcStatus::Active` guard.

- [ ] **Step 6: Commit**

```bash
git add libs/updraft_core testdata/nmea
git commit -m "core: Add scenario snapshot tests"
```

---

### Task 8: TypeScript bindings

**Files:**
- Create: `libs/updraft_core/src/bindings.rs`
- Create: `libs/updraft_core/examples/generate_protocol_bindings.rs`
- Modify: `libs/updraft_core/src/lib.rs`
- Modify: `libs/updraft_core/Cargo.toml`
- Delete: `frontend/src/lib/protocol/generated/` contents, regenerated in this task

**Interfaces:**
- Consumes: `Topic`, `Instruments`, `LatLon` from task 3
- Produces: `frontend/src/lib/protocol/generated/{Topic,Instruments,LatLon}.ts` and a test that fails when they drift

This restores the golden-file mechanism that lived in `server/src/wire/bindings.rs`, moved to the crate that now owns the wire types.

`ts` stays a non-default feature so `ts-rs` never enters the shipping binary's dependency graph. CI covers the drift test through `cargo test --workspace --all-features`. `required-features` on the example turns "forgot the flag" into a clear build refusal rather than a confusing error.

- [ ] **Step 1: Add the test dependency and gate the example**

In `libs/updraft_core/Cargo.toml`, under `[dev-dependencies]`:

```toml
tempfile = "=3.27.0"
```

and at the end of the file:

```toml
[[example]]
name = "generate_protocol_bindings"
required-features = ["ts"]
```

- [ ] **Step 2: Write the generation module and its failing test**

Create `libs/updraft_core/src/bindings.rs`:

```rust
use crate::topic::Topic;
use std::path::{Path, PathBuf};
use ts_rs::TS as _;

/// Directory holding the TypeScript bindings committed for the frontend.
pub fn committed_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../frontend/src/lib/protocol/generated")
}

/// Writes the TypeScript bindings derived from the wire types.
pub fn generate(output_dir: &Path) -> std::io::Result<()> {
    match std::fs::remove_dir_all(output_dir) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error),
    }
    std::fs::create_dir_all(output_dir)?;

    let config = ts_rs::Config::new().with_out_dir(output_dir);
    Topic::export_all(&config).map_err(std::io::Error::other)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{committed_dir, generate};
    use claims::{assert_ok, assert_some};
    use std::collections::BTreeMap;
    use std::path::Path;

    const REGENERATE_COMMAND: &str =
        "cargo run -p updraft_core --features ts --example generate_protocol_bindings";

    fn read_dir_files(dir: &Path) -> BTreeMap<String, String> {
        let entries = assert_ok!(std::fs::read_dir(dir), "failed to read {}", dir.display());

        entries
            .map(|entry| {
                let path = assert_ok!(entry).path();
                let name = assert_some!(path.file_name()).to_string_lossy().into_owned();
                let contents = assert_ok!(
                    std::fs::read_to_string(&path),
                    "failed to read {}",
                    path.display()
                );
                (name, contents)
            })
            .collect()
    }

    #[test]
    fn committed_bindings_are_up_to_date() {
        let generated = assert_ok!(tempfile::tempdir());
        assert_ok!(generate(generated.path()));

        let committed = read_dir_files(&committed_dir());
        let regenerated = read_dir_files(generated.path());

        assert_eq!(
            committed.keys().collect::<Vec<_>>(),
            regenerated.keys().collect::<Vec<_>>(),
            "committed TypeScript bindings are out of date, run `{REGENERATE_COMMAND}`"
        );

        for (name, committed) in committed {
            let regenerated = assert_some!(regenerated.get(&name));
            assert_eq!(
                committed, *regenerated,
                "committed TypeScript binding {name} is out of date, run `{REGENERATE_COMMAND}`"
            );
        }
    }
}
```

Add to `libs/updraft_core/src/lib.rs`:

```rust
#[cfg(feature = "ts")]
pub mod bindings;
```

- [ ] **Step 3: Write the generator example**

Create `libs/updraft_core/examples/generate_protocol_bindings.rs`:

```rust
fn main() -> std::io::Result<()> {
    let output_dir = updraft_core::bindings::committed_dir();
    updraft_core::bindings::generate(&output_dir)?;
    println!("wrote TypeScript bindings to {}", output_dir.display());
    Ok(())
}
```

- [ ] **Step 4: Run the test to verify it fails**

Run: `cargo test -p updraft_core --all-features bindings`
Expected: FAIL. The committed directory still holds the old `Snapshot.ts`, `Change.ts` and friends, so the file lists differ.

- [ ] **Step 5: Regenerate the bindings**

Run: `cargo run -p updraft_core --features ts --example generate_protocol_bindings`
Expected: prints the output path. `frontend/src/lib/protocol/generated/` now contains `Topic.ts`, `Instruments.ts` and `LatLon.ts` only.

- [ ] **Step 6: Run the test to verify it passes**

Run: `cargo test -p updraft_core --all-features`
Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add libs/updraft_core frontend/src/lib/protocol/generated
git commit -m "core: Generate TypeScript bindings for the topic types"
```

---

### Task 9: The shell driver

**Files:**

- Create: `tauri/src/driver.rs`
- Modify: `tauri/src/lib.rs`
- Modify: `tauri/Cargo.toml`

**Interfaces:**

- Consumes: `Core`, `CoreConfig`, `Input`, `Effect`, `Timestamp`, `Topic` from the core
- Produces: `Driver::spawn(config: CoreConfig, open: OpenFn, tick_interval: Duration) -> DriverHandle`, where `DriverHandle` has `send(&self, input: Input)` and `subscribe(&self, sink: Sink)`, `type Sink = Box<dyn Fn(&Topic) -> bool + Send>` and `type OpenFn = Box<dyn Fn(ConnectionId, ConnectionSpec, DriverHandle) + Send>`

The driver owns the only mutable `Core` **and** the subscriber list. Everything reaches it as a message on one channel, so ordering stays deterministic even though transports, the tick and the UI all run concurrently, and no shared state needs a lock.

A subscriber is a `Sink` closure returning `false` once its consumer is gone, rather than a `tauri::ipc::Channel`, so the driver carries no Tauri dependency and tests subscribe with a plain closure. On subscribe the driver immediately sends the current value of every topic.

`open` is injected for the same reason: the driver never depends on the transport layer.

The driver runs for the process lifetime. Tying its exit to handles being dropped would model the wrong thing, because holding no handles is an ordinary state rather than a signal that work is finished. Explicit shutdown belongs as a `Message` variant if something ever needs it.

- [ ] **Step 1: Add dependencies**

In `tauri/Cargo.toml`, under `[dependencies]`:

```toml
updraft_core = { path = "../libs/updraft_core" }
tokio = { version = "=1.53.1", features = ["macros", "net", "rt-multi-thread", "sync", "time", "io-util"] }
```

and under a new `[dev-dependencies]`:

```toml
approx = "=0.6.0-rc2"
claims = "=0.8.0"
```

- [ ] **Step 2: Write the failing tests**

Create `tauri/src/driver.rs`:

```rust
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use updraft_core::{ConnectionId, ConnectionSpec, Core, CoreConfig, Effect, Input, Timestamp, Topic};

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_abs_diff_eq;
    use claims::assert_some;
    use tokio::time::timeout;

    const RMC: &[u8] = b"$GPRMC,120000.00,A,5049.38,N,00611.16,E,45.0,270.0,010126,,,A\r\n";
    const LINK: ConnectionId = ConnectionId(1);
    const PATIENCE: Duration = Duration::from_secs(5);

    fn config() -> CoreConfig {
        CoreConfig {
            connections: vec![(LINK, ConnectionSpec::tcp("127.0.0.1", 4353))],
        }
    }

    /// Subscribes and returns a receiver of every topic the driver emits.
    fn topic_stream(handle: &DriverHandle) -> mpsc::UnboundedReceiver<Topic> {
        let (sender, receiver) = mpsc::unbounded_channel();
        handle.subscribe(Box::new(move |topic: &Topic| {
            sender.send(topic.clone()).is_ok()
        }));
        receiver
    }

    /// Awaits topics until one carries a position, so the onboarding
    /// emission of empty state does not have to be counted.
    async fn next_position(receiver: &mut mpsc::UnboundedReceiver<Topic>) -> updraft_core::LatLon {
        loop {
            let received = timeout(PATIENCE, receiver.recv())
                .await
                .expect("a topic within the timeout");
            let Topic::Instruments(instruments) = assert_some!(received);
            if let Some(position) = instruments.position {
                return position;
            }
        }
    }

    #[tokio::test]
    async fn subscribing_delivers_current_state_immediately() {
        let handle = Driver::spawn(config(), Box::new(|_, _, _| {}), Duration::from_millis(100));
        let mut topics = topic_stream(&handle);

        let received = timeout(PATIENCE, topics.recv())
            .await
            .expect("onboarding topic within the timeout");

        assert_eq!(
            assert_some!(received),
            Topic::Instruments(updraft_core::Instruments::default())
        );
    }

    #[tokio::test]
    async fn decoded_fixes_reach_subscribers() {
        let handle = Driver::spawn(config(), Box::new(|_, _, _| {}), Duration::from_millis(100));
        let mut topics = topic_stream(&handle);

        handle.send(Input::bytes(LINK, RMC));

        let position = next_position(&mut topics).await;
        assert_abs_diff_eq!(position.latitude_degrees, 50.823, epsilon = 1e-3);
    }

    #[tokio::test]
    async fn start_asks_for_a_transport_per_configured_connection() {
        let (sender, mut receiver) = mpsc::unbounded_channel();
        let handle = Driver::spawn(
            config(),
            Box::new(move |connection, _spec, _handle| {
                let _ = sender.send(connection);
            }),
            Duration::from_millis(100),
        );

        handle.send(Input::Start);

        let requested = timeout(PATIENCE, receiver.recv())
            .await
            .expect("an open request within the timeout");
        assert_eq!(assert_some!(requested), LINK);
    }
}
```

No test sleeps. Each awaits the event it cares about under a generous timeout, so it finishes as soon as the work is done and fails fast rather than flaking on a loaded machine.

- [ ] **Step 3: Run the tests to verify they fail**

Run: `cargo test -p updraft_tauri driver`
Expected: FAIL, `cannot find type 'Driver' in this scope`.

- [ ] **Step 4: Implement the driver**

Insert into `tauri/src/driver.rs` between the `use` lines and the test module:

```rust
/// Receives every emitted topic. Returns `false` once its consumer is
/// gone, which is how the driver prunes dead subscribers.
pub type Sink = Box<dyn Fn(&Topic) -> bool + Send>;

/// Brings up the transport for a connection the core asked for.
///
/// Injected rather than called directly so the driver carries no
/// dependency on the transport layer and can be tested with a stub.
pub type OpenFn = Box<dyn Fn(ConnectionId, ConnectionSpec, DriverHandle) + Send>;

enum Message {
    Input(Input),
    Subscribe(Sink),
}

#[derive(Clone)]
pub struct DriverHandle {
    messages: mpsc::UnboundedSender<Message>,
}

impl DriverHandle {
    /// Queues an input. A dropped driver makes this a no-op rather than an
    /// error, because shutdown races are expected during teardown.
    pub fn send(&self, input: Input) {
        let _ = self.messages.send(Message::Input(input));
    }

    /// Registers a sink. It immediately receives the current value of
    /// every topic, so a client that reloads mid-flight resyncs without
    /// needing a distinct snapshot message.
    pub fn subscribe(&self, sink: Sink) {
        let _ = self.messages.send(Message::Subscribe(sink));
    }
}

/// Owns the single mutable [`Core`] and the subscriber list, and drives
/// both.
///
/// Keeping subscribers inside the task means there is no shared state and
/// therefore no lock, and the current-topics reply needed on subscribe is
/// just a local call.
///
/// The driver runs for the lifetime of the process. Holding no handles is
/// an ordinary state, not a reason to stop: a webview between reloads has
/// no subscriber, and a profile with no devices has no transport. If
/// stopping ever needs to be possible, it belongs as an explicit
/// [`Message`] variant rather than as a consequence of dropping a handle.
pub struct Driver;

impl Driver {
    pub fn spawn(config: CoreConfig, open: OpenFn, tick_interval: Duration) -> DriverHandle {
        let (sender, mut receiver) = mpsc::unbounded_channel();
        let handle = DriverHandle { messages: sender };
        let driver_handle = handle.clone();

        tokio::spawn(async move {
            let started = Instant::now();
            let mut core = Core::new(config);
            let mut sinks: Vec<Sink> = Vec::new();
            let mut ticker = tokio::time::interval(tick_interval);
            ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

            loop {
                let message = tokio::select! {
                    _ = ticker.tick() => Message::Input(Input::Tick),
                    received = receiver.recv() => match received {
                        Some(message) => message,
                        // Unreachable while the driver holds its own
                        // handle. Exiting is still the right response if
                        // that ever changes.
                        None => break,
                    },
                };

                let input = match message {
                    Message::Subscribe(sink) => {
                        if core.topics().iter().all(&sink) {
                            sinks.push(sink);
                        }
                        continue;
                    }
                    Message::Input(input) => input,
                };

                let at = Timestamp::from_millis(started.elapsed().as_millis() as u64);
                for effect in core.apply(input, at) {
                    match effect {
                        Effect::Emit(topic) => sinks.retain(|sink| sink(&topic)),
                        Effect::OpenConnection { connection, spec } => {
                            open(connection, spec, driver_handle.clone());
                        }
                        Effect::CloseConnection { connection } => {
                            tracing::warn!(
                                ?connection,
                                "close requested, but transports run for the process lifetime in this milestone"
                            );
                        }
                    }
                }
            }
        });

        handle
    }
}
```

- [ ] **Step 5: Wire the module in**

Add to `tauri/src/lib.rs` above `fn init_tracing`:

```rust
pub mod driver;
```

`pub`, not private: a private module makes every item in it dead code under `-D warnings` until Task 11 wires it up. Task 11 drops the `pub` again once `run()` uses them.

- [ ] **Step 6: Run the tests to verify they pass**

Run: `cargo test -p updraft_tauri`
Expected: PASS, 3 tests.

- [ ] **Step 7: Commit**

```bash
git add tauri
git commit -m "tauri: Add the driver owning the core and its subscribers"
```

---

### Task 10: TCP transport

**Files:**
- Create: `tauri/src/transport/mod.rs`
- Create: `tauri/src/transport/tcp.rs`
- Modify: `tauri/src/lib.rs`

**Interfaces:**
- Consumes: `DriverHandle` from task 9, `ConnectionId`, `ConnectionSpec`, `ConnectionState`, `Input` from the core
- Produces: `transport::open(connection: ConnectionId, spec: ConnectionSpec, handle: DriverHandle)`, which spawns a task that maintains the link and feeds `Input::Bytes` and `Input::ConnectionChanged`

The shell owns reconnection entirely. An open request means "maintain this link", so the task retries with backoff until the process ends.

- [ ] **Step 1: Write the failing test**

Create `tauri/src/transport/tcp.rs`:

```rust
use crate::driver::DriverHandle;
use std::time::Duration;
use tokio::io::AsyncReadExt as _;
use tokio::net::TcpStream;
use updraft_core::{ConnectionId, ConnectionState, Input};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::driver::Driver;
    use claims::assert_some;
    use tokio::io::AsyncWriteExt as _;
    use tokio::net::TcpListener;
    use tokio::sync::mpsc;
    use tokio::time::timeout;
    use updraft_core::{ConnectionSpec, CoreConfig, Topic};

    const PATIENCE: Duration = Duration::from_secs(5);

    #[tokio::test]
    async fn bytes_from_a_listening_peer_reach_the_core() {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("binds");
        let port = listener.local_addr().expect("has an address").port();
        let connection = ConnectionId(1);

        let handle = Driver::spawn(
            CoreConfig {
                connections: vec![(connection, ConnectionSpec::tcp("127.0.0.1", port))],
            },
            Box::new(|_, _, _| {}),
            Duration::from_millis(100),
        );

        let (sender, mut topics) = mpsc::unbounded_channel();
        handle.subscribe(Box::new(move |topic: &Topic| {
            sender.send(topic.clone()).is_ok()
        }));

        run(connection, "127.0.0.1".to_owned(), port, handle.clone());

        let (mut stream, _) = listener.accept().await.expect("accepts");
        stream
            .write_all(b"$GPRMC,120000.00,A,5049.38,N,00611.16,E,45.0,270.0,010126,,,A\r\n")
            .await
            .expect("writes");

        loop {
            let received = timeout(PATIENCE, topics.recv())
                .await
                .expect("a topic within the timeout");
            let Topic::Instruments(instruments) = assert_some!(received);
            if instruments.position.is_some() {
                return;
            }
        }
    }
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test -p updraft_tauri tcp`
Expected: FAIL, `cannot find function 'run' in this scope`.

- [ ] **Step 3: Implement the transport**

Insert into `tauri/src/transport/tcp.rs` between the `use` lines and the test module:

```rust
const INITIAL_BACKOFF: Duration = Duration::from_millis(250);
const MAX_BACKOFF: Duration = Duration::from_secs(10);
const READ_BUFFER_BYTES: usize = 4_096;

/// Maintains a TCP link until the process ends.
///
/// The core asked for this link to exist, so reconnection and backoff are
/// this task's business, not the core's. The core only learns the current
/// state through [`Input::ConnectionChanged`].
pub fn run(connection: ConnectionId, host: String, port: u16, handle: DriverHandle) {
    tokio::spawn(async move {
        let mut backoff = INITIAL_BACKOFF;

        loop {
            handle.send(Input::connection_changed(
                connection,
                ConnectionState::Connecting,
            ));

            match TcpStream::connect((host.as_str(), port)).await {
                Ok(stream) => {
                    handle.send(Input::connection_changed(
                        connection,
                        ConnectionState::Connected,
                    ));
                    // Reset only once the link has actually carried data. A
                    // peer that accepts and immediately drops would otherwise
                    // retry at the floor forever.
                    if pump(connection, stream, &handle).await {
                        backoff = INITIAL_BACKOFF;
                    }
                }
                Err(error) => {
                    tracing::warn!(%host, port, %error, "TCP connect failed");
                }
            }

            handle.send(Input::connection_changed(
                connection,
                ConnectionState::Disconnected,
            ));

            tokio::time::sleep(backoff).await;
            backoff = (backoff * 2).min(MAX_BACKOFF);
        }
    });
}

/// Reads until the link closes or errors. Returns whether any bytes
/// arrived, which is what tells the caller the connection was real.
async fn pump(connection: ConnectionId, mut stream: TcpStream, handle: &DriverHandle) -> bool {
    let mut buffer = vec![0_u8; READ_BUFFER_BYTES];
    let mut received = false;

    loop {
        match stream.read(&mut buffer).await {
            Ok(0) => return received,
            Ok(read) => {
                received = true;
                handle.send(Input::bytes(connection, &buffer[..read]));
            }
            Err(error) => {
                tracing::warn!(%error, "TCP read failed");
                return received;
            }
        }
    }
}
```

- [ ] **Step 4: Add the transport dispatch**

Create `tauri/src/transport/mod.rs`:

```rust
pub mod tcp;

use crate::driver::DriverHandle;
use updraft_core::{ConnectionId, ConnectionSpec};

/// Brings up the transport for one connection spec.
///
/// The core names a link and how to reach it. Which socket type that
/// implies, and everything about keeping it alive, stops here.
pub fn open(connection: ConnectionId, spec: ConnectionSpec, handle: DriverHandle) {
    match spec {
        ConnectionSpec::Tcp { host, port } => tcp::run(connection, host, port, handle),
    }
}
```

Add to `tauri/src/lib.rs` below `mod driver;`:

```rust
mod transport;
```

- [ ] **Step 5: Run the tests to verify they pass**

Run: `cargo test -p updraft_tauri`
Expected: PASS, 4 tests.

- [ ] **Step 6: Commit**

```bash
git add tauri
git commit -m "tauri: Add TCP transport with shell-owned reconnection"
```

---

### Task 11: The subscribe command

**Files:**

- Create: `tauri/src/ipc.rs`
- Modify: `tauri/src/lib.rs`

**Interfaces:**

- Consumes: `Driver`, `DriverHandle`, `transport::open`
- Produces: the `subscribe` Tauri command taking a `Channel<Topic>`, and a fully wired `run()`

All the subscriber bookkeeping lives in the driver, so this file only adapts a `tauri::ipc::Channel` into a `Sink` closure. No tests: it is three lines of adapter over code already covered in task 9, and exercising it would need a running Tauri app.

- [ ] **Step 1: Write the command**

Create `tauri/src/ipc.rs`:

```rust
use crate::driver::DriverHandle;
use tauri::ipc::Channel;
use updraft_core::Topic;

/// Registers the webview's channel as a subscriber.
///
/// `Channel::send` fails once the webview is gone, which is exactly the
/// signal the driver uses to prune the sink.
#[tauri::command]
pub fn subscribe(channel: Channel<Topic>, handle: tauri::State<'_, DriverHandle>) {
    handle.subscribe(Box::new(move |topic: &Topic| {
        channel.send(topic.clone()).is_ok()
    }));
}
```

- [ ] **Step 2: Wire everything together**

In `tauri/src/lib.rs`, add `mod ipc;` beside the other modules, drop the `pub` from `driver` and `transport` now that `run()` references them, and replace `pub fn run()`:

```rust
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![ipc::subscribe])
        .setup(|app| {
            if let Some(guard) = init_tracing(app.handle()) {
                app.manage(guard);
            }

            // Connections become runtime-mutable settings in milestone 5.
            let config = updraft_core::CoreConfig {
                connections: vec![(
                    updraft_core::ConnectionId(1),
                    updraft_core::ConnectionSpec::tcp("127.0.0.1", 4353),
                )],
            };

            // `setup` runs on the main thread outside any runtime context,
            // so `tokio::spawn` inside the driver would panic. Enter Tauri's
            // runtime for the call rather than making the driver depend on
            // Tauri to spawn itself.
            let handle = {
                let _runtime = tauri::async_runtime::handle();
                let _guard = _runtime.inner().enter();
                driver::Driver::spawn(
                    config,
                    Box::new(transport::open),
                    std::time::Duration::from_millis(100),
                )
            };

            handle.send(updraft_core::Input::Start);
            app.manage(handle);

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

The shell never opens a connection on its own initiative. `Input::Start` makes the core emit the open effects, and the driver routes them to `transport::open`, so the configured list is acted on through exactly one path.

- [ ] **Step 3: Run everything**

Run: `cargo test -p updraft_tauri && cargo clippy --workspace --all-targets -- -D warnings`
Expected: PASS, no warnings.

- [ ] **Step 4: Commit**

```bash
git add tauri
git commit -m "tauri: Add the \`subscribe\` command"
```

---

### Task 12: Frontend client, store and map wiring

**Files:**
- Create: `frontend/src/lib/client/index.ts`
- Create: `frontend/src/lib/client/tauri.ts`
- Create: `frontend/src/lib/client/fake.ts`
- Create: `frontend/src/lib/client/fake.test.ts`
- Create: `frontend/src/lib/stores/instruments.svelte.ts`
- Create: `frontend/src/lib/stores/instruments.test.ts`
- Delete: `frontend/src/lib/gnss.ts`
- Modify: `frontend/src/routes/+layout.svelte`
- Modify: `frontend/src/lib/flight-view/FlightView.svelte`
- Modify: `frontend/src/lib/map/Map.svelte`
- Modify: `frontend/src/lib/map/Ownship.svelte`
- Modify: `frontend/src/lib/map/ownship.ts`
- Modify: `frontend/src/lib/map/ownship.test.ts`
- Modify: `frontend/src/lib/map/Map.stories.svelte`
- Modify: `frontend/package.json`

**Interfaces:**
- Consumes: generated `Topic`, `Instruments`, `LatLon` from task 8; the `subscribe` command from task 11
- Produces: `UpdraftClient` with `subscribe(onTopic: (topic: Topic) => void): () => void`, `TauriClient`, `FakeClient` with `emit(topic: Topic)`, and `InstrumentsStore` with a reactive `current: Instruments`

The map components move from the old `GnssData` shape to `Instruments`. Components stay presentational: only `+layout.svelte` touches the client, which is also what keeps the subscription alive across navigation to future routes such as settings.

- [ ] **Step 1: Add the Tauri API dependency**

In `frontend/package.json`, under `dependencies`:

```json
"@tauri-apps/api": "2.11.1"
```

Run: `pnpm install`

- [ ] **Step 2: Write the failing tests**

Create `frontend/src/lib/client/fake.test.ts`:

```typescript
import { describe, expect, it, vi } from 'vitest';
import type { Topic } from '$lib/protocol/generated/Topic';
import { FakeClient } from './fake';

function instruments(trackDegrees: number): Topic {
  return {
    topic: 'instruments',
    value: {
      position: { latitudeDegrees: 50.823, longitudeDegrees: 6.186 },
      trackDegrees,
      groundSpeedMetersPerSecond: null,
      altitudeMslMeters: null,
    },
  };
}

describe('FakeClient', () => {
  it('delivers emitted topics to a subscriber', () => {
    let client = new FakeClient();
    let received: Topic[] = [];

    client.subscribe((topic) => received.push(topic));
    client.emit(instruments(270));

    expect(received).toEqual([instruments(270)]);
  });

  it('stops delivering after unsubscribe', () => {
    let client = new FakeClient();
    let onTopic = vi.fn();

    let unsubscribe = client.subscribe(onTopic);
    unsubscribe();
    client.emit(instruments(90));

    expect(onTopic).not.toHaveBeenCalled();
  });
});
```

Create `frontend/src/lib/stores/instruments.test.ts`:

```typescript
import { describe, expect, it } from 'vitest';
import { InstrumentsStore } from './instruments.svelte';

describe('InstrumentsStore', () => {
  it('replaces its value with the latest topic', () => {
    let store = new InstrumentsStore();

    store.apply({
      topic: 'instruments',
      value: {
        position: { latitudeDegrees: 50.823, longitudeDegrees: 6.186 },
        trackDegrees: 270,
        groundSpeedMetersPerSecond: 45,
        altitudeMslMeters: 200,
      },
    });

    expect(store.current.trackDegrees).toBe(270);
    expect(store.current.position).toEqual({
      latitudeDegrees: 50.823,
      longitudeDegrees: 6.186,
    });
  });
});
```

- [ ] **Step 3: Run the tests to verify they fail**

Run: `pnpm --filter @updraft/frontend test`
Expected: FAIL, cannot resolve `./fake` or `./instruments.svelte`.

- [ ] **Step 4: Implement the client**

Create `frontend/src/lib/client/index.ts`:

```typescript
import type { Topic } from '$lib/protocol/generated/Topic';

export type TopicListener = (topic: Topic) => void;

/**
 * The only boundary between the frontend and the Rust shell.
 *
 * Components never import an implementation of this. The layout receives
 * one, so tests and browser-only development can substitute the fake.
 */
export interface UpdraftClient {
  /**
   * Starts delivering topics to `onTopic`.
   *
   * The returned function stops local delivery. It does not tell the Rust
   * side to stop sending: the driver prunes a subscriber only when a send
   * to it fails, which happens when the webview goes away. That is enough
   * while the layout owns the only subscription and never unmounts.
   */
  subscribe(onTopic: TopicListener): () => void;
}
```

Create `frontend/src/lib/client/fake.ts`:

```typescript
import type { Topic } from '$lib/protocol/generated/Topic';
import type { TopicListener, UpdraftClient } from './index';

/** Drives the frontend without a Rust process behind it. */
export class FakeClient implements UpdraftClient {
  #listeners = new Set<TopicListener>();

  subscribe(onTopic: TopicListener): () => void {
    this.#listeners.add(onTopic);

    return () => {
      this.#listeners.delete(onTopic);
    };
  }

  /** Publishes a topic as though the core had emitted it. */
  emit(topic: Topic): void {
    for (let listener of this.#listeners) {
      listener(topic);
    }
  }
}
```

Create `frontend/src/lib/client/tauri.ts`:

```typescript
import { Channel, invoke } from '@tauri-apps/api/core';
import type { Topic } from '$lib/protocol/generated/Topic';
import type { TopicListener, UpdraftClient } from './index';

export class TauriClient implements UpdraftClient {
  subscribe(onTopic: TopicListener): () => void {
    let channel = new Channel<Topic>();
    channel.onmessage = onTopic;

    let closed = false;
    void invoke('subscribe', { channel }).catch((error: unknown) => {
      if (!closed) {
        console.error('Failed to subscribe to the state stream', error);
      }
    });

    return () => {
      closed = true;
      channel.onmessage = () => {};
    };
  }
}
```

- [ ] **Step 5: Implement the store**

Create `frontend/src/lib/stores/instruments.svelte.ts`:

```typescript
import type { Instruments } from '$lib/protocol/generated/Instruments';
import type { Topic } from '$lib/protocol/generated/Topic';

const EMPTY: Instruments = {
  position: null,
  trackDegrees: null,
  groundSpeedMetersPerSecond: null,
  altitudeMslMeters: null,
};

/**
 * Holds the latest instruments topic.
 *
 * Topics arrive whole, so the store replaces rather than merges and the
 * view is a pure function of the last message received.
 */
export class InstrumentsStore {
  current = $state.raw<Instruments>(EMPTY);

  apply(topic: Topic): void {
    if (topic.topic !== 'instruments') return;

    this.current = topic.value;
  }
}
```

The adjacent tag chosen in task 3 is what makes this a plain narrow-then-assign.

- [ ] **Step 6: Run the tests to verify they pass**

Run: `pnpm --filter @updraft/frontend test`
Expected: PASS.

- [ ] **Step 7: Move the map components onto `Instruments`**

Replace `frontend/src/lib/map/ownship.ts`:

```typescript
import type * as GeoJSON from 'geojson';
import type { LatLon } from '$lib/protocol/generated/LatLon';

export function positionCoordinates(position: LatLon): [number, number] {
  return [position.longitudeDegrees, position.latitudeDegrees];
}

/** Builds the GeoJSON point feature that positions the ownship symbol. */
export function ownshipFeature(
  position: LatLon,
  trackDegrees: number | null,
): GeoJSON.Feature<GeoJSON.Point> {
  return {
    type: 'Feature',
    geometry: {
      type: 'Point',
      coordinates: positionCoordinates(position),
    },
    properties: {
      track: trackDegrees ?? 0,
    },
  };
}
```

Replace `frontend/src/lib/map/ownship.test.ts`:

```typescript
import { describe, expect, it } from 'vitest';
import { ownshipFeature, positionCoordinates } from './ownship';

let position = { latitudeDegrees: 50.823, longitudeDegrees: 6.186 };

describe('ownship', () => {
  it('orders coordinates as longitude then latitude for GeoJSON', () => {
    expect(positionCoordinates(position)).toEqual([6.186, 50.823]);
  });

  it('carries the track into the feature properties', () => {
    expect(ownshipFeature(position, 270).properties).toEqual({ track: 270 });
  });

  it('falls back to zero when the track is unknown', () => {
    expect(ownshipFeature(position, null).properties).toEqual({ track: 0 });
  });
});
```

In `frontend/src/lib/map/Ownship.svelte`, replace the script block:

```svelte
<script lang="ts">
  import type { LatLon } from '$lib/protocol/generated/LatLon';

  import { GeoJSONSource, SymbolLayer } from 'svelte-maplibre-gl';

  import { ownshipFeature } from './ownship';

  let { position, trackDegrees }: { position: LatLon; trackDegrees: number | null } = $props();
</script>
```

and change the source data binding to `data={ownshipFeature(position, trackDegrees)}`.

In `frontend/src/lib/map/Map.svelte`, change the type import, the prop and the derived values:

```svelte
  import type { Instruments } from '$lib/protocol/generated/Instruments';
```

```svelte
  let { instruments, testMode = false }: { instruments: Instruments; testMode?: boolean } =
    $props();
```

```svelte
  const position = $derived(instruments.position);
  const center = $derived(position ? positionCoordinates(position) : DEFAULT_CENTER);
```

Update the import of `latLonCoordinates` to `positionCoordinates`, and the `Ownship` usage to `<Ownship {position} trackDegrees={instruments.trackDegrees} />`.

Apply the same prop rename in `frontend/src/lib/flight-view/FlightView.svelte` and `frontend/src/lib/map/Map.stories.svelte`.

- [ ] **Step 8: Wire the layout to the client**

The layout owns the subscription so it survives navigation to future routes. Replace the script block of `frontend/src/routes/+layout.svelte`:

```svelte
<script lang="ts">
  import '../app.css';
  import 'virtual:uno.css';

  import { onMount } from 'svelte';
  import { page } from '$app/state';

  import favicon from '$lib/assets/favicon.svg';
  import { FakeClient } from '$lib/client/fake';
  import { TauriClient } from '$lib/client/tauri';
  import FlightView from '$lib/flight-view/FlightView.svelte';
  import { getLocale } from '$lib/paraglide/runtime.js';
  import { InstrumentsStore } from '$lib/stores/instruments.svelte';

  type TestWindow = Window & { __updraftFake?: FakeClient };

  let { children } = $props();

  const instruments = new InstrumentsStore();
  const testMode = new URLSearchParams(window.location.search).get('testMode') === '1';

  onMount(() => {
    let inTauri = '__TAURI_INTERNALS__' in window;
    let client = inTauri ? new TauriClient() : new FakeClient();

    // Only in test mode: a plain web build should not hand every visitor a
    // handle for injecting instrument data.
    if (testMode && client instanceof FakeClient) {
      (window as TestWindow).__updraftFake = client;
    }

    return client.subscribe((topic) => instruments.apply(topic));
  });

  $effect(() => {
    document.documentElement.lang = getLocale();
  });
</script>
```

and change the markup's flight view to `<FlightView instruments={instruments.current} {testMode} />`.

Falling back to the fake outside Tauri is the browser-development path the client abstraction exists for: the whole UI runs with no Rust build, and it is what the e2e suite drives in task 13.

- [ ] **Step 9: Verify the whole frontend**

Run: `pnpm check && pnpm test && pnpm lint && pnpm build`
Expected: all pass.

- [ ] **Step 10: Commit**

```bash
git add frontend
git commit -m "frontend: Replace the state stream with the Tauri topic client"
```

---

### Task 13: End-to-end map test

**Files:**

- Create: `e2e/tests/map.spec.ts`
- Delete: `e2e/tests/.gitkeep`
- Modify: `e2e/package.json`

**Interfaces:**

- Consumes: `window.__updraftFake` from task 12, `window.__updraftTest.map` from `Map.svelte`'s existing test mode
- Produces: a Playwright test that runs against the built frontend with no Rust process

The map assertions here are lifted from the `position.spec.ts` deleted in task 1, which drove the same checks through the server's simulation endpoint. Recover the original from git if the details are needed. Only the input side changes: topics are emitted through the fake client instead of posted over HTTP.

- [ ] **Step 1: Write the test**

Create `e2e/tests/map.spec.ts`:

```typescript
import type { Page } from '@playwright/test';
import type { GeoJSONSource, Map as MapLibreMap } from 'maplibre-gl';

import { expect, test } from '@playwright/test';

type Instruments = {
  position: { latitudeDegrees: number; longitudeDegrees: number };
  trackDegrees: number;
  groundSpeedMetersPerSecond: number;
  altitudeMslMeters: number;
};

type MapState = {
  center: number[];
  renderedCoordinates: number[] | null;
  sourceCoordinates: number[];
};

type TestWindow = Window & {
  __updraftTest?: { map: MapLibreMap };
  __updraftFake?: { emit: (topic: unknown) => void };
};

const POSITION_A: Instruments = {
  position: { latitudeDegrees: 50.823, longitudeDegrees: 6.186 },
  trackDegrees: 45,
  groundSpeedMetersPerSecond: 30,
  altitudeMslMeters: 400,
};

const POSITION_B: Instruments = {
  position: { latitudeDegrees: 50.824, longitudeDegrees: 6.187 },
  trackDegrees: 90,
  groundSpeedMetersPerSecond: 31,
  altitudeMslMeters: 410,
};

test('renders the ownship position and follows live updates', async ({ page }) => {
  await page.goto('/?testMode=1');
  await page.waitForFunction(() => '__updraftFake' in window);

  await emitInstruments(page, POSITION_A);
  await expectMapPosition(page, POSITION_A);

  await emitInstruments(page, POSITION_B);
  await expectMapPosition(page, POSITION_B);
});

async function emitInstruments(page: Page, instruments: Instruments) {
  await page.evaluate((value) => {
    (window as TestWindow).__updraftFake?.emit({ topic: 'instruments', value });
  }, instruments);
}

async function expectMapPosition(page: Page, instruments: Instruments) {
  let { latitudeDegrees, longitudeDegrees } = instruments.position;

  await expect
    .poll(() => readMapState(page), {
      message: `map to render position ${latitudeDegrees}, ${longitudeDegrees}`,
    })
    .toEqual({
      center: [expect.closeTo(longitudeDegrees, 6), expect.closeTo(latitudeDegrees, 6)],
      renderedCoordinates: [expect.closeTo(longitudeDegrees, 4), expect.closeTo(latitudeDegrees, 4)],
      sourceCoordinates: [expect.closeTo(longitudeDegrees, 6), expect.closeTo(latitudeDegrees, 6)],
    });
}

async function readMapState(page: Page): Promise<MapState | null> {
  return page.evaluate(async () => {
    let map = (window as TestWindow).__updraftTest?.map;
    let source = map?.getSource<GeoJSONSource>('ownship');
    if (!map || !source) return null;

    let data = await source.getData();
    if (data.type !== 'Feature' || data.geometry?.type !== 'Point') return null;

    let center = map.getCenter();
    let renderedOwnship = map.queryRenderedFeatures({ layers: ['ownship-symbol'] })[0];

    return {
      center: [center.lng, center.lat],
      renderedCoordinates:
        renderedOwnship?.geometry.type === 'Point' ? renderedOwnship.geometry.coordinates : null,
      sourceCoordinates: data.geometry.coordinates,
    };
  });
}
```

Each assertion checks a different position, so the second one proves the update path rather than re-asserting the first.

- [ ] **Step 2: Remove the empty-suite scaffolding from task 1**

The suite has a real spec again, so an empty run should fail rather than pass silently.

```bash
git rm e2e/tests/.gitkeep
```

In `e2e/package.json`, change the `test` script back to `playwright test`.

- [ ] **Step 3: Run the test**

Run: `pnpm test:e2e`
Expected: PASS, 1 test. The `pretest` script builds the frontend first.

If it fails on the first `expectMapPosition`, the fake is not reaching the store. If it fails only on the second, the store is not reactive.

- [ ] **Step 4: Verify the whole repository**

Run: `cargo test --workspace --all-features && cargo clippy --workspace --all-targets --all-features -- -D warnings && pnpm check && pnpm test && pnpm lint`
Expected: all pass.

- [ ] **Step 5: Manual verification of the walking skeleton**

In one terminal:

```bash
while true; do cat testdata/nmea/basic.nmea; sleep 1; done | nc -l 4353
```

In another:

```bash
pnpm tauri:dev
```

Expected: the glider symbol appears on the map near 50.82 N, 6.19 E and its heading matches the fixture's track. This is the milestone's actual deliverable.

Do not skip this step. Nothing in the automated suite starts a real Tauri process, so an entire class of bug reaches here unchallenged. The first run of this milestone aborted at startup with "there is no reactor running": `setup` executes on the main thread outside any runtime context, and the driver's `tokio::spawn` panicked. The driver's own tests never saw it because `#[tokio::test]` always provides a context. Check the app's log under the OS log directory for `TCP connect failed` too, which is how a transport that never reaches the feeder shows up.

- [ ] **Step 6: Commit**

```bash
git add e2e
git commit -m "e2e: Drive the map test through the fake client"
```

---

## Milestone complete

At this point a position fix travels from a TCP socket through the NMEA decoder, into deterministic core state, out as a topic the moment a value changes, to every subscriber, into a Svelte store, and onto the map as the ownship symbol. Every layer below the map has tests, and the core has a snapshot harness that later milestones extend rather than replace.

Milestone 2 adds the Android mobile plugin: foreground service and platform location.
