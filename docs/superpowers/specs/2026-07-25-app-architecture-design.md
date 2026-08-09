# Application architecture

## Context

Updraft targets the feature set in [`docs/discovery/_features.md`](../../discovery/_features.md), roughly 200 items marked as targets, spanning moving map, glide computer, task engine, airspace, traffic, hardware I/O, weather, contest optimization and EFIS. No single design covers that. This document defines the top-level architecture only: module boundaries, data flow and the testability strategy. Each subsystem gets its own design and plan later.

Two goals drive every decision here:

1. Reach an MVP quickly.
2. Make regressions mechanically detectable rather than culturally avoided.

## Relationship to `docs/design/`

This document was produced from the discovery material alone, without reading the existing design docs or implementation. Where the two disagree, this document wins, and the conflicting parts of `docs/design/` are superseded and need updating: `server.md`, `tauri.md`, `multi-client.md`, and the transport sections of `README.md`, `core.md`, `frontend.md`, `runtime.md` and `testing.md`. `docs/roadmap.md` tracks implementation against the superseded design and is replaced by the delivery milestones below.

Both describe a functional core with an imperative shell, which is a general pattern rather than a specific agreement. The designs differ on nearly everything below that:

| `docs/design/` | This document |
| --- | --- |
| An embedded axum server on loopback is the single transport for state, commands and bulk data, in both standalone and Tauri hosting | No server. State and commands over Tauri IPC, bulk data over a custom URI scheme |
| `Change` and `ChangeGroup` carry client-visible state updates | Cadence-grouped topics, each message a whole snapshot of one topic |
| `Resource` is bulk data served by reference over HTTP | Bulk data is served over `updraft://`, addressed by role |
| The core returns `next_deadline` and the shell wakes it accordingly | The shell drives a fixed 10 Hz tick |
| Multi-client and multi-display are load-bearing | Both deferred |

Read-only queries and sending current state on subscribe appear in both designs.

MBTiles served through a custom URI scheme handler has already been validated in an earlier spike and works, including on a physical Android device. Its performance has not been measured, which is a later assessment rather than a blocker. If a strong need for an HTTP server appears, it can be extracted then.

## Fixed constraints

- Tauri shell, Rust business logic, Svelte frontend, MapLibre map rendering.
- The axum server and the multi-display goal are dropped for now. The architecture must not preclude their return.
- Android is the MVP target platform. macOS is the development platform.

## MVP scope

- Vector basemap. Starts with the hosted OpenFreeMap `positron` style. Offline MBTiles support is part of the MVP but sequenced as one of the last pieces.
- Airspace layer from a local OpenAir file. No download manager.
- Internal GPS as position source.
- FLARM targets on the map, over NMEA via Bluetooth SPP.
- TCP transport as the development equivalent of a Bluetooth device.
- Android foreground service to keep the app alive.
- A hardcoded set of data fields. No configurable layout system.
- Persistent settings: units, map orientation, locale, devices.

Out of MVP: IGC recording, glide computer, tasks, download managers, user-facing replay, configurable infoboxes.

### Delivery milestones

The MVP ships as six plans, each producing working, testable software.

| # | Milestone | Deliverable |
| --- | --- | --- |
| 1 | Core and shell rewrite | Walking skeleton on macOS: TCP transport to NMEA to core to topic to ownship on the map |
| 2 | Android platform | Foreground service, wake lock and internal GPS through the mobile plugin, plus the two Tauri-level fixes background execution needs. The app survives backgrounding, activity destruction and relaunch |
| 3 | Devices and traffic | Bluetooth SPP transport, FLARM targets rendered on the map |
| 4 | Airspace | OpenAir file parsed into core types, airspace layer served over `updraft://` |
| 5 | Settings and data fields | Persistent settings, units wired through, a hardcoded data field set |
| 6 | Offline basemap | MBTiles served over `updraft://localhost/basemap` |

The foreground service comes before Bluetooth deliberately. Android's background execution limits break a backgrounded SPP connection, so the service is a prerequisite for a device link that survives a flight rather than an independent feature.

Milestone 1 begins by deleting `updraft_core` and `updraft_runtime` outright. The rewrite is done with fresh eyes against this document, and the previous implementation is explicitly not a reference.

## Overview

The system is a functional core with an imperative shell.

The core is pure. It has no clock, no filesystem, no sockets and no Tauri dependency. It accepts inputs, updates state, returns effects, and answers read-only queries. Everything about it is deterministic, so a scripted sequence of inputs produces a byte-identical sequence of effects on every run and every platform.

