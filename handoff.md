# Handoff: shared Svelte UI components

Status: Proposal, not implemented

This document lists Svelte components that are worth extracting from the
current frontend. The goal is to reduce line count, remove copied CSS, and
stop screens from recreating the same UI in slightly different forms.

The survey covers every `.svelte` file under `frontend/src/lib` and
`frontend/src/routes`. Story files and map layer components are excluded.

## Existing primitives

These components already exist in `frontend/src/lib` and are the base for the
proposals below. New components must follow the same conventions.

| Component           | Role                                         |
| ------------------- | -------------------------------------------- |
| `Button`            | Action button with variants and loading      |
| `TextField`         | Labelled text input with hint and error      |
| `ListRow`           | Bordered row with value, trailing, or link   |
| `RadioList`         | Grouped radio options in a card              |
| `InlineChoiceGroup` | Segmented radio choice                       |
| `StatusPill`        | Tone-coloured status label                   |
| `ValueTile`         | Label, value, and unit readout               |
| `ConfirmDialog`     | Destructive confirmation dialog              |
| `ScreenScaffold`    | Screen header, scrolling body, action footer |
| `ExternalLink`      | Anchor that opens in the system browser      |
| `MapOverlayControl` | Round map overlay button or link             |

## Conventions for each extraction

- Put the component in `frontend/src/lib` next to the existing primitives.
- Declare props in a named `Props` type. Do not use inline object types.
- Expose visual configuration through CSS custom properties, as `ValueTile`
  and `TrafficSymbol` do. Do not add a `style` prop.
- Use semantic tokens from `theme.css`. Do not copy `light-dark()` values.
- Add a `.stories.svelte` file that shows each visual state.
- Add a `.svelte.test.ts` file only for behavior the component owns.
  Do not repeat screen tests.
- Extract one component per commit. Migrate the call sites in the same commit
  so the copied CSS disappears with the extraction.
- Do not add props for possible future use.

## Proposed components

The table ranks the proposals. The line estimate counts markup and CSS that
the migration removes from call sites, minus the size of the new component.

| Priority | Component                         | Copies today | Net lines saved (estimate) |
| -------- | --------------------------------- | ------------ | -------------------------- |
| 1        | `Button` prop additions           | 10 + 6 + 1   | ~50                        |
| 2        | `Notice`                          | 7            | ~60                        |
| 3        | `Section`                         | 10           | ~80                        |
| 4        | `Card`, `DetailList`, `DetailRow` | 5            | ~120                       |
| 5        | `ResultList`, `ResultRow`         | 4            | ~180                       |
| 6        | `EmptyState`                      | 2 + 1        | ~50                        |
| 7        | `ErrorMessage`                    | 3 + 17       | ~40                        |
| 8        | `Select`                          | 2            | ~40                        |
| 9        | `TextField` adoption              | 4            | ~50                        |
| 10       | `Switch`                          | 1            | 0                          |
| 11       | `Spinner`                         | 3            | ~30                        |
| 12       | `SummaryGrid`                     | 3            | ~30                        |

### 1. `Button` prop additions

No new component. Three additions remove hand-rolled buttons and inline
styles.

`fullWidth?: boolean`. Ten call sites pass `style="width: 100%"`:
`ConfirmDialog` (2), `ExternalDeviceForm` (4), `AirspaceSetting` (2),
`WaypointSettings` (1), `SettingsIndexScreen` (1). `docs/css.md` asks for
class-based variants instead of inline style strings, so the current pattern
also breaks a documented rule.

`icon?: string`. Six call sites render a leading `<span class="i-mdi-...">`
inside the button with a local `.action-icon` rule. The icon size differs by
file (1.25rem in `SettingsIndexScreen`, 1.5rem in `ExternalDeviceForm` and
`AirspaceSetting`, 1.75rem for the import icon). One prop fixes one size.

