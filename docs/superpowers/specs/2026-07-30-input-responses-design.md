# Typed input responses

## Context

Updraft's core currently accepts one `Input` enum and returns only a list of
effects. The runtime queues the input and applies it later. A Tauri command
therefore returns as soon as it has enqueued an input, before the core has
accepted or rejected the requested change.

This is sufficient for observations such as device bytes, but it is incomplete
for user mutations. Invalid external-device IDs and invalid reorderings
currently produce no effects and log warnings. The initiating frontend cannot
distinguish that rejection from a successful no-op. Waiting for a later topic
does not solve the problem because topics are not correlated with requests and
a rejected or no-op input emits no topic.

Topics still serve a separate purpose. They synchronize authoritative shared
state to every subscriber, provide a complete initial snapshot, and recover a
frontend after reload. Request responses must complement that state stream
rather than replace it.

## Scope

This design:

- associates every core input type with exactly one response type,
- makes the core return effects and the typed response together,
- gives the runtime one asynchronous `send()` operation that waits until the
  core has applied the input and dispatched its effects,
- returns domain rejections to the initiating host,
- preserves topics as the only shared-state update path,
- adapts existing asynchronous and callback-based producers to the new
  completion contract,
- exposes natural mutation results through concrete Tauri commands and
  frontend client methods.

This design does not:

- wait for settings persistence or a device connection attempt to complete,
- make caller cancellation cancel an admitted input,
- add optimistic frontend state updates,
- add state revisions,
- replace settings or external-device topics,
- add a generic "send arbitrary input" IPC endpoint,
- add external-device settings UI,
- add recording infrastructure before a recorder exists.

## Typed inputs

The current `Input` enum becomes a sealed trait. Each existing enum variant
becomes one concrete input type:

```rust
pub trait Input: private::Sealed + Send + 'static {
    type Response: Send + 'static;

    #[doc(hidden)]
    fn apply_to(self, core: &mut Core, at: Timestamp) -> Update<Self::Response>;
}
```

Sealing keeps the set of inputs owned by `updraft_core`. Hosts may submit the
provided inputs but cannot introduce behavior that bypasses the core's domain
model.

`Core::apply()` remains the public mutation entry point:

```rust
pub fn apply<I: Input>(
    &mut self,
    input: I,
    at: Timestamp,
) -> Update<I::Response> {
    input.apply_to(self, at)
}
```

The generic update keeps the response paired with the effects produced by the
same input:

```rust
pub struct Update<R> {
    pub effects: Vec<Effect>,
    pub response: R,
}
```

There is no global response enum. A `SetLocale` response therefore cannot be
confused with a `DeleteExternalDevice` response.

## Response types

Responses carry only the natural result of one input. They do not repeat the
complete published state.

- `Start`, `Tick`, `Bytes`, `ConnectionChanged`, and `InternalGps` return `()`.
- `SetLocale` returns `()`.
- `AddExternalDevice` returns the allocated `ExternalDeviceId`.
- `DeleteExternalDevice` returns `Result<(), UnknownExternalDevice>`.
- `EditExternalDevice` returns `Result<(), UnknownExternalDevice>`.
- `SetExternalDeviceEnabled` returns
  `Result<(), UnknownExternalDevice>`.
- `ReorderExternalDevices` returns
  `Result<(), InvalidExternalDeviceOrder>`.

`UnknownExternalDevice` carries the requested device ID. The reorder error
identifies the supplied order as invalid without exposing the aggregate's
internal validation algorithm.

A repeated mutation is a successful no-op. Selecting the active locale,
submitting the current device order, editing a device to its current
specification, or setting its existing enabled state returns success with no
effects.

Unknown IDs and invalid orderings return their typed errors with no effects.
These expected domain rejections do not need a core warning merely to make
them observable. Stale observations such as bytes for an unknown or disabled
device remain ignored unit-response inputs because they are facts reported by
a producer, not failed user requests.

## Responses, effects, and topics

The three outputs have distinct roles:

- A response reports the outcome of one correlated input.
- Effects request outside work caused by that input.
- Topics publish current shared state to every subscriber.

Successful state changes continue emitting the same complete settings or
external-device topic and the same persistence effects. Responses never update
frontend stores. This keeps topics as the only shared-state update path and
avoids an older command response overwriting a newer topic.

The runtime dispatches every effect in order before completing the response.
Dispatch means handing the work to its adapter. `PersistSettings` queues the
snapshot for the existing background writer. `OpenConnection` and
`CloseConnection` update transport workers. The response does not wait for a
filesystem write, connection establishment, or disconnection to finish.

Topic delivery and promise resolution cross different Tauri channels and do
not need a defined frontend ordering. A topic may become visible before or
after the initiating promise resolves because only the topic mutates the
shared store.

## Runtime request envelope

`DriverHandle` exposes one input operation:

```rust
pub async fn send<I: Input>(
    &self,
    input: I,
) -> Result<I::Response, DriverStopped>;
```

