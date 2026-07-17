# CSS

Updraft uses a small global foundation and scoped styles in Svelte components.
Global CSS defines shared values and document defaults. Components own the
appearance of their markup.

## Compatibility

Updraft runs in Tauri system webviews, so the browser engine depends on the
platform and device. The CSS and production-build compilation floor is
Chromium 87, Android WebView 87, and Safari 14. This is not a promise that the
complete application supports those browser versions. Libraries such as
MapLibre and the Tauri integration need separate compatibility testing.

The authoritative targets live in
[`frontend/vite.config.ts`](../frontend/vite.config.ts). Lightning CSS processes
CSS in both development and production. The development transformer and
production minifier derive their targets from the same definition so their
output does not drift. Lightning CSS is a pinned direct development dependency.

Vite's development client assumes a newer browser even when Lightning CSS emits
compatible CSS. Older webviews are therefore tested with a production build
rather than the Vite development server.

Modern CSS is part of the source code only when Lightning CSS can safely lower
it for the configured targets. Its compiled output is verified before adoption.
A feature that cannot be lowered has a complete functional fallback or is not
used. Progressive enhancements always leave a usable fallback.

OKLCH colors and `light-dark()` expressions driven by root-level theme
switching are accepted uses of modern CSS. Their compiled output has been
verified for this browser floor. Theme switching remains at the document root
because lowering `light-dark()` cannot preserve every case where `color-scheme`
changes farther down the tree.

## Global styles

`frontend/src/app.css` is the single global entrypoint. It contains only these
imports, in dependency order:

```text
reset.css -> fonts.css -> colors.css -> theme.css -> base.css -> utils.css
```

The imported files live in `frontend/src/styles/`:

- `reset.css` establishes predictable browser defaults, including box sizing
  and inherited typography.
- `fonts.css` loads application typefaces.
- `colors.css` contains the versioned Tailwind color palette.
- `theme.css` defines semantic tokens and light and dark theme behavior.
- `base.css` styles global elements such as `html`, `body`, links, and form
  controls. It may consume tokens defined by the preceding files.
- `utils.css` contains accessibility and document-level helpers that work on
  arbitrary elements.

`utils.css` grows only in response to a concrete repeated need. It is not a
general set of spacing, color, flexbox, or grid utilities. Stable layout
concepts are Svelte components with scoped styles instead.

Third-party CSS is imported close to the component that owns the integration.
It appears in the `app.css` import chain only when it genuinely affects the
whole application. Storybook loads the same `app.css` entrypoint as the
application so components use the real reset, tokens, themes, and base styles.

## Component styles

Regular visual rules live in the component's `<style>` block, where Svelte
scopes them by default. Simple class selectors are named for an element's role,
such as `.toolbar` or `.status`, rather than its current appearance.

Specificity stays low. Component styles do not use IDs, deeply nested selectors,
or `!important`. Lightning CSS lowers native CSS nesting for the configured
targets in both development and production. Nesting stays shallow and clarifies
a state or relationship rather than reproducing the component's markup
hierarchy.

Variants and states use Svelte class arrays or objects, `data-*` attributes, and
native pseudo-classes. Dynamic values pass through Svelte's `style:` directive
and CSS custom properties rather than constructed inline style strings.

CSS custom properties are also the preferred styling interface for reusable
components. They let a parent configure a component without reaching through
its scoped-style boundary. Public component properties have meaningful defaults
and documentation near the component.

`:global(...)` is limited to markup that the component cannot control, such as
markup from a third-party library. Global selectors are anchored below an
element owned by the component so they cannot affect unrelated content.

## Themes and colors

Updraft has two visual themes: light and dark. Display settings offer `system`,
`light`, and `dark`. The system setting follows `prefers-color-scheme` and does
not set a `data-theme` attribute. Explicit settings select a theme at the
document root:

```css
:root {
  color-scheme: light dark;
}

:root[data-theme='light'] {
  color-scheme: light;
}

:root[data-theme='dark'] {
  color-scheme: dark;
}
```

The saved setting is applied early enough to avoid showing the wrong theme
during startup. Theme-selection logic lives at the application level rather
than in individual components. There is no infrastructure for additional
themes or user-defined palettes.

`colors.css` is a committed copy of the complete default Tailwind color
palette. The file records the release, source URL, retrieval date, and license
notice. Updraft does not otherwise use Tailwind as a CSS framework or runtime
build dependency. The source values remain in OKLCH, while Lightning CSS emits
sRGB and wider-gamut fallbacks.

Reusable UI consumes semantic tokens from `theme.css`. Tokens are named for
their purpose, such as `--color-warning-surface`, rather than for a hue. A token
can select palette values for both themes:

```css
--color-warning-surface:
  light-dark(var(--color-amber-100), var(--color-amber-950));
```

A direct palette variable is acceptable for isolated, theme-independent
styling. Theme-aware styling normally uses a semantic token. A local
`light-dark()` expression is acceptable for a genuine one-off choice when it
uses the root theme mechanism. Repeated usage or a domain role becomes a
semantic token. Components do not contain hard-coded colors.

## Accessibility and interaction

Every semantic color is checked in both themes. Text and controls have
sufficient contrast, and keyboard focus remains clearly visible. Interactive
styling does not depend on hover alone. Motion and transitions respect
`prefers-reduced-motion`.

Responsive rules belong near the component whose layout they change. Touch
targets and important information remain usable across the supported screen
sizes and input methods.

## Review expectations

A CSS change is ready when:

- its rules are scoped to a component unless they are truly global
- its colors follow the semantic-token and direct-palette policy
- its result works in light and dark themes
- its focus, contrast, motion, and non-hover behavior is accessible
- its CSS syntax is lowered for the configured targets or has a usable fallback
- its third-party global selectors are narrowly anchored

## Open question

- How and when should full application compatibility, especially MapLibre on
  older WebKit versions, be tested?