The shell is the `tauri` crate. It owns everything with a syscall in it: transports, threads, timers, network, persistence, the URI scheme handler and the platform plugin. It executes effects and feeds the results back as inputs.

```
  shell (tauri crate)                          core (pure)

  Bluetooth SPP ┐
  TCP           ├─ transports ─┐
  platform GPS  ┘              │
                               ├─ inputs ──▶  apply(input, at) ─┐
  10 Hz timer ────── tick ─────┤                                │
                               │                                │
  Tauri commands ── UI ────────┘                                │
                                                                │
  effect executor  ◀─────────────── effects ────────────────────┘
        │
        ├─▶ emit topic ──▶ Tauri channel ──▶ frontend stores
        │        (current value of every topic on subscribe)
        ├─▶ open / close connection  ─┐
        ├─▶ run computation          ─┤
        ├─▶ fetch URL                ─┼─▶ results fed back as inputs
        └─▶ persist settings         ─┘

  Tauri command ── query ──▶ read-only against state ──▶ result returned directly

  URI scheme handler ──▶ basemap tiles, airspace GeoJSON   (pull, not effects)
```

## Crate layout

The existing leaf crates stay as they are. They already encode the discipline this design depends on: `updraft_units` treats display formatting as a UI concern, `updraft_geo` leaves wire parsing to each format, and `updraft_nmea::parse` is a pure function over a caller-owned byte buffer that never owns a connection.

| Crate | Role |
| --- | --- |
| `updraft_units` | SI quantity newtypes |
| `updraft_geo` | Coordinates, bounding boxes, geodesy |
| `updraft_egm96` | Geoid undulation |
| `updraft_nmea` | NMEA framing and sentence decoding |
| `updraft_polar` | Glide polar model and polar store |
| `updraft_sprites` | Build-time MapLibre SDF spritesheet generation |
| `openair` (crates.io) | OpenAir airspace file parsing |
| `updraft_core` | The functional core |
| `tauri` | The imperative shell |
| Tauri mobile plugin | Android Kotlin glue |

The core is rearchitected. `updraft_runtime` already holds roughly the role the shell has here, and is removed and replaced by the shell. The `server` crate drops out. The mobile plugin is new and stays a single crate covering all of the app's platform needs.

The shell lives in the `tauri` crate rather than a separate lib. Most of it is Tauri-coupled anyway, and one crate is simpler. The parts that are not coupled, MBTiles reading, the TCP transport and the computation worker pool, would test faster outside the Tauri build, so that is the signal to extract a shell lib if it ever becomes painful.

New crates get extracted when a boundary proves it wants to exist, not in anticipation.

## The core

### Inputs

The core accepts a small enum of input kinds. These are deliberately not flattened into a single record type, because the kinds differ in trust, semantics and the information the domain needs from them.

- **Bytes from a connection**, tagged with the connection identity. The core owns a buffer per connection and drives `updraft_nmea::parse` over it, handling framing and resynchronization. A connection can emit any sentence family: fixes, traffic, air data. Which connection produced a value is exactly what position-source arbitration and failover will need.
- **Structured location** from the platform GPS. Different trust level, different failure modes, and it never arrives as text.
- **Structured simulator data**, once user-controlled replay and demo mode exist. A simulator produces a synthetic whole picture rather than a stream of observations.
- **Computation results**, tagged with the generation they were computed from.
- **Connection state transitions** reported by the shell.
- **Commands** from the UI.
- **Tick**, carrying the current timestamp.

### State and `apply`

State is a plain struct with one entry point:

```rust
fn apply(&mut self, input: Input, at: Timestamp) -> Vec<Effect>
```

Timestamps are always passed in, never read. That is what makes the core clockless and its tests deterministic.

### Queries

Some questions cannot be answered from pushed topics, because they are about bulk data the frontend does not hold. Tapping the map to ask "what is here" is the motivating case: it is a spatial lookup against airspace, waypoints and traffic, and the answer depends on where the user tapped.

A query is a read-only request against current state. It returns a result directly, mutates nothing, produces no effects and is never recorded. Being a pure read over state, it is as easy to test as any other pure function.

Queries are not a general read channel, and the boundary matters. Anything the UI needs continuously belongs in a topic. Queries answer one-off questions about data too large or too situational to push. "What airspace is under this point" is a query. "What is the current altitude" is not.

### Effects

An effect is anything that crosses the process boundary:

- Emit a topic to the frontend.
- Open or close a connection.
- Run a computation.
- Fetch a URL.
- Persist settings.