`href?: Pathname`. `DevicesScreen` builds an "Add external device" link with
30 lines of CSS that copy the `primary` `large` button styles, plus a disabled
`<span>` twin. `ListRow` and `MapOverlayControl` already show the pattern for
a component that renders an `<a>` or a `<button>` from the same props. Use a
discriminated `Props` union so `href` and `onclick` exclude each other.

### 2. `Notice`

A padded card that holds one short message. The identical CSS block appears
seven times under the names `.empty-state` and `.empty-results`:

- `routes/airspaces/[id]/AirspaceDetails.svelte`
- `routes/airspaces/[id]/+page.svelte`
- `routes/traffic/[id]/TrafficDetails.svelte`
- `routes/nearby/[latitude]/[longitude]/NearbyTraffic.svelte`
- `routes/nearby/[latitude]/[longitude]/NearbyAirspaces.svelte`
- `routes/nearby/[latitude]/[longitude]/NearbyWaypoints.svelte`
- `routes/nearby/[latitude]/[longitude]/+page.svelte`

The nearby page snippets also render bare `<p>` fallbacks for the same
states, so the loading text looks different depending on which branch runs.

Proposed props:

```ts
type Props = {
  children: Snippet;
  action?: Snippet;
  role?: 'status' | 'alert';
};
```

`action` covers the failed state in `AirspaceDetails`, which adds a retry
`Button` under the message. `WaypointLookup` and `GlideSettings` render the
same "message plus retry button" state without any card styling and should
adopt this component too.

### 3. `Section`

A titled block for a details or settings screen. The uppercase caption
heading (`--text-section-title`, `letter-spacing: 0.08em`,
`text-transform: uppercase`, `margin: 0 var(--space-1) var(--space-2)`)
appears in ten files. Four of them also copy the `section + section`
spacing rule: `AboutScreen`, `NearbyResultsScreen`, `AirspaceDetails`,
`TrafficDetails`.

Proposed props:

```ts
type Props = {
  title: string;
  level?: 2 | 3;
  children: Snippet;
};
```

The component renders `<section aria-labelledby>` with a generated id, as
`NearbyResultsScreen` does by hand today. `AirspaceDetails` uses `h3` for the
operating hours block, so `level` is needed on day one.

Two places use a different letter spacing for the same style: `DevicesScreen`
uses `0.06em` and `AirspaceSetting` uses `0.08em`. Pick one in the component.

The `<legend>` elements in `RadioList` and `InlineChoiceGroup` use the same
style but must stay inside their `<fieldset>`. Leave them as they are. The
`ValueTile` label also shares the style and stays inside `ValueTile`.

### 4. `Card`, `DetailList`, `DetailRow`

`Card` is the bordered surface: `1px solid var(--color-border)`,
`var(--radius-card)`, `var(--color-card-surface)`, `overflow: hidden`.
Fourteen files repeat these four declarations. `Card` renders a `div` with
`children` and an optional `class`. Other components compose it.

`DetailList` and `DetailRow` render the label and value rows that details
screens use inside a card. Five files build this list from scratch:

| File                     | Row class      | Differences from the majority    |
| ------------------------ | -------------- | -------------------------------- |
| `AirspaceDetails.svelte` | `.detail-card` | Reference implementation         |
| `TrafficDetails.svelte`  | `dl > div`     | `dt` does not shrink             |
| `AboutScreen.svelte`     | `.row`         | Every `dd` uses the numeric font |
| `AirspaceSetting.svelte` | `.source-row`  | `dd` is bold and not muted       |
| `WaypointDetails.svelte` | none           | Stacked layout, no card          |

The majority row is: flex, space between, `gap: var(--space-4)`,
`min-height: var(--target-min)`, `padding: var(--space-2) var(--space-5)`,
separator border between rows, `dt` in `--text-row-label`, `dd` in
`--text-row-detail` muted and end aligned, and a `numeric` variant in
`--color-value-text` with `--font-numeric` and tabular numerals.

Proposed props for `DetailRow`:

```ts
type Props =
  | { label: string; value: string; numeric?: boolean; children?: never }
  | { label: string; value?: never; numeric?: never; children: Snippet };
```