Every call creates a typed oneshot channel:

```rust
struct Request<I: Input> {
    input: I,
    reply: oneshot::Sender<I::Response>,
}
```

`Request<SetLocale>` can contain only a sender for
`<SetLocale as Input>::Response`. The compiler rejects a sender for any other
input response.

The driver queue must hold requests for different concrete input types. A
private object-safe interface erases only the concrete request type:

```rust
trait ErasedInput: Send {
    fn run(self: Box<Self>, driver: &mut DriverState, at: Timestamp);
}
```

`Request<I>` implements `ErasedInput`. Its implementation applies `I`,
dispatches the returned effects, and sends the response through the already
paired `oneshot::Sender<I::Response>`. It uses no `Any`, downcast, or global
response enum.

The existing message enum retains subscription as a separate runtime
operation:

```rust
enum Message {
    Input(Box<dyn ErasedInput>),
    Subscribe(Sink),
}
```

Subscription registers a long-lived state consumer and is not a core input.
Its atomic snapshot-first behavior does not change.

The initial `Start` input is processed inside the driver task before it accepts
external messages. Periodic `Tick` inputs are processed by the same internal
application helper. Neither needs to make the driver call its own handle.

This slice does not introduce a separate fire-and-forget path. Every submitted
input has the same completion semantics, including unit-response observations.
The per-input oneshot cost is acceptable for current producer rates. A
specialized path should be considered only if measurements show a problem.

## Ordering, cancellation, and shutdown

An awaited `send()` completes after:

1. the request has entered the driver queue,
2. all earlier requests have been processed,
3. the core has applied this input,
4. the driver has dispatched its effects,
5. the driver has delivered its response.

Producers await one input before sending their next input. This provides
backpressure from the core to each transport reader and preserves source-local
ordering.

Once a request has entered the queue, dropping or cancelling its caller does
not cancel the input. The core still applies it and the driver still dispatches
its effects. Sending the response to a dropped receiver fails silently because
caller cancellation is expected.

If queue admission fails, `send()` returns `DriverStopped` and the input was
not accepted. If the driver terminates after admission but before replying,
the dropped oneshot also becomes `DriverStopped`. Domain errors remain inside
`I::Response` and are distinct from runtime termination.

## Host adapters

Async TCP and SPP loops await each input they submit. Reading the next chunk or
event therefore waits until the previous one has reached the core.

Tauri `Channel` callbacks cannot await. A callback-based producer forwards its
parsed values into one ordered channel. One async adapter task receives them
and awaits `DriverHandle::send()` sequentially. The Android SPP adapter already
uses this general channel-to-async-loop structure. GNSS reporting adopts the
same structure rather than spawning one task per callback, which could reorder
fixes.

Tauri continues exposing concrete commands. It never accepts a serialized
arbitrary core input:

```rust
#[tauri::command]
async fn set_locale(
    locale: Locale,
    handle: State<'_, DriverHandle>,
) -> Result<(), DriverStopped> {
    handle.send(SetLocale { locale }).await
}
```

External-device commands construct their corresponding typed inputs and map
domain errors to rejected invocations. A command-specific serializable host
error distinguishes a core rejection from `DriverStopped`, flattening the
runtime's outer result and the input's domain result at the IPC boundary.
Adding a device returns its allocated ID.

The frontend client retains concrete methods such as `setLocale()`,
`addExternalDevice()`, and `deleteExternalDevice()`. Their promises control
pending and error presentation. Shared stores continue applying topics only.
The fake client mirrors both the natural command results and topic publication.

## Recording and replay

No `RecordedInput` enum is added in this slice because the repository does not
yet have an input recorder. Concrete input types retain the traits needed by
current tests.

A future recorder may introduce a closed tagged representation containing the
concrete input values but never response channels. Replay dispatches each value
through the same typed input implementation and discards its deterministic
response. Outside effect and worker results remain ordinary recorded inputs,
as required by the existing deterministic-core design.

## Validation

Core tests cover each changed input contract:

- the natural response and exact effects for a successful change,
- successful no-op responses with no effects,
- typed domain errors with no effects,
- allocated device IDs returned by additions,
- unchanged topic and persistence behavior for successful mutations.

Runtime tests cover:

- FIFO input application and response ordering,
- responses completing only after all effects have been dispatched,
- unit-response inputs using the same path,
- cancelled callers not suppressing admitted inputs,
- driver termination before admission or response,
- snapshot-first subscriptions remaining atomic.

Host tests cover:

- TCP, SPP, and GNSS producers preserving input order while awaiting
  completion,
- real Tauri IPC deserialization for concrete commands,
- typed domain rejections reaching the invoking frontend,
- generated device IDs reaching the invoking frontend.

Frontend tests cover:

- concrete client method results and rejections,
- fake-client parity,
- stores changing only in response to topics,
- a successful response not being treated as a second state update.