The shell has one executor that matches exhaustively on effects, so a newly added effect cannot be silently forgotten.

Effects exist only for I/O. Pure derivation stays inline inside `apply`. Adding an effect for something that does not touch the outside world is the smell that this pattern is being over-applied.

### Policy versus mechanism

The core decides *what should happen*. The shell decides *how to make it happen*.

**Connections.** The core already owns device settings as the single source of truth, and connection state has to reach the UI, so the core necessarily knows both what should be connected and what is connected. Letting the shell independently decide what to connect would create two authorities over one fact. So the core emits open and close effects.

An open effect means "maintain this connection", not "make one attempt". The shell owns everything mechanical about that: sockets, Bluetooth pairing, Android quirks, reconnection and backoff. It reports state transitions back as inputs. The core has no stake in the backoff curve, only in whether a link is currently up, which is what source arbitration and status display need.

**Computations.** Long-running work such as glide-range footprints and contest optimization is a pure function in the core with no knowledge of threads. Whether it is stale enough to recompute is domain logic, so the core decides when to request it and tags each request with a generation. The shell runs it off the update path.

Scheduling is run-to-completion with a conflated pending slot. While a computation is in flight, new inputs overwrite a single pending value rather than queuing or restarting. Restarting on every input starves: inputs arrive at roughly 1 Hz and computations can take longer than that. On completion, if something is pending, the next run starts immediately with the newest value. Results carry their input generation, so the core accepts or discards them as stale and consumers know the age of what they are looking at.

Cancellation stays available, but only for shutdown and for categorical input changes such as a new aircraft profile, where in-flight work is genuinely invalidated rather than merely stale.

### Tick

The shell feeds a tick input at 10 Hz. This is what lets a clockless core express time-based policy deterministically: reconnect backoff, stale-fix detection, warning timeouts. Tests feed synthetic ticks and get exact behaviour with no sleeping and no flakiness. 10 Hz is also the natural upper bound for the instruments topic, which suits a vario.

## The shell

Two Tauri-level fixes belong to the shell rather than the plugin. Emulator
spikes identified both causes. The
[Android platform verification](../../verification/2026-07-26-android-platform.md)
records the measured results.

