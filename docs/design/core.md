# The Rust Core

The core is a plain Rust library. It owns authoritative flight state, which means the shared state that the app trusts. It also owns the decisions based on that state. The core performs **no I/O, spawns no threads, and reads no clocks**.

A shared [runtime](runtime.md) owns clocks, queues, workers, subscriptions, resources, and effect execution. Hosts add transport bindings and platform adapters.

All Rust domain code runs in one process and one `App`. The code is split into a few large domain modules, but these modules call each other directly. The module boundaries help people find code. They may change as the product grows.

## Mental Model

The architecture has five concepts:

- **Input:** a recorded event or request that may change authoritative state, such as a command, sensor observation, clock advancement, or completed effect.
- **Query:** a read-only request against current state. Queries are not inputs and are not recorded.
- **Change:** a client-visible state update produced after handling an input.
- **Effect:** a request for work outside the core, such as audio, file I/O, network I/O, device output, or expensive computation.
- **Resource:** bulk or growing data served by reference instead of copied through the state stream.

```text
clients and adapters
        │ Input
        ▼
   App::handle() ─────► Change ─────► subscribers
        │
        ├─────────────► Effect ─────► runtime adapter or worker
        │                                  │
        └──── next deadline                └──── result returns as Input

Resource references travel as Changes. Resource bytes travel over HTTP.
```

For a normal domain feature, a contributor should need only this model, the `App` interface, and the relevant domain module. Queue rules, worker lifecycle, delivery to subscribers, and resource storage are runtime details described in [runtime.md](runtime.md). Recording formats and replay behavior live in [replay.md](replay.md).

## The Application Interface

```rust
impl App {
    /// Apply one input; the only mutation entry point.
    pub fn handle(&mut self, input: Input) -> Update;
    /// Read-only queries against current state.
    pub fn query<Q: Query>(&self, query: Q) -> Q::Output;
    /// Shared current state for a newly subscribing client.
    pub fn snapshot(&self) -> Snapshot;
}

pub trait Query {
    type Output;

    fn execute(self, app: &App) -> Self::Output;
}

pub struct Update {
    pub changes: Vec<Change>,
    pub effects: Vec<Effect>,
    pub next_deadline: Option<Duration>,
}
```

Inputs, changes, and effects are enums grouped by domain. The core crate never depends on tokio, rayon, or an I/O library. Whole-flight scenario tests are a plain loop over `handle()` with no async runtime, sleeps, or wall clock.

Queries run on the core loop and must finish quickly. They do not perform I/O or return unbounded data. Bulk data uses resources, and expensive calculations use workers.

## State Ownership

State falls into four categories:

| Category | Owner | Distribution |
| --- | --- | --- |
| Authoritative domain state | `App` and its domain modules | Shared snapshot and changes |
| Saved display configuration | Rust-side storage, scoped by display profile | Scoped requests, not the shared flight snapshot |
| Temporary presentation state | Each frontend client | Never shared or saved by the core |
| Bulk and reference data | Core dataset handles plus runtime resource storage | Versioned references and HTTP resources |

Authoritative domain state includes the current fix, selected sensor values, traffic, the active task, computed flight values, settings that affect flight behavior, and device connection state. It is mutated only through `handle()` and has no shared mutable state across threads.

The `App` contains large domain modules such as flight, navigation, traffic, devices, and settings. Each domain owns its state, inputs, update logic, and changes. Concepts with clear states, such as flight mode or warning status, use small state machines instead of several related booleans. Domains call each other directly inside the process.

Saved display configuration includes layouts, data-field pages, and per-display preferences. It is stored on the Rust side because the embedded server may use a different web origin after each start. The configuration is keyed by display profile and does not become shared flight state. A secondary display can therefore use a different layout without changing the pilot's display.

Temporary presentation state includes the map viewport, open dialogs, unfinished editor changes, and animation state. Some temporary state may affect an external service. For example, the primary viewport selects the OGN area of interest. The client sends this as a specific temporary request. The app does not share it as presentation state with other clients.