`children` covers the `StatusPill`, `ExternalLink`, and the two-span
altitude cell in `TrafficDetails`. `DetailList` renders `<dl>` inside `Card`
and owns the separator rule, so `DetailRow` stays a plain `<div>` with
`dt` and `dd`.

`WaypointDetails` should move to the same list. Its stacked layout is the
only details screen that looks different.

Alternative: extend `ListRow` with a grouped variant. This is not
recommended. `ListRow` owns link and size behavior that a `dl` row does not
need, and the grouped row needs `dt` and `dd` semantics.

### 5. `ResultList`, `ResultRow`

A card that lists navigating rows with a name, a detail line, an optional
leading symbol, and a chevron. Three nearby result lists copy about 70 lines
of CSS each with small differences:

| File                     | Leading          | Min height        | Chevron |
| ------------------------ | ---------------- | ----------------- | ------- |
| `NearbyTraffic.svelte`   | `TrafficSymbol`  | `--target-flight` | yes     |
| `NearbyAirspaces.svelte` | none             | `--target-flight` | yes     |
| `NearbyWaypoints.svelte` | `WaypointSymbol` | `--target-flight` | no      |

`DevicesScreen` builds the same row as `.edit-link` at the bottom of each
device card. The `ListRow` story states that the chevron is the only
navigation affordance, so the missing chevron in `NearbyWaypoints` is a bug
that the extraction fixes.

Proposed props for `ResultRow`:

```ts
type Props = {
  href: Pathname;
  name: string;
  detail: string;
  leading?: Snippet;
  stale?: boolean;
};
```

`stale` replaces the `.text.stale` rule in `NearbyTraffic`. `ResultList`
renders `<ul>` inside `Card` and owns the `li + li` separator.

The row uses `--text-row-label` and `--text-row-detail` like `ListRow`. Keep
the two components separate. `ListRow` is a standalone bordered row for
settings navigation. `ResultRow` is a grouped row inside a list card.

### 6. `EmptyState`

A centered block with a large icon, a title, and a description. Two files
implement it with the same values: `DevicesScreen` (`.empty-state`) and
`AirspaceSetting` (`.empty-state`). Both use a 3rem muted icon,
`700 1.375rem` title, muted `--text-body` description with
`max-width: 18rem`, and `padding: var(--space-8) var(--space-5)`.
`WaypointSettings` renders the same state as a bare `<p>` and should adopt
the component.

Proposed props:

```ts
type Props = {
  icon: string;
  title: string;
  description: string;
};
```

Do not merge this with `Notice`. `Notice` is a one-line card for loading,
not found, and empty results inside a screen section. `EmptyState` fills a
whole screen body.

### 7. `ErrorMessage`

An inline error line with the `i-mdi-alert-circle-outline` icon and
`--color-danger-subtle-text`. `TextField` and `RadioList` contain the same
25 lines of markup and CSS. `AirspaceSetting` has a boxed variant with a
`--color-danger-subtle-surface` background.

Beyond those three, seventeen `role="alert"` paragraphs render errors with
no shared style:

- No styling at all: `GlideSettings`, `GlidePerformanceControls`,
  `WaypointLookup`, `NearbyWaypoints`, nearby `+page`.
- `--color-danger-subtle-text` only: `WaypointSettings`, `DevicesScreen`,
  `ConfirmDialog`.
- Local `light-dark(var(--color-red-700), var(--color-red-300))` instead of
  the token: `ExternalDeviceForm`. This breaks the `docs/css.md` rule that
  repeated usage becomes a semantic token.

Proposed props:

```ts
type Props = {
  message: string;
  id?: string;
};
```

The component renders `role="alert"` and the icon. `id` lets `TextField` and
`RadioList` keep their `aria-describedby` links. Add the boxed variant only
if `AirspaceSetting` still needs it after review.

### 8. `Select`

A labelled native `<select>` with a chevron icon. Two files style a select
and they do not match:

