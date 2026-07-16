# Flight Deck

The Flight Deck is Updraft's primary in-flight surface. A fullscreen map is the
base layer. A fixed Menu control, the Situation Bar, map controls, and the
infobox dock sit above it without turning the map into a collection of permanent
toolbars.

## Layout

The map remains the base layer beneath these edge regions. In portrait, the
infobox dock moves to the bottom and leaves the Situation Bar wide. In landscape,
the dock moves to the right and the Situation Bar uses the remaining width. This
conceptual wireframe does not decide whether the regions reserve or overlay map
pixels. The optional pinned area is content-sized up to the available width. Its
size below is only illustrative.

```text
Portrait                        Landscape
┌───────────────────────┐       ┌───────────────────────────────────┬─────────┐
│ Menu │ Situation Bar  │       │ Menu │ Situation Bar              │ Infobox │
│─┐───────────────────┌─│       │───────┐───────────────────┌───────│ dock    │
│ │0..n pinned targets│ │       │       │0..n pinned targets│       │         │
│ └───────────────────┘ │       │       └───────────────────┘       │         │
│                       │       │                                   │         │
│                       │       │                Map                │         │
│                       │       │           [map controls]          │         │
│          Map          │       │                                   │         │
│     [map controls]    │       └───────────────────────────────────┴─────────┘
│                       │
│                       │
│                       │
├───────────────────────┤
│      Infobox dock     │
└───────────────────────┘
```

The map remains freely pannable. Moving away from the aircraft reveals a return
to position control. Map orientation and automatic zoom may react to flight mode,
but the pilot can always pan and return manually.

## Situation Bar

The Situation Bar occupies one fixed region. In its normal state it shows the
focused navigation target. A warning temporarily replaces the target in the
same region, so the map does not resize when a warning appears. Warning behavior
is defined in [warnings.md](warnings.md).

The target presentation prioritizes:

1. relative bearing
2. arrival height
3. distance
4. target name

When horizontal space is tight, the target name shortens before a navigation
value is removed. Numeric values are never truncated. Arrival height is fixed
initially. A later option may replace it with the required glide ratio or
another target metric.

Relative bearing initially compares the target bearing with the current ground
track. It is displayed as `◁ 23°`, `12° ▷`, or `◁ 0° ▷`. Rounding and a small
dead zone around zero prevent left and right from flickering. A future navigation
calculation may adjust the required track for crosswind without changing this
presentation.

## Navigation Targets

Several targets may remain active or monitored, but exactly one is focused. The
focused target drives the map course line, distance, arrival calculations,
relative bearing, and every target-dependent infobox. Switching focus does not
deactivate the task or remove the other targets.

Targets form this non-wrapping sequence:

`Emergency | Task | Additional target 1 | Additional target 2 | …`

An additional target may represent a waypoint, an arbitrary map position, or
live traffic. Different target types occupy the same ordered sequence rather
than separate groups.

Task is the default position. Swiping horizontally anywhere on the Situation Bar
changes the focused target. The whole bar is the touch target, so permanent
arrows or tabs are unnecessary. A brief, non-clickable indicator confirms the
new position. Tapping the bar opens a list for direct selection and target
management.

### Pinned Targets

Any target may be pinned, including the dynamic Emergency and Task positions
and each additional target. Nothing is pinned by default, and there is no fixed
limit. Every pinned target receives a compact live readout directly below the
Situation Bar. The pinned area is only as wide as its readouts require, up to the
available width. Readouts follow the navigation sequence order. The app does not
collapse or hide them to protect map space.

The Situation Bar and pinned area are separate controls with the same action.
Tapping either opens the target list. Individual readouts within the pinned area
do not have distinct actions, and unused map width beside the pinned area remains
part of the map. The exact readout density and responsive layout remain an
implementation-time design decision.

The focused target appears only in the Situation Bar, even if it is pinned. Its
pinned readout is omitted to avoid showing the same information twice. When a
warning replaces the focused target, the pinned readouts remain visible and the
focused target temporarily appears above them as another compact readout. That
temporary readout disappears when the warning clears.

Emergency and Task pins follow their dynamic positions rather than preserving
the currently resolved waypoint. Emergency therefore follows the selected
landable, while Task follows task progress.

### Task Target

The Task presentation, whether focused or pinned, must show all of these values
in some form:

- relative bearing to the next turnpoint
- distance to the next turnpoint
- remaining distance along the complete task
- arrival height at the task finish

The two distances need clear labels or visual grouping so they cannot be
mistaken for one another. Their exact arrangement is deferred until this UI is
implemented and tested at different screen sizes.

### Traffic Targets

Live traffic may occupy any additional-target position and may be focused or
pinned like another target type. Its position, course line, relative bearing,
and distance update as new traffic reports arrive. Relative altitude replaces
arrival height in its target presentation.

When updates stop, the target remains in the same sequence position and retains
its last-known marker. The UI marks it unavailable and shows the age of its last
report. Navigation guidance continues toward that last-known position. New
reports for the same traffic identity resume live updates. The pilot must focus
another target or remove this target to stop using it for guidance.

## Map Inspector

A normal map tap always opens the inspector for the selected position. Its
persistent point section shows distance and arrival height at that position and
offers point-level actions such as **Navigate here** and **Drop marker**. These
actions remain available regardless of how many nearby objects are found.

Nearby objects appear in a separate results list. With no matches, the list is
empty. Initially, one match remains a single-item list rather than opening its
details directly. Several matches use a full-screen categorized list on phones
so closely spaced or overlapping objects remain easy to distinguish. Wide
displays may use a side list while retaining the map. The single-result behavior
may be revisited after implementation testing.

The inspector grows incrementally. Its result types include waypoints and
landables first, followed by airspaces, traffic, task points, weather, and
terrain. List items may update while visible, for example when the distance to a
traffic target changes.

## Emergency Mode

Emergency occupies one position in the target sequence. Entering it initially
selects the highest-ranked suitable landable. The candidate set contains up to
three landables and includes at least one suitable airfield when available.

The map shows guidance lines to all candidates. Each candidate has a label with
its name and arrival height. The selected candidate uses a strong highlighted
line and marker. The others use quieter lines but remain clearly visible. Labels
may be offset and connected to their markers with leader lines when needed to
prevent overlaps.

Tapping a candidate label or marker immediately selects it, updates the
Situation Bar and dependent infoboxes, and highlights its line. This explicit
control bypasses the general map inspector. The marker and label are separate
touch targets.

Once the pilot selects a candidate, it remains selected until another candidate
is chosen explicitly. Ranking changes may replace the two unselected candidates
in the background, but never silently redirect navigation. If the selected
target becomes unsuitable or unreachable, the UI shows that condition clearly.
