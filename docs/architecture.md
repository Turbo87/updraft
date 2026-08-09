# Application Architecture

Updraft is a native Tauri application. A deterministic Rust core owns shared
flight state and decisions. The Tauri shell owns I/O and platform integration.
The Svelte frontend owns presentation.

This document describes the current architecture. The
[roadmap](roadmap.md) describes planned changes.

## System boundary

The application has three main parts:

- `updraft_core` owns deterministic state transitions and domain decisions.
- `updraft_tauri` owns the core driver, transport workers, persistence, native
  plugins, and frontend integration.
- `frontend` renders state and sends user actions through an `UpdraftClient`.

Supporting Rust crates own parsers, units, geometry, airspace data, replay
tools, and other reusable domain functions. They do not depend on Tauri.

Updraft does not use an HTTP server in its current application path. Remote
clients and multiple displays are deferred. They must not shape the current
architecture without a concrete requirement.

## Core

`updraft_core::Core` is a deterministic state machine. It does not perform I/O,
read a system clock, create threads, or depend on Tauri or Tokio.

The shell applies one typed `Input` at a shell-supplied monotonic `Timestamp`.
Each input declares its response type. `Core::apply()` returns that response and
a list of `Effect` values.

Effects request external work. The current effects publish a topic, open or
close a transport, or persist settings. The shell matches effects exhaustively.
Pure calculations stay in the core.

The shell sends a `Tick` input at a fixed interval. Scenario tests supply exact
timestamps. Core behavior does not depend on wall-clock time or test sleeps.

## Driver and shell

One Tauri driver task owns the mutable core. It also owns the subscriber list
and active transport workers. Inputs enter through a typed `DriverHandle`.

The driver applies inputs in order. It dispatches every returned effect before
it completes the input response. A new subscriber immediately receives the
current value of every topic.

The shell owns work that crosses a process or platform boundary. This includes:

- TCP and Android Bluetooth SPP connections
- Android foreground execution and internal GNSS
- application settings and airspace-source storage
- native file selection
- Tauri commands and channels
- custom resource responses

Transport workers send bytes and connection state back through the driver.
The core decides which configured transports should be active. The shell owns
connection attempts, cancellation, retries, and platform APIs.

## Frontend protocol

The frontend uses one `UpdraftClient` interface. The production implementation
uses concrete Tauri commands. Browser tests and Storybook use a fake client.
Components do not import a client implementation directly.

Commands report completion or return a typed response. Shared state changes
arrive only through topics. Most topics contain a complete snapshot. The
traffic topic sends one onboarding snapshot and then sends deltas.

Rust protocol types generate committed TypeScript types. Tests verify their
serialized forms. A protocol change must update the Rust type, its serialization
coverage, and the generated TypeScript output together.

## Resources

Topics do not carry large datasets. The Tauri shell registers the `updraft://`
scheme for data that MapLibre or another frontend consumer reads by URL.

The current airspace resource uses
`updraft://localhost/airspace.geojson`. The URL contains a generation value when
the frontend must reload a replaced dataset. The browser test client uses its
own deterministic data path.

Resource projection and serialization run outside the core driver task. The
core can own an immutable canonical dataset when domain queries need it. The
shell owns platform storage and the frontend-specific resource representation.

## State ownership

Shared domain state has one owner:

- The core owns flight instruments, selected source state, traffic, airspace
  status, application settings, and configured external devices.
- The shell owns platform handles, files, sockets, retries, and subscriber
  delivery.
- The frontend owns temporary presentation state, such as map position,
  mounted routes, open pages, and optimistic form values.

Frontend stores contain the current projection of core topics. They are not a
second authority. A fake client must follow the same command and topic contract
as the production client.

## Platform integration

The Tauri shell is the shipping application host. Desktop platforms use Rust
adapters where available. Android integration uses one Kotlin plugin for the
foreground service, permissions, internal GNSS, and Bluetooth SPP.

Platform code converts native events into typed shell inputs. It does not own
flight-domain policy. The core does not know whether bytes came from TCP,
Bluetooth, or a future transport.

## Testing

Tests run at the layer that owns the behavior:

- Rust unit and scenario tests cover parsers, domain state, source selection,
  effects, and deterministic time.
- Tauri tests cover the driver, command deserialization, storage, resources,
  and transport adapters.
- Frontend unit tests cover stores, formatting, components, and MapLibre data.
- Playwright tests cover complete frontend paths through the browser fake.
- Physical verification covers Android lifecycle and hardware behavior that
  automated tests cannot establish.

Storybook presents useful component states. It is not a second automated test
suite.

## Deferred architecture

The current architecture does not include these functions:

- an HTTP API
- remote or secondary clients
- multiple independent displays
- a general background-job framework
- a generic message bus
- a plugin system for domain features

A focused feature requirement can introduce one of these concepts later. That
change must define its ownership, security boundary, and test path before it
changes the current application path.