- `ExternalDeviceForm` has a 40-line `.select-wrapper` with an absolute
  chevron, `--color-card-surface`, and an uppercase section-title label.
- `GlideSettings` styles the select like the `TextField` input with
  `--color-screen-surface` and a `--text-row-label` label.

Proposed props, mirroring `TextField`:

```ts
type Props = Omit<HTMLSelectAttributes, 'value'> & {
  label: string;
  value?: string;
  hint?: string;
  error?: string;
  children: Snippet;
};
```

`children` holds the `<option>` elements. `ExternalDeviceForm` changes the
option list based on Bluetooth support, so an `options` array prop does not
fit. Use the `TextField` label style so every form control shares one label
appearance.

### 9. `TextField` adoption for number inputs

No new component. `GlideSettings` (arrival reserve) and
`GlidePerformanceControls` (MC, bugs, ballast) each copy the `TextField`
input and label CSS for `type="number"` inputs. `TextField` already spreads
`HTMLInputAttributes`, so `type`, `min`, `step`, `readonly`, and `onchange`
pass through today.

Two changes are needed at the call sites:

- Bind `value` as a string and reset it after a save instead of writing
  `input.value` in the event handler.
- Pass the `*_hint` and `*_invalid` messages through `hint` and `error`
  instead of separate `<p>` elements.

This also gives those screens the error icon and the invalid border that
`ExternalDeviceForm` already has.

### 10. `Switch`

A labelled toggle. `DevicesScreen` implements a custom checkbox with
`role="switch"` in about 60 lines of CSS (`.enabled-row`,
`.checkbox-control`, `.checkbox-visual`). It is the only toggle today, so the
extraction saves no lines. It is listed because a toggle is a base control,
and `docs/css.md` names a planned theme setting that needs one.

Proposed props:

```ts
type Props = {
  label: string;
  checked: boolean;
  disabled?: boolean;
  onChange: (checked: boolean) => void;
};
```

The component renders the whole row as a `<label>` with the text on the
left and the control on the right, as `.enabled-row` does today. The
disabled state keeps the `cursor: wait` behavior that `DevicesScreen`
uses while a request is pending.

### 11. `Spinner`

The `i-mdi-loading` icon with the spin animation and the reduced-motion pulse
fallback. `Button`, `StatusPill`, and `DevicesScreen` each declare the same
two `@keyframes` blocks with different names. Extract one component that
sizes itself with `1em` so each host controls the size through `font-size`.

### 12. `SummaryGrid`

A card that shows `ValueTile` cells in a grid with 1px separator gaps
(`gap: 1px` on a `--color-separator` background). Three files repeat it:
`NearbyResultsScreen`, `AirspaceDetails`, `TrafficDetails`.

Proposed props:

```ts
type Props = {
  columns: 2 | 3;
  children: Snippet;
};
```

`TrafficDetails` adds a full-width alarm cell and `NearbyResultsScreen`
stacks a two-column and a three-column grid. Both work with `children` and
a `grid-column: 1 / -1` rule at the call site. This is the lowest priority
because the three layouts differ the most.

## Related observations

These findings are outside the component scope but came up in the survey.

- `frontend/src/lib/BuildInformation.svelte` and
  `frontend/src/lib/DataCredits.svelte` have no importers other than their
  own tests. `AboutScreen` replaced them. Delete them.
- `NearbyResultsScreen.formatCoordinate` and `TrafficDetails.formatPosition`
  are the same function. `WaypointDetails` formats coordinates a third way
  without hemisphere letters. Move one implementation to `$lib`.
- `TrafficDetails.formatRelativeAltitude` and
  `NearbyTraffic.formatSignedAltitude` compute the same signed altitude
  string. Move one implementation to `nearby-traffic.ts`.
- `DevicesScreen` and `ExternalDeviceForm` both define a `deviceEndpoint`
  or `visibleEndpoint` helper with the same body.
- The label style for form controls differs between `TextField`
  (`--text-row-label`, muted) and the `ExternalDeviceForm` select
  (section-title, uppercase). Decide on one before the `Select` extraction.
