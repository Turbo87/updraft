# Map State Design

## Purpose

The frontend needs shared access to the MapLibre map and its camera state.
Route screens must be able to inspect or control the map through the existing
application context.

This change adds one reactive `MapState` object. It moves the existing map
instance and follow state out of `Map.svelte`. It also tracks the camera center,
zoom, bearing, and pitch.

This change does not change map-follow behavior.

## Scope

This design includes these functions:

- Store the MapLibre instance in shared frontend state.
- Store the current center, zoom, bearing, and pitch.
- Store the current follow mode.
- Make the same `MapState` instance available through `AppContext`.
- Pass that instance explicitly through `FlightView` to `Map.svelte`.
- Bind the MapLibre camera properties directly to `MapState`.
- Keep the state while another application screen covers the Flight View.
- Give route screens direct access to the MapLibre instance.

This design does not include these functions:

- State persistence across application restarts.
- Browser storage or Rust-side display storage.
- New camera controls or map-follow behavior.
- Wrapper methods for MapLibre operations.
- A map command queue or command API.
- Core, Tauri, protocol, or settings changes.

## State model

The frontend adds `MapState` in `$lib/map-state.svelte.ts`. The class contains
these public reactive fields:

- `map` is the MapLibre `Map` instance or `undefined`.
- `center` contains `lat` and `lng` values.
- `zoom` contains the current zoom level.
- `bearing` contains the current bearing in degrees.
- `pitch` contains the current pitch in degrees.
- `followMode` shows whether the map follows the ownship position.

The initial state is:

- `map` is `undefined`.
- `center` is latitude `50.823` and longitude `6.186`.
- `zoom` is `11`.
- `bearing` is `0`.
- `pitch` is `0`.
- `followMode` is `true`.

The `map` field uses `$state.raw`. Svelte can react when MapLibre replaces the
reference. Svelte does not apply deep reactive state to the MapLibre object.

The other fields use `$state`. MapLibre and other frontend components can read
and update them directly. The class does not contain methods.

## Lifetime

The root layout creates one `MapState` instance for one application session.
The layout keeps the Flight View mounted behind route screens. The same state
therefore remains active while those screens are open.

A new application session creates a new state with the initial values. The
frontend does not save or restore any field.

## Application context

`AppContext` adds one `mapState` field. Route components use
`getAppContext().mapState` when they need direct map access.

The root layout creates the state before it calls `setAppContext()`. The layout
also passes the state to `FlightView` as a required prop. `FlightView` passes it
to `Map.svelte` as a required prop.

This explicit prop path keeps `Map.svelte` independent from `AppContext`.
Component tests and Storybook can provide a state without an application
context provider.

The data flow is:

```text
+layout.svelte
|- creates MapState
|- publishes MapState through AppContext
`- passes MapState to FlightView
   `- passes MapState to Map.svelte
```

## MapLibre bindings

`Map.svelte` binds these MapLibre properties to `MapState`:

- `map`.
- `center`.
- `zoom`.
- `bearing`.
- `pitch`.

MapLibre sets `mapState.map` when it creates the map. MapLibre sets the field to
`undefined` when it destroys the map. No adapter or conversion is necessary.

MapLibre starts with the camera values from `MapState`. A user camera movement
updates the bound fields. An imperative MapLibre camera operation also updates
the fields through MapLibre movement events.

A component can update a bound camera field directly. MapLibre then applies
that value to the camera.

## Follow mode integration

`Map.svelte` replaces its local `following` state with
`mapState.followMode`. The existing map-follow effects and event handlers keep
their current responsibilities.

A user pan sets `followMode` to `false`. The Return control sets it to `true`.
Position updates call `easeTo()` through `mapState.map` while follow mode is
active.

A center-only follow operation preserves zoom, bearing, and pitch. The bound
camera fields still record their current values.

## Direct MapLibre access

An application-context consumer can use `mapState.map` for imperative MapLibre
operations. Examples include rendered-feature queries, source access, and
camera operations.

`MapState` does not validate or wrap these operations. A consumer can bypass
the normal behavior in `Map.svelte`. This direct access is an intentional part
of this design.

Consumers must handle an `undefined` map. The map is unavailable before
MapLibre construction and after MapLibre destruction.

## Automated tests

Implementation follows red-green-refactor.

Focused map-component browser tests create a `MapState` and pass it to
`Map.svelte`. These tests verify these behaviors:

- MapLibre puts its map instance in `mapState.map`.
- Camera movement updates center, zoom, bearing, and pitch.
- A user pan sets `followMode` to `false`.
- The Return control sets `followMode` to `true`.
- Position updates still move the camera while follow mode is active.
- Position updates still leave the camera unchanged in manual mode.

The existing MapLibre end-to-end test remains the vertical follow-mode test.
The test-mode window hook remains available for that test. Storybook does not
duplicate these behavior tests.

No core, Tauri, protocol, storage, or persistence test is necessary.

## Acceptance criteria

This feature is complete when all of these statements are true:

- The root layout creates one `MapState` for each application session.
- `AppContext` provides that state to route screens.
- `FlightView` and `Map.svelte` receive the same state through props.
- `MapState` contains `map`, `center`, `zoom`, `bearing`, `pitch`, and
  `followMode`.
- MapLibre keeps the map and camera fields synchronized.
- The existing follow-mode behavior uses `mapState.followMode`.
- Route screens can use the raw MapLibre instance when it is available.
- Other application screens do not reset the state.
- A new application session starts with the initial values.
- The implementation does not add persistence, wrapper methods, or backend
  changes.
