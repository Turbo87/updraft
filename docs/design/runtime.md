# The Shared Runtime

The runtime is the layer around the deterministic [core](core.md). Each runtime owns one `App`, an input queue, a monotonic clock, compute workers, effect adapters, state-stream subscribers, and computed resource storage. The runtime constructs a fresh `App` from `AppConfig` when its clock starts, so it observes every timer and effect request that the app produces. The axum server and Tauri shell use this layer and add only transport or platform bindings.

Most domain work does not need to know these details. Callers submit typed `Input` values and subscribe to a state stream that starts with a snapshot. Hosts provide adapters for the effects they support.

## Input Queue

The runtime feeds inputs to `App::handle()` through a plain bounded FIFO. A full queue blocks the producer and never drops an input. FLARM alarm edges share the queue, so a lossy `try_send` policy would be unsafe.

There are no priority lanes or separate storage for the latest sensor values. Sensor adapters limit their output rates. Normal device traffic leaves the handler mostly idle, and replacing an older scalar value in state is cheap. Priority lanes would require a rule for every future input type. A wrong rule could silently drop an alarm edge.

Recording captures inputs in the order the runtime passes them to `handle()`. The recorded sequence is therefore exactly what the core observed.

If measurements later show that sensor bursts delay commands, the runtime can add a separate command queue and check it first. This does not change the core protocol or recording format.

## Time and Timers

The runtime defines one monotonic time origin and gives adapters timestamps from the same timeline. After each input, it sets one host timer for `Update.next_deadline`. When that timer expires, the runtime submits a clock input.

Timer identity, deadlines, and deterministic ordering remain core state. The runtime only sleeps and reports that time advanced. Replay can therefore advance time directly without recreating a second scheduler.

## Compute Workers

Cheap derived values remain synchronous. A known expensive calculation enters the worker path through `Effect::Compute`, carrying a snapshot of everything the job needs. Its result returns as an ordinary input.

Compute workers run CPU calculations only. Network polling, downloads, uploads, and device streams use effect adapters with lifecycles suited to those operations.

Each worker kind runs at most one job at a time. A small job slot in the core records whether another run is needed. When the current job finishes, the core asks for a new job if more work arrived. The domain decides which state changes make an older result invalid and whether an older result is safe to display.

Some changes make all earlier work invalid, such as replacing a task or seeking to a distant replay position. These changes increase a compute revision. The core ignores results from older revisions. The runtime clears the worker's cached state when the revision changes.

Invalidating running work also signals a cooperative cancellation token. A worker that can stop between calculation steps returns a typed cancellation, which frees the core job slot without recording a failure. Workers that do not check the token remain correct because the core still rejects their stale result, but fresh work waits for them to finish.

Workers may keep cached intermediate data between runs. A live optimizer can update data from the growing flight trace instead of starting again from nothing. Worker data is a cache, not authoritative state. It is not included in snapshots and starts empty after a restart.

A worker panic becomes a typed failure input. Without a completion or failure input, the core could keep waiting forever for a job that has stopped. The runtime resets the failed worker's cached state before it accepts later jobs.

This lifecycle stays inside the core job slot and the runtime worker adapter. Other feature code does not need to manage it. Domains provide job inputs, rules for rejecting old results, and code that applies valid results. The design does not include a general computation graph.

## Effects

The runtime uses a small `match` over effect types. It does not use a general effect framework. Expected effect groups include:

- warning and notification audio,
- IGC, recording, and snapshot writes,
- compute jobs,
- network fetches and uploads,
- device output,
- computed resource publication.

Effect execution never blocks the input loop. File writes use a dedicated I/O thread. Network work uses async tasks. Compute jobs use per-kind workers. Device output uses the connection writer.

Effect enums describe the work requested by the core. Runtime adapters own how that work runs, including polling, retries, cancellation, caching, and provider-specific behavior. Adapter traits may be used within a domain when implementations are interchangeable. There is no common background-job interface.

Effects that need a result return a typed input. Long operations quickly return an operation ID, then publish progress and completion as normal changes. Warning audio does not use client subscriptions, so it still works when the webview or state stream has a problem.

## State Stream Delivery

The runtime owns the snapshot-first subscription contract:

- registration and snapshot capture occur in one loop turn,
- changes are published in input order,
- subscriber buffers are bounded,
- a full subscriber buffer drops that subscription and records the reason,
- reconnecting starts again with a fresh snapshot.

There is no replay buffer or general patch format. A fresh current snapshot is how a client recovers.

Changes are grouped by domain. This is a useful host-side filtering key without requiring separate streams. High-rate instruments such as attitude and live vario receive their own groups when they land, so a secondary display can omit them without affecting low-rate navigation state. Because a group's encoding can evolve without touching other consumers, a compact binary channel for one high-rate signal, such as the live audio-vario stream, stays a deferred option to revisit only on a measured need.

The runtime measures maximum pending messages, queue wait, and handler time. It also records worker failures and slow-client drops. The warning path has a total time budget under 100 ms. Real sensor adapters provide the workload used to evaluate these measurements instead of relying on unit-test timing.

## Resource Storage

State streams carry resource references, never bulk bytes. A computed resource effect carries its ID, version, and a cheap shared handle to its bytes. The runtime stores the bytes before it publishes the matching change. A client can therefore fetch every version it sees.

Host HTTP routes serve runtime resources directly without entering the input loop or query path. Static resources such as map packs may stream directly from application storage through the same route family.

The own track grows for the whole flight. Each new segment receives a stable feature ID and an increasing sequence number. After the first load, a client fetches `?since=<seq>` and applies only the new segments. A fresh client fetches the whole current track. The server may simplify older parts before sending it.

Resource storage is a runtime detail. The core keeps only the resource identity or read-only dataset handle needed for calculations and queries.

## Host Responsibilities

The standalone server and Tauri shell share this runtime. Hosts are responsible for:

- mapping HTTP requests or platform events into inputs and queries,
- providing concrete effect adapters,
- exposing the state stream and resource routes,
- detecting when the runtime stops and reporting the failure,
- applying authentication and client permissions at the transport boundary.

Domain feature code does not know which host is active.
