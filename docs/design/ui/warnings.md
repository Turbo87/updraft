# Warnings

Warning generation and safety decisions belong to the Rust core. Safety-related
audio runs on the native side. The frontend presents the current warning state
and sends explicit acknowledgement or suppression commands.

## Situation Bar Presentation

The highest-priority active warning replaces the target information in the
Situation Bar. The bar keeps its normal height, so the map and infoboxes do not
move. Additional active warnings remain available through the warning details,
but the Situation Bar does not show a counter.

Pinned targets remain visible. While the warning occupies the Situation Bar,
the focused target appears temporarily as the first compact readout above the
permanently pinned targets. It returns to the Situation Bar when the warning
clears.

Each warning has two primary interactions:

- Tapping the compact `✓` button acknowledges the warning. It means "I have seen
  this warning," removes it from the bar, and restores the target information.
- Tapping the warning body opens its details and, when relevant, the complete
  warning list.

The acknowledgement button does not control audio. Warning audio is not
continuous. While a warning occupies the bar, its acknowledgement and detail
actions replace target switching. Removing the warning restores the target and
its gestures.

Acknowledged or suppressed warnings disappear completely until they become
relevant again. They do not remain as muted entries on the Flight Deck.

## Screens

An active unacknowledged warning changes the existing Map control in every
screen. Its map icon becomes a warning triangle and the control uses the highest
active warning's severity color. It has no warning counter. Its position, size,
and action do not change. Tapping it returns to the Flight Deck, where the full
warning appears in the Situation Bar.

Collision warnings require immediate attention and remain fully visible in
every screen. The normal compact traffic-warning component appears as a fixed
overlay directly below the screen header. It does not move the
underlying controls and blocks touches from passing through to covered content.
The header and warning-aware Map control remain visible above it.

Tapping the collision-warning body returns to the Flight Deck with that traffic
target emphasized. Tapping `✓` acknowledges it without leaving the screen.
The warning then disappears until new or worsening collision danger makes it
relevant again.

## Relevance, Audio, and Native Notifications

A warning produces a one-time sound when it first becomes relevant. A future
version may use a voice message instead. The same warning does not repeat audio
merely because it remains active.

When the app is in the background and the platform supports notifications,
every active warning that is not acknowledged or suppressed is also represented
by a native notification. Newly relevant warnings create notifications while
backgrounded. Moving the app to the background also creates any missing
notifications for warnings that are already active.

The notification mirrors the existing core warning rather than creating a
second warning state. Its identity updates or replaces an existing notification
instead of producing duplicates. Tapping it opens the Flight Deck with that
warning selected. Acknowledging, suppressing, or clearing the warning removes
the corresponding notification where the platform permits.

A warning becomes relevant again when it:

- clears and later recurs
- increases in severity
- reaches the end of a suppression period
- changes in another way that materially changes the hazard

The core owns these transitions so the same rules apply when the webview is
backgrounded or suspended. Warning priority and acknowledgement state must be
deterministic and testable through simulation and replay.

## Airspace Warnings

Airspace details provide suppression choices appropriate for a persistent
geographic hazard:

- Until clear
- 5 minutes
- 15 minutes
- 1 hour
- Today

Choosing one suppresses both the visual warning and repeat audio until that
condition expires or the warning becomes relevant in a materially different
way. The details continue to show the affected airspace, predicted conflict, and
available context.

## Traffic Warnings

Traffic warnings favor immediate acknowledgement and details. The compact `✓`
action is sufficient for normal use, and timed suppression is not prominent.
New or worsening collision danger makes the warning relevant again according to
the traffic threat state.
