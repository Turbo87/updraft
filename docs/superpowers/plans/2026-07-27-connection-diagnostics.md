# Connection Diagnostics Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans to implement this plan task by task. Track progress with the checkboxes below.

**Goal:** Add transport-neutral connection lifecycle and delivered-byte diagnostics to `updraft_core`, while keeping concrete TCP failures in the transport adapter.

**Architecture:** A private core helper tracks one attempt per configured `ConnectionId` and emits structured `tracing` events from inputs that already cross the core boundary. The TCP adapter continues to own socket errors and retry policy.

**Tech Stack:** Rust 2024, `tracing` 0.1.44, `tracing-test` 0.2.6, Tokio.

This is PR 1 from the [Android SPP design](../specs/2026-07-27-android-spp-design.md).

## Constraints

- This PR contains no Bluetooth code or retry refactor.
- The core logs lifecycle events, endpoint identifiers, and byte counts. It never logs payload content.
- Transport adapters log failures whose concrete cause is unavailable to the core.
- Count bytes delivered to a known core connection, not bytes read from a socket.
- Diagnostic state must not change domain state or returned effects.
- Do not add compatibility aliases, payload sampling, periodic progress logs, or a generic transport abstraction.
- Dependencies remain pinned with `=`.
- If production changes approach 400 lines, stop and ask before expanding the PR.

## Task 1: Core lifecycle diagnostics and TCP failure context

**Files:**

- Create: `libs/updraft_core/src/connection_diagnostics.rs`
- Modify: `libs/updraft_core/src/core.rs`
- Modify: `libs/updraft_core/src/lib.rs`
- Modify: `libs/updraft_core/Cargo.toml`
- Modify: `tauri/src/transport/tcp.rs`
- Modify: `Cargo.lock`

**Private interface:**

```rust
ConnectionDiagnostics::default()
ConnectionDiagnostics::insert(ConnectionId, ConnectionSpec)
ConnectionDiagnostics::remove(ConnectionId)
ConnectionDiagnostics::changed(ConnectionId, ConnectionState)
ConnectionDiagnostics::bytes(ConnectionId, usize)
```

Each structured event includes `connection` and `endpoint`. A connected disconnect also includes `delivered_bytes`.

- [ ] **Step 1: Add failing lifecycle tests**

Add `tracing-test = "=0.2.6"` to the core dev dependencies. In the existing `core.rs` test module, add:

- `connection_lifecycle_reports_endpoint_and_delivered_bytes`
  - Apply `Connecting`, `Connected`, byte chunks of lengths 3 and 2, then `Disconnected`.
  - Assert one event per lifecycle transition.
  - Assert levels are debug, info, info, and info.
  - Assert the first-byte event occurs once.
  - Assert the endpoint and connection ID appear on the same event line.
  - Assert the disconnect reports `delivered_bytes=5`.
- `failed_attempt_is_debug_and_counters_reset_on_reconnect`
  - Disconnect once before connecting, then complete two attempts with 3 and 2 bytes.
  - Assert the first disconnect is debug.
  - Assert later disconnect totals are 3 and 2.
- `unknown_and_empty_bytes_produce_no_delivery_log`
  - Apply bytes for an unknown ID and an empty chunk for a known ID.
  - Assert neither produces a first-byte event.
- `removed_connection_produces_no_further_diagnostics`
  - Insert a connection, record a connected attempt with bytes, then remove it.
  - Assert later lifecycle and byte inputs for that ID are ignored.

Keep the existing TCP-only test configuration. A small test helper may find a line by message, but all field assertions for an event must inspect that same line.

- [ ] **Step 2: Confirm the expected red state**

```bash
cargo test -p updraft_core connection_lifecycle_reports_endpoint_and_delivered_bytes
```

Expected: the test fails because lifecycle events do not exist. Fix test harness problems until that is the failure.

- [ ] **Step 3: Implement the private diagnostics helper**

Add `tracing = "=0.1.44"` to the core dependencies and declare the module privately. Derive `Default` for `ConnectionDiagnostics`.

Track this state for each configured connection:

```rust
struct Attempt {
    connected: bool,
    first_bytes_reported: bool,
    delivered_bytes: usize,
}
```

Keep the cloned `ConnectionSpec` beside the attempt so every event can render the endpoint.

`insert(id, spec)` adds or replaces that connection and starts it with default attempt state. This supports the later runtime-settings path without adding its core input in this slice.

`remove(id)` drops the endpoint and its attempt state. The current core has no removal input, so mark only this method with `#[cfg_attr(not(test), expect(dead_code))]`. Remove the expectation when that input lands instead of manufacturing a call in this slice.

Implement these transitions:

- `Connecting`: reset attempt state and emit debug `Connecting`.
- `Connected`: reset attempt state with `connected = true` and emit info `Connected`.
- First non-empty bytes for a known connection: emit info `First bytes` once, then count every non-empty chunk.
- `Disconnected` after `Connected`: emit info `Disconnected` with the total, then reset.
- `Disconnected` before `Connected`: emit debug `Disconnected`, then reset.
- Unknown connection IDs and empty chunks: return without logging or mutation.

In `Core::new()`, start with empty diagnostics and insert a cloned spec for each configured connection. In `Core::apply()`, record known byte lengths before decoding and record every `ConnectionChanged` input. Preserve all existing effects.

- [ ] **Step 4: Confirm core tests are green**

```bash
cargo test -p updraft_core --all-features
```

- [ ] **Step 5: Add endpoint context to TCP failure warnings**

Include `connection`, `host`, `port`, and `error` in both `TCP connect failed` and `TCP read failed`. Pass the endpoint into `pump()` if needed. Keep retry, socket handling, and byte counting out of the core.

- [ ] **Step 6: Verify the task**

```bash
cargo fmt --all --check
cargo test -p updraft_core --all-features
cargo clippy -p updraft_core --all-targets --all-features -- -D warnings
cargo test -p updraft_tauri --all-features
cargo clippy -p updraft_tauri --all-targets --all-features -- -D warnings
cargo doc -p updraft_core --no-deps --all-features
```

Every command must exit 0 without unexpected warnings or log output.

- [ ] **Step 7: Deslop and self-review the task**

```bash
git diff --check
git diff -- libs/updraft_core tauri/src/transport/tcp.rs Cargo.lock
rg -n "TODO|TBD|FIXME|previously|removed|legacy|just in case" libs/updraft_core tauri/src/transport/tcp.rs
```

Remove redundant comments, duplicate state, speculative helpers, and validation already guaranteed by configured IDs. Recheck event levels, fields, byte totals, reset behavior, unknown IDs, empty chunks, payload exclusion, and unchanged effects. Repeat Step 6 after any change.

- [ ] **Step 8: Commit the reviewed task**

```bash
git add Cargo.lock libs/updraft_core/Cargo.toml libs/updraft_core/src/connection_diagnostics.rs libs/updraft_core/src/core.rs libs/updraft_core/src/lib.rs tauri/src/transport/tcp.rs
git commit -m "core: Trace connection lifecycle"
```

## PR 1 Review Gate

Do not push or create a pull request.

```bash
git diff --check origin/main...HEAD
git diff --stat origin/main...HEAD
git diff origin/main...HEAD
git log --oneline origin/main..HEAD
```

Repeat Step 6. Present the complete diff, verification results, and deslop findings to the user. Wait for explicit approval before pushing or opening PR 1. Start the SPP branch only after this PR is merged, unless the user explicitly approves a stacked branch.