Reference data is not copied into the client state stream. The core may hold a read-only dataset handle for airspace, waypoints, or terrain because queries and calculations need it. Clients receive only the active identity or version. They fetch the bytes through the [bulk resource path](#the-bulk-geodata-path).

## Inputs

Inputs cover four sources:

- user commands from a host transport,
- normalized sensor observations carrying their source and timestamps,
- monotonic clock advancement,
- completed effect and computation results.

Device connections and platform sensors normalize data before it enters the core. The exact queue and backpressure contract belongs to the [runtime](runtime.md#input-queue).

## Time Is an Input

The core never reads a clock. Adapters stamp observations with monotonic timestamps from their runtime's clock. GPS time stays in flight data and IGC records. Monotonic time is used only for scheduling, delay rules, freshness, and lookahead.

Timers are authoritative state. `Update.next_deadline` tells the runtime when to deliver the next clock input, so tests and replay use the same scheduling logic as production. Detailed timer ownership lives in [runtime.md](runtime.md#time-and-timers).

## Computation

Derived values are synchronous by default. Each calculation runs as often as its cost allows:

- every fix for ground speed, track, altitude fusion, and AGL,
- every vario update for speed to fly,
- roughly once per second for proximity, nearest-waypoint, and final-glide calculations,
- asynchronous workers for known expensive work such as live scoring and glide reach.

Warnings stay synchronous so warning generation cannot wait behind background work. The working end-to-end budget from warning input to audible alert is under 100 ms.

Expensive work starts through `Effect::Compute`. A small core-side job slot tracks whether work is pending, whether a job is running, and which results are still valid. The runtime executes the job, keeps any worker cache, and reports success or failure. These details are described in [runtime.md](runtime.md#compute-workers). Each domain decides whether an older result is still valid. An older score may be safe to show, while a safety-related calculation may need to reject it.

## Effects

Effects keep I/O and other outside work out of the core. The core still decides when that work is needed. Example effects include warning audio, log and snapshot writes, network requests, device output, compute jobs, and resource publication.

Effects that need a result return a typed input to the core. Effects that only need to run, such as warning audio, do not use the client state stream. The [runtime](runtime.md#effects) executes effects and handles failures.

## Outputs

Clients receive one state stream. A subscription starts with a small shared `Snapshot`, followed by FIFO-ordered batches of `Change` values.

Two rules define the contract:

- **Atomic subscribe:** registration and snapshot capture happen together, so no change can fall between them.
- **Reconnect is resubscribe:** a reconnect starts from a fresh snapshot without a replay buffer or sequence bookkeeping.

The snapshot carries current shared values and active sets only. Datasets, histories, time series, and display-specific configuration use queries or resources. This keeps reconnect cheap for the entire flight.

Change payloads use four shapes:

- **Last value:** position, wind, MacCready, final glide, and vario.
- **Keyed collection:** traffic and devices.
- **Events plus active set:** warnings whose transitions drive UI and audio behavior.
- **Reference:** a version or resource identifier for reach, track, and similar bulk data.

Moving objects are published as a **kinematic state vector**: position, track, ground speed, turn rate, climb rate, and a timestamp. Clients use it to estimate the current render position, so frame-rate animation never crosses the transport. State delivery, slow-client handling, and high-rate groups are described in [runtime.md](runtime.md#state-stream-delivery).

## The Message Protocol

Hosts expose one request/response mechanism plus the state stream. Commands mutate state and therefore become recorded inputs. Queries are read-only and never enter the input log. Every request returns a result or a typed error.

The contract is the same for every host. Production uses the axum server's HTTP and SSE transport, both standalone and embedded in Tauri. Requests and stream changes use JSON. Rust protocol types generate committed TypeScript bindings through `ts-rs`. CI checks that the generated files are current.

## The Bulk Geodata Path

Bulk and growing data never crosses the message channel. Map tiles, terrain, airspace geometry, reach polygons, tracks, and weather overlays are served by HTTP routes. State changes carry only versioned resource references.

Computed resource bytes live in a runtime-owned resource store. The runtime stores the bytes before it announces their version. Static datasets may be streamed from storage directly. The own track sends only new segments after the first load instead of sending the whole flight again. These rules are described in [runtime.md](runtime.md#resource-storage).

## Determinism & Replay

The same ordered inputs produce the same core state changes. A recording includes outside results and worker results. This makes a field failure repeatable without requiring every platform to produce identical floating-point results.

The recording format, companion file for worker results, CI recompute mode, and user-facing IGC replay are separate concerns described in [replay.md](replay.md).

## Snapshots & Resume

The client subscription snapshot and the crash-resume snapshot are different types with different purposes. Client snapshots provide current shared display state. Resume snapshots persist a small, explicitly versioned set of flight-critical state alongside incremental IGC logging. See [replay.md](replay.md#crash-resume) and [data.md](data.md).

## Alternatives Considered

Rejected macro-architectures:

- **Actor frameworks:** several independent actors would replace one replayable input order with several schedules.
- **ECS:** most state is one value of each kind, not thousands of similar objects.
- **Demand-driven incremental computation:** inputs push updates into the app. The app is not a large database that answers many linked calculations on demand.
- **A declared computation graph:** a handful of cheap values and a few explicit workers do not justify graph machinery.
- **Full CQRS or event sourcing:** the recorded input sequence already provides the needed history.
- **Reactive signal graphs in the core:** they add another scheduler that replay would need to control.
- **CRDTs:** secondary clients are views of one authority, not peer state owners.
- **Priority and coalescing queues:** measured input rates do not justify the extra rules and risk of dropping the wrong input.
- **Per-topic state streams:** one stream with all change groups uses fewer connections and is easier for clients to manage.