- **`prevent_exit()`.** Stock Tauri exits the process when the last window closes, because tao's Android event loop calls `std::process::exit`. When the activity is destroyed the process dies about two seconds after `onTaskRemoved` and takes the foreground service with it. Handling `RunEvent::ExitRequested` with `api.prevent_exit()` is not optional for an app that must keep recording after the user leaves it.
- **Webview re-creation on relaunch.** After activity destruction, the service
  keeps the process alive but no webview remains. `tauri-runtime-wry` drops the
  mobile `Resumed` event when no window exists. The shell watches the Android
  activity lifecycle and rebuilds the window through `run_on_main_thread`.
  It repeats the offer because tao can lose the first event-loop wake. The
  upstream reports are [tauri#15671](https://github.com/tauri-apps/tauri/issues/15671)
  and [tauri#15678](https://github.com/tauri-apps/tauri/pull/15678).

The `tauri` crate holds:

- The effect executor.
- Transports: Bluetooth SPP, TCP, platform location.
- Computation scheduling and worker threads.
- MBTiles reading.
- Settings persistence.
- The `updraft://` URI scheme handler.
- Tauri commands and the update channel.
- `prevent_exit()` and webview re-creation, per the two fixes above.

### Mobile plugin

Bluetooth SPP, the foreground service and the Android location provider all need Kotlin, so they live in a single Tauri v2 mobile plugin rather than plain Rust. This is the part of the MVP least testable on macOS and most likely to be slow, so the seam is drawn such that the Rust side never knows which platform it is talking to.

A two-plugin split was considered and rejected for now: one for background execution and internal GPS, one for device byte streams. The platform matrices do diverge, iOS has no SPP at all, and the two would change at different rates. But the boundary cannot be drawn well until milestone 3 shows how the transport code actually wants to be shaped, and the coupling runs the wrong way for a clean split: the foreground-service type mask depends on whether a device is connected, while the service itself would live in the other plugin.

Splitting later is a contained rename of the crate, the Android package and the plugin identifier in capabilities. The trigger to do it: if milestone 3's transport code ends up sharing nothing with the session code beyond the type mask.

The control plane is `run_mobile_plugin` with JSON. The data plane is a `tauri::ipc::Channel` handed to Kotlin, surfaced on the Rust side as a receiver. Fixes arrive as typed values, never as synthesised NMEA.

The MVP covers Bluetooth Classic SPP only. Modern FLARM devices do also offer BLE, and that follows later.

Verification splits three ways. The NMEA data path is fully exercisable on macOS through the TCP transport, which is why that transport exists. The Android emulator covers foreground service behaviour, permissions and lifecycle, interactively rather than in CI, where emulator startup is too slow to be worth it. Bluetooth SPP against a real instrument needs a physical device and stays a manual check.

Two Android lifecycle limits remain:

- Production code does not call `stop_session()`. The location subscription
  and partial wake lock remain active until the process stops.
- Tauri 2.11.5 retains the first activity and its activity result launchers.
  Updraft requests its current permissions before activity destruction. Future
  permission prompts after a relaunch must not use the stale activity.

## The boundary

### Rust to frontend

One Tauri channel carrying a tagged union of topics. Each message is a complete snapshot of that one topic, not a delta, emitted only when that topic changed.

The topic set is open and adjusted as needs emerge. Illustrative starting point: instruments (up to 10 Hz), traffic (around 1 Hz), derived values (sub-Hz), config (on change).

The governing rule is **push small and hot, pull big and cold**. Grouping by update cadence avoids paying full serialization cost for a vario update, without the complexity of per-field deltas. One channel rather than one per topic gives a single subscription point, preserved ordering and one place to fake.

Separate topics also map onto separate frontend stores, so a vario tick does not invalidate components reading only settings.

**On subscribe, the current value of every topic is emitted.** Topics are otherwise sent only on change, so a client connecting between changes would see nothing until the next update, and settings might never change at all. This is not only a multi-client concern: the webview is recreated on reload during development and can be recreated by Android after the process is backgrounded, and it has to resync.

Onboarding reuses the ordinary topic messages rather than introducing a snapshot message type, so the frontend stays a pure function of the messages it has received and needs no special first-message handling.

### Frontend to Rust

Tauri commands, deliberately coarse. Each either maps to a command input into the core, so user actions enter through the same path as sensor data, or runs a query and returns its result directly.

### Bulk data

Bulk data is served, never pushed:

- `updraft://localhost/basemap/{z}/{x}/{y}.pbf` from MBTiles.
- `updraft://localhost/airspace.geojson` projected on demand from core state.
- Sprites are static files generated by `updraft_sprites`.

MapLibre fetches these directly, so there is no IPC serialization and MapLibre parses GeoJSON in its own worker. When the underlying data changes, a generation counter in the config topic changes and the frontend re-points the source.

The resource paths are namespaced by role rather than by mechanism, because terrain and weather overlays are also tiles. `updraft://localhost/basemap/`, `updraft://localhost/terrain/` and `updraft://localhost/airspace.geojson` stay flat and self-describing where `updraft://localhost/tiles/` would immediately need a sub-namespace.

Airspace is parsed once into core domain types, which the core needs anyway for proximity warnings. The GeoJSON served to MapLibre is a projection of those types. One parse, one source of truth, two consumers.

### Type generation

TypeScript types for the boundary are generated from the Rust types with `ts-rs`. Hand-written mirrors drift, and a drifted boundary is exactly the silent regression class this architecture exists to eliminate.

Generation follows the golden file pattern rather than running at build time. The generated TypeScript is committed, and a test asserts that the committed files match what generation currently produces. That keeps the build simple, makes the types readable and diffable in review, and turns drift into a failing test instead of something that silently compiles.

This mechanism already exists in `server/src/wire/bindings.rs`, including the up-to-date check. It moves rather than being rebuilt when the server is removed.

Branding the numeric fields (`Meters`, `MetersPerSecond`) is nearly free once types are generated, and prevents formatting a speed as an altitude.

### Frontend and backend are decoupled by a client module

One frontend module owns every `invoke` call and the channel subscription, exposing a typed interface. Tests and browser-only development swap in a hand-written fake.

MSW is not used for this. MSW hooks `fetch`, XHR and WebSockets, but Tauri v2 IPC is `postMessage` on desktop and a custom-protocol request on Android, so it is not reliably interceptable. MSW earns its keep when intercepting code you do not own, and here both sides of the boundary are ours. A hand-written fake is smaller, synchronous and type-checked against the same interface the real client implements. It also lets most UI work happen in a plain browser with no Tauri build.

## Frontend

Under `lib/`:

| Module | Responsibility |
| --- | --- |
| `client/` | Real and fake implementations behind one interface |
| `stores/` | One store per pushed topic |
| `map/` | MapLibre integration and its pure derivations |
| `components/` | Data fields, panels, dialogs |
| `units.ts` | Unit conversion only. Already exists, stays a single file |

Two conventions carry most of the testability:

- **Container components read stores, leaf components take props.** Strict prop-drilling everywhere buys little, but keeping every data field, panel and dialog purely presentational means they test with plain inputs and no fake at all. Only a handful of containers touch stores.
- **Leaf components never import the client.** Commands are dispatched from containers.

### Units

The existing `frontend/src/lib/units.ts` is reused unchanged. Its responsibility is conversion between units, taking an SI number and returning a number. It does not produce display strings.

Rendering a value with its unit label and styling is the infobox's job.

### Map

The style is assembled from the hosted `positron` basemap initially, moving to `updraft://localhost/basemap/` when offline MBTiles lands, plus the existing SDF spritesheet for icons.

Airspace is a GeoJSON source fetched over `updraft://` with data-driven styling by class. Traffic is a GeoJSON source updated from the traffic store, using SDF sprites recoloured by threat level. One rendering mechanism for both, no DOM markers.

Decisions are factored out as pure functions where practical: traffic symbology by threat level, airspace class filtering, GeoJSON projection, camera derivation. Those test without a map instance. Introspecting the MapLibre instance is a legitimate test technique where it is the clearer option, for example bearing, layer visibility and source contents. Imperative-only application of declarative inputs is not prescribed as a rule, it is judged per case.

Camera derivation details are left open until that work is planned.

### Data fields

A hardcoded set of presentational components in a fixed layout for the MVP. The configurable grid, pages and per-flight-mode layouts come later.

## Testing strategy

### Core scenario tests

The centrepiece. Drive `apply` with a scripted input sequence, fixture bytes from `testdata/`, synthetic ticks and commands, then collect the resulting effect stream and snapshot it.

Because the core has no clock, no threads and no I/O, this is byte-identical on every run and every platform. And because the effect stream contains the emitted topic payloads, the snapshot is literally what the UI would have seen over time. Replay a recorded flight, get a timeline of UI-visible state, and any behavioural change shows as a diff.

**Floats in snapshots** are handled by serializing at fixed, quantity-appropriate precision rather than full `f64`: altitude to 0.1 m, angles to 0.01 degrees, speeds to 0.01 m/s. Real behavioural changes still show. Last-bit differences from FMA contraction or a different optimisation level do not. Assertions on individual computed values use `approx`, which the existing crates already support.

### Everything else

- **Leaf crates** keep their own tests.
- **Pure core functions** (computations, projections, source arbitration) get ordinary unit tests.
- **Queries** are tested by building a state, running the query and asserting the result. Spatial lookups get fixture-based cases, including boundaries and empty results.
- **The effect executor** is thin enough for a few targeted tests: MBTiles reads, URI scheme routing, transport reconnection.
- **`units.ts` and pure map functions** are plain unit tests.
- **Presentational components** test with plain props and no fake.
- **Containers and stores** test by feeding the fake client a scripted sequence of updates and asserting on rendered output.
- **Playwright** smoke tests in the existing `e2e/` directory, treated as nice-to-have rather than load-bearing.

### Shared fixtures

Worth attempting, but not worth fighting for: exporting Rust scenario snapshots as JSON and replaying them through the frontend's fake client, so both sides share one definition of what the boundary looks like. If it turns out harder than expected, drop it.

## Deferred

These are not in the MVP, and the architecture must not preclude them.

| Deferred | How it fits |
| --- | --- |
| User-controlled replay and demo mode | Its own input kind, with UI commands for play, pause, speed and seek |
| BLE transport | Another transport in the shell. The core sees a connection either way |
| Glide computer, task engine | Pure computations in the core |
| OGN and other network sources | An input kind plus a fetch effect. Network I/O never enters the core |
| IGC recording | An effect |
| Download managers | Effects plus a bulk-data generation bump |
| Configurable infobox system | Frontend only, additive over the hardcoded MVP layout |
| axum server, multi-display | The core already has no opinion on how many consumers it has |

Where the settings UI is concerned, the core's uniform treatment of sources must not leak into the interface. A network source needs connectivity policy that a serial link does not, so device settings are organised by the user's mental model rather than by the internal source abstraction.

## Open questions

- Custom URI scheme performance under sustained tile load. Measured later, not a blocker.
- Battery cost of a fixed 10 Hz tick over a multi-hour flight. If it measures badly, the core can return its next deadline instead. Not designed around now.
