# UI Design

Updraft uses a map-first interface. The **Flight Deck** is the primary in-flight
surface. Ground-oriented work such as planning, flight review, aircraft setup,
and data management lives in separate screens. Terms used by these documents
are defined in the [glossary](../../glossary.md).

The detailed UI is split by responsibility:

- [flight-deck.md](flight-deck.md): the map, navigation targets, Emergency mode,
  the Situation Bar, and map inspection
- [warnings.md](warnings.md): warning presentation, acknowledgement,
  suppression, audio behavior, and native notifications
- [infoboxes.md](infoboxes.md): infobox pages, automatic Thermal behavior, and
  layout editing
- [screens.md](screens.md): Main Menu, app navigation, launch behavior, and the
  ground-oriented screens
- [devices.md](devices.md): device priority, capability presentation, and source
  configuration

The implementation architecture lives in [frontend.md](../frontend.md). Device
source selection and merging live in [devices.md](../devices.md). These UI
documents describe how the user sees and controls those systems.

## Mental Model

The app has two levels:

1. The Flight Deck keeps the map and flight information immediately available.
2. Main Menu opens the rest of the app without destroying the current Flight
   Deck state.

There is no persistent bottom navigation bar. A fixed Menu control opens Main
Menu, and every screen has a Map control that returns directly to the preserved
Flight Deck.

The app remains complete during flight. It may add contextual shortcuts for
in-flight actions, but it does not hide or reorder normal destinations. Active
warnings make the screen's Map control use a warning icon and severity color.
Collision warnings also remain fully visible below the screen header.

## State Ownership

Shared flight state, navigation decisions, and warning state belong to the Rust
core. Saved display configuration, including infobox pages, belongs to the
display profile in Rust-side storage. Temporary presentation state such as the
map viewport, an open dialog, or an unfinished edit remains in the frontend.

One saved infobox layout applies to portrait and landscape. The layout is not
saved separately by orientation. The mapping of its geometry between the two
orientations remains an open prototype decision.

Multi-display support reuses the same model later, but does not shape the first
UI. Each display may eventually use its own saved display profile without
changing shared flight state.

## Responsive and In-flight Interaction

Phones use one full-screen level at a time for complex selection and editing.
Wide displays may retain map context with a side list or a master-detail layout.
The same actions and information remain available in both forms.

In-flight controls use large touch targets, strong contrast, and generous hit
areas. No action is available only through long press. The UI must remain
readable in direct sunlight and usable in turbulence. Color communicates warning
severity, selection, and data state, but shape and text must also carry the
meaning.

## Open Prototype Decisions

- **Launch behavior:** compare direct Flight Deck startup with a preflight
  dashboard and a lightweight preflight overlay. An active flight always resumes
  directly on the Flight Deck.
- **Infobox geometry:** compare ordered reflow, a shared normalized grid, and a
  common dock geometry when mapping one layout between portrait and landscape.
