# Map Follow Mode Design

## Purpose

The Flight View map follows the current ownship position. The pilot can pan the
map to inspect another area and can then return to automatic position following.

This design defines the first follow-mode slice. Follow mode controls only the
camera center. It does not control zoom, bearing, or pitch.

## Scope

This design includes these functions:

- Follow each valid ownship position.
- Enter manual mode when the pilot pans the map.
- Keep manual mode active across later position updates.
- Show a conditional Return to position control in manual mode.
- Resume follow mode when the pilot uses the Return control.
- Pause centering during zoom, rotation, and pitch interactions.
- Resume centering after those interactions end.
- Preserve follow state while another application screen covers the Flight
  View.
- Disable camera animation in the existing frontend test mode.

This design does not include these functions:

- Last-known-position tracking.
- Follow-mode or viewport persistence across application restarts.
- Track-based look-ahead placement.
- Automatic zoom.
- North-up, track-up, or target-up policy.
- Circling-specific camera behavior.
- Position interpolation or prediction.
- A general map-camera state store.

The later `map-orientation` roadmap item owns smart offsets, automatic zoom, and
flight-mode-dependent camera policy.

## Follow state

The map component owns one local `following` state. It starts as `true`.

When `following` is `true`, each valid ownship position becomes the camera
center target. When `following` is `false`, position updates continue to move
the ownship symbol but do not move the camera.

Follow state is temporary frontend presentation state. It does not enter the
core, the instruments topic, browser storage, or Rust-side display-profile
storage.

The root layout keeps the Flight View mounted behind other application screens.
Follow state therefore remains unchanged while those screens are open. A new
application session creates the map again and starts in follow mode.

## Position availability

A null position does not change follow state. It also does not start a camera
operation.

If position becomes null while the map is following, the camera stops at its
current location. Follow mode remains enabled. The next valid position resumes
normal following.

If position becomes null while the map is in manual mode, the Return control
remains available. Using Return enables follow mode and hides the control. The
camera does not move until the next valid position arrives.

The frontend does not retain a last known position for this behavior.

## Camera movement

The map uses its fixed default center only during MapLibre construction. It does
not derive the MapLibre `center` property directly from the instruments topic.

After construction, the map applies follow targets through the MapLibre camera
API. A follow operation changes only `center`. It preserves the current zoom,
bearing, and pitch.

Each valid position while follow mode is active starts a short `easeTo()`
transition. The initial production duration is 300 milliseconds. The transition
is not essential motion, so MapLibre can remove it for a user who prefers
reduced motion.

A new position replaces an active follow transition. The map does not queue
position transitions. Test mode uses a zero-millisecond duration but keeps the
same state transitions.

The first valid position after startup uses the same transition as later
positions. The first valid position after an unavailable interval also uses the
same transition.

## Pan interaction

A user pan stops the active camera transition and sets `following` to `false`.
The map then stays at the pilot-selected location.

The implementation must respond to user-originated pan input. A programmatic
camera movement must not enter manual mode. A combined gesture that activates
MapLibre pan behavior enters manual mode.

The Return control appears when the map enters manual mode. It remains visible
until the pilot uses it.

## Zoom, rotation, and pitch interaction

Zoom, rotation, and pitch do not disable follow mode. They pause automatic
centering while the interaction is active.

MapLibre can report more than one active camera interaction for one gesture. The
map tracks the active interaction kinds. It resumes centering only after the
last active zoom, rotation, or pitch interaction ends.

Position updates during the pause do not start camera transitions. When the
pause ends, the map starts one transition to the latest valid position. It keeps
the zoom, bearing, and pitch that the pilot selected.

If the interaction also activates pan behavior, pan takes precedence. The map
enters manual mode and does not recenter when the other interaction kinds end.

## Return control

The Return control is a floating circular button in the lower trailing corner of
the map viewport. Its touch target is at least 48 pixels in each dimension. It
uses theme colors and the shared focus-ring behavior.

The control has a localized `Return to position` accessible label. Its visible
icon is decorative.

`Map.svelte` conditionally renders the control while `following` is `false`. The
button component accepts an `onClick` callback. It does not accept a visibility
property and it does not access MapLibre or instrument state.

Using Return stops an active camera transition and sets `following` to `true`.
The control disappears immediately. If a valid position exists, the map starts
the normal follow transition. If position is null, the map waits for the next
valid position.

A user pan during a Return transition stops that transition and enters manual
mode again.

## Component ownership

`Map.svelte` owns:

- The MapLibre instance.
- Local follow state.
- The active camera-interaction kinds.
- Position-driven camera effects.
- MapLibre event handling.
- Conditional Return-control rendering.

The instruments store continues to hold only the latest instruments topic. It
does not retain a last known position or map interaction state.

The Return-control component owns only its button markup, icon, accessible
label, and styling. It reports activation through `onClick`.

No core, Tauri, protocol, settings, or persistence change is required.

## Automated tests

The existing MapLibre end-to-end test remains the main vertical position test.
It expands to cover this sequence:

1. A valid position moves the ownship and camera.
2. A real pointer pan reveals the Return control.
3. A later position moves the ownship but not the camera.
4. Return hides the control and moves the camera to the current position.
5. A later position moves the camera again.

Focused map-component browser tests cover these cases:

- Zoom pauses centering and resumes at the latest valid position.
- Rotation pauses centering and resumes at the latest valid position.
- Pitch pauses centering and resumes at the latest valid position.
- A combined interaction resumes only after its last active kind ends.
- Pan interrupts a Return transition and enters manual mode.
- Return with a null position enables follow mode without moving the camera.
- The next valid position moves the camera after that Return action.
- Programmatic camera movement does not enter manual mode.

Tests assert final map state. They do not wait for production animation time.
Storybook does not duplicate these interaction tests.

## Acceptance criteria

This feature is complete when all of these statements are true:

- The map starts in follow mode.
- Each valid position starts a short center-only transition while following.
- A user pan stops following and reveals Return.
- Position updates do not move the camera in manual mode.
- Return immediately enables follow mode and hides itself.
- Return waits for a valid position when none is available.
- Zoom, rotation, and pitch preserve follow mode and their selected camera
  values.
- Pan takes precedence over a combined camera interaction.
- Other application screens preserve the current follow state.
- A new application session starts in follow mode.
- Reduced-motion and test-mode users do not receive the production animation.
- The implementation does not add last-known-position tracking, persistence,
  smart offsets, automatic zoom, orientation policy, or circling behavior.
