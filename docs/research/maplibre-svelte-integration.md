# Research: MapLibre GL JS Integration in a Svelte 5 Application

> **Status:** research note supporting the `frontend-map` roadmap milestone.
> Investigated 2026-07-07. Versions cited are current as of that date:
> maplibre-gl **5.24.0** (v6 in pre-release, `6.0.0-20`), Svelte **5.56**,
> svelte-maplibre-gl **2.1.0**, svelte-maplibre **1.3.0**.

MapLibre GL JS is framework-agnostic and imperative: you hand it a DOM
container, then drive it through method calls (`addSource`, `addLayer`,
`setPaintProperty`, `setData`, …). Integrating it into a Svelte 5 app is
therefore a question of *how much* of that imperative surface to wrap in
declarative, rune-reactive components — and who maintains the wrapper.

Three realistic approaches exist:

1. **Vanilla integration** — instantiate `maplibregl.Map` ourselves and build
   a thin in-house layer (context + a few `$effect`s) as needed.
2. **[svelte-maplibre-gl][mierune-gh]** (MIERUNE) — a Svelte 5-native
   component wrapper, listed on the official MapLibre plugins page as *the*
   Svelte v5 binding.
3. **[svelte-maplibre][dimfeld-gh]** (dimfeld) — the older community wrapper,
   rewritten for Svelte 5 runes in v1.0 (March 2025).

All three end up in the same place for performance-critical work: MapLibre's
own APIs, called imperatively. The comparison is about everything around that
hot path — composition, reactivity plumbing, maintenance risk, and how well
each approach fits Updraft's specific constraints.

## Updraft's requirements, restated as evaluation criteria

From [design/frontend.md](../design/frontend.md) and
[design/testing.md](../design/testing.md):

- **One fullscreen map with a deep custom layer stack** (basemap/hillshade,
  airspace, waypoints, glide-reach, own track, traffic, ownship) — mostly
  statically *structured*, but with highly dynamic *data* and *filters*
  (altitude-band filtering, declutter, threat colouring).
- **10 Hz hot path**: ownship position updates "bypass Svelte reactivity and
  write straight to the map at frame rate. Reactivity is for UI chrome, not
  the 10 Hz path."
- **Offline-first**: PMTiles basemaps via `pmtiles://` custom protocol; no
  CDN fetches at runtime.
- **Day/night themes**: runtime basemap style switching that must not
  destroy the six custom overlay layers.
- **Deterministic e2e tests**: a `testMode` flag disabling map easing and
  animation; tests await explicit "map idle" signals.
- **SPA, no SSR** (`adapter-static`, `ssr = false`) — SSR-safety of the
  integration is a non-issue for us.
- **Longevity and agent-friendliness**: the code should be boring, typed, and
  survivable across MapLibre major versions (v6 is imminent).

## Approach 1: Vanilla maplibre-gl

### Lifecycle: three idiomatic Svelte 5 options

**`onMount` + `onDestroy`** — the classic, still first-class in Svelte 5 and
what every official tutorial (Mapbox, MapTiler, MapLibre) teaches:

```svelte
<script lang="ts">
  import maplibregl from 'maplibre-gl';
  import 'maplibre-gl/dist/maplibre-gl.css';
  import { onMount, onDestroy } from 'svelte';

  let container: HTMLDivElement;
  let map: maplibregl.Map;

  onMount(() => {
    map = new maplibregl.Map({ container, style, center, zoom });
  });
  onDestroy(() => map?.remove());
</script>

<div class="map" bind:this={container}></div>
```

**`$effect` with a creation guard** — what both wrapper libraries use
internally: `let container = $state()` via `bind:this`, then an `$effect`
guarded by `if (map || !container) return`, reading initial reactive values
through `untrack()` so the creating effect never re-runs.

**Attachments (`{@attach …}`, Svelte 5.29+)** — the designated successor to
actions: a function receives the element on mount, returns a teardown, and
colocates node + setup + cleanup. For an expensive singleton like a map, the
documented discipline is: create the map in the attachment body, put reactive
updates in *nested* `$effect`s, never read reactive state in the body itself
(otherwise the whole attachment re-runs and the map is destroyed and
recreated):

```svelte
<div class="map" {@attach (el) => {
  const map = new maplibregl.Map({ container: el, ...untrack(() => options) });
  $effect(() => { map.setMaxZoom(maxZoom); });  // fine-grained updates
  return () => map.remove();
}}></div>
```

Notably, as of mid-2026 no published MapLibre tutorial uses attachments yet;
tutorials use `onMount`, and both wrapper libraries use `bind:this` +
`$effect`. All three work; for a single app-lifetime fullscreen map the
difference is cosmetic. There is no SSR dimension for us (pure SPA).

### Structuring a larger app around a raw map

The state-of-the-art pattern (readable directly from svelte-maplibre-gl's
source, and worth copying even if we don't use the library):

- A **context class** holding `map = $state.raw(...)` (raw, so the WebGL
  object is not deep-proxied), shared via Symbol-keyed
  `setContext`/`getContext` so child components (layer modules, controls,
  dialogs) can react to map availability with plain `$effect`s.
- A **style-load queue**: `addSource`/`addLayer` before the style is loaded
  is the classic footgun; a `waitForStyleLoaded(cb)` helper that executes
  immediately or defers to the `styledata` event is ~15 lines.
- **State → map**: one `$effect` per property with equality guards, calling
  `setFilter`/`setPaintProperty`/`jumpTo`.
- **Map → state**: a `map.on('move')` handler writing back to `$state`
  fields *with equality guards* to break write-back loops.
- **Hot path**: keep 10 Hz data in plain non-`$state` variables and call
  `source.setData()` / `source.updateData()` / `marker.setLngLat()` directly
  from the transport callback. This is exactly what MapLibre's own
  "update a feature in real time" example does (at 100 Hz, no framework
  involved). For per-target updates in larger collections (traffic),
  `GeoJSONSource.updateData(diff)` applies keyed partial diffs and only
  re-renders affected tiles — the right API for live tracking, and one no
  wrapper currently exposes declaratively anyway.

### Assessment

- **DX**: most boilerplate up front (~100–200 lines for context, lifecycle,
  style-load queue, camera sync), but everything after that is plain MapLibre
  — the documentation, examples, and Stack Overflow surface of the upstream
  project apply 1:1. No wrapper vocabulary to learn (relevant for LLM agents,
  which know raw MapLibre extremely well). Declarative composition of the
  layer stack has to be invented in-house, or the stack stays imperative
  (fine for seven well-known layers, less nice for per-feature markers and
  popups).
- **Performance**: optimal by construction; nothing sits between the
  transport callback and the map.
- **Risk**: zero third-party wrapper risk, zero lag on MapLibre v6; the cost
  is owning subtle plumbing (style-switch layer preservation, write-back
  loop guards) that wrappers have already debugged.

## Approach 2: svelte-maplibre-gl (MIERUNE)

Svelte 5-native (peer `svelte >= 5`, runes and snippets throughout, no legacy
stores/slots), built by MIERUNE Inc., a Japanese GIS company. Listed on the
official MapLibre plugins page as the Svelte v5 component library.

### API shape

Spec-faithful, thin, and minimally opinionated: `<MapLibre>` props extend
`maplibregl.MapOptions`; layer components (`FillLayer`, `LineLayer`,
`SymbolLayer`, `CircleLayer`, `HillshadeLayer`, `RasterLayer`,
`FillExtrusionLayer`, `HeatmapLayer`, …) extend the corresponding
`*LayerSpecification`; sources (`GeoJSONSource`, `VectorTileSource`,
`RasterTileSource`, `RasterDEMTileSource`, `ImageSource`, …) extend
`*SourceSpecification`. Escape hatches are first-class and explicitly
promised ("traditional imperative MapLibre GL JS usage remains fully
supported"):

- `bind:map` and a `children` snippet that receives the `maplibregl.Map`;
- `bind:source` on every source component hands you the live
  `maplibregl.GeoJSONSource` etc. — i.e. the 10 Hz `setData`/`updateData`
  path is one binding away;
- `getMapContext()` in any descendant;
- `RawSource`/`RawLayer`/`CustomLayer` (WebGL custom layer interface) for
  anything unwrapped.

Extension packages: `@svelte-maplibre-gl/pmtiles` (registers the pmtiles
protocol as a component, pmtiles v4), `/deckgl`, `/contour`, `/terradraw`.

### Reactivity model (verified from source)

- Fine-grained `$effect` per map option; **camera writes batch into a single
  `map.jumpTo()`** with only changed keys — no easing, which is exactly what
  our deterministic `testMode` wants by default.
- Layer `paint`/`layout` are **key-level diffed** → `setPaintProperty` /
  `setLayoutProperty` only for changed keys; `filter` → `setFilter`. This
  maps perfectly onto altitude-band airspace filtering as a rune.
- GeoJSON `data` change → full `source.setData()` (differential
  `updateData` is an acknowledged TODO — for the hot path we'd use
  `bind:source` and call it ourselves, which is what we'd do anyway).
- **Runtime style switching preserves user layers**: `setStyle` runs through
  a `transformStyle` that merges component-managed sources/layers back into
  the new base style, keeping their anchor positions, and re-applies
  terrain/sky/light. This is precisely the day/night basemap switch we need,
  and it is genuinely fiddly to hand-roll.
- All mutations queue behind style readiness automatically.
- No throttling/debouncing anywhere — a bound state change is one MapLibre
  call, same cost as hand-written code.

### Health and caveats

- v2.1.0 published 2026-07-06; 36 releases since Nov 2024; Playwright e2e
  suite added May 2026. Supports maplibre-gl `^5.19` **and v6 pre-releases
  already** (`>=6.0.0-0`), released the day after maplibre `6.0.0-20`.
- ~14k npm downloads/week; 317 GitHub stars; corporate-backed but
  **bus factor ≈ 1–2** (one developer authored ~70% of commits).
- **Dual-licensed MIT OR Apache-2.0** — identical to Updraft's own licensing.
- TypeScript quality is high: prop types derive from maplibre-gl's own types,
  so paint/layout/option typing tracks upstream exactly.
- **Offline caveat**: `autoloadGlobalCss` defaults to `true` and injects
  `maplibre-gl.css` **from the unpkg CDN at runtime**. For Updraft this must
  be `autoloadGlobalCss={false}` plus a local CSS import — a one-liner, but
  it must not be forgotten.
- Docs are quickstart + ~31 live examples (terrain, globe, pmtiles, deck.gl,
  custom WebGL layers, hover styles, base-style switching…); no prose API
  reference — the TS types serve that role.

## Approach 3: svelte-maplibre (dimfeld)

The original Svelte wrapper (513 stars, ~26k downloads/week — still the
larger install base). v1.0 (March 2025) was a genuine runes rewrite; as of
July 2026 both wrappers are Svelte 5-native, so the old "svelte-maplibre is
Svelte 4-era" framing no longer holds.

Philosophy differs from MIERUNE's: it bakes in higher-level conveniences —
`MarkerLayer` (a Svelte snippet rendered per feature), built-in hover
feature-state management with `hoverStateFilter`, clustering helpers,
`JoinedData` for client-side attribute joins, popups with
`openOn="hover"`, `filterLayers` to declutter a basemap. In exchange it is
more opinionated and further from the MapLibre spec surface.

Points that matter for Updraft, mostly against:

- **Camera sync uses `map.easeTo()`** (animated) rather than `jumpTo` —
  the opposite of what deterministic `testMode` wants; we would have to
  bypass its camera bindings.
- **maplibre-gl is a regular dependency pinned `^4.0.0 || ^5.0.1`** (not a
  peer dependency), and `pmtiles` v3 is a hard dependency with auto-protocol
  registration — more version coupling, no v6 track yet.
- Slower maintenance: single community maintainer, last release 2026-03-31,
  README still says it "needs proper documentation" and the docs site still
  shows a "full documentation is coming soon" banner; 28 open issues include
  unfixed event/typing edge cases.
- `MarkerLayer` re-queries source features on every `moveend`/`zoom` and
  renders a DOM node per feature — convenient, but exactly the kind of
  hidden cost we'd audit and probably avoid for traffic symbols (SymbolLayer
  in WebGL scales far better than DOM markers).
- License MIT; escape hatches exist (`bind:map`, `getMapContext()`).

## Comparison summary

| | Vanilla | svelte-maplibre-gl (MIERUNE) | svelte-maplibre (dimfeld) |
|---|---|---|---|
| Svelte 5 idiom | whatever we write | runes-native, snippets | runes-native since v1.0 |
| API philosophy | none (raw MapLibre) | thin, spec-faithful | thicker, convenience-oriented |
| Layer stack composition | imperative / in-house | declarative components, key-diffed paint/layout/filter | declarative components, dequal-diffed |
| Camera sync | ours to write | batched `jumpTo` (deterministic) | `easeTo` (animated) |
| Style switch keeping overlays | hand-rolled (hard) | built-in merge, anchors preserved | built-in (snapshot + re-add) |
| 10 Hz hot path | direct | `bind:source` → direct | `bind:map` → direct |
| `updateData` (partial GeoJSON diff) | direct | via `bind:source` (not declarative) | not exposed |
| pmtiles | `addProtocol`, ~5 lines | official extension pkg (pmtiles v4) | hard dep (pmtiles v3), auto-registered |
| Offline safety | full control | must set `autoloadGlobalCss={false}` | CSS bundled via module import |
| maplibre-gl v6 readiness | immediate | peer allows v6 pre-releases today | pinned `^4 \|\| ^5`, no v6 work visible |
| Maintenance | us | corporate-backed, very active, bus factor ~1–2 | single maintainer, slowing |
| Downloads/week (July 2026) | — | ~14k | ~26k |
| License | — | MIT OR Apache-2.0 (same as Updraft) | MIT |
| TypeScript | upstream types | derives from upstream types | good, minor `any`s |

Bundle size is a non-factor in the choice: maplibre-gl itself is ~1.03 MB
minified / ~269 KB gzip and effectively non-tree-shakeable; both wrappers add
negligible weight on top.

## Recommendation

**Use svelte-maplibre-gl for the declarative shell, and MapLibre's raw APIs
(via `bind:map` / `bind:source`) for the hot path.** Concretely:

- `<MapLibre>` + source/layer components structure the layer stack
  (airspace, waypoints, glide-reach, track, traffic, ownship as sibling
  components), with runes driving `filter`/`paint` — altitude-band filtering
  and threat colouring become one-line `$derived` expressions.
- Ownship/traffic position updates go through `bind:source` →
  `setData()`/`updateData()` directly from the topic decoder, exactly as the
  design doc's hot-path exception prescribes. The wrapper adds zero code to
  this path.
- `@svelte-maplibre-gl/pmtiles` (or a five-line manual `addProtocol` at
  module scope) covers offline basemaps; **`autoloadGlobalCss={false}`** with
  a local CSS import is mandatory for offline operation.
- The `jumpTo`-based camera bindings and animation-free reactive updates
  align with `testMode` determinism out of the box.

Why not vanilla: we would end up re-implementing the wrapper's genuinely
tricky parts — style-switch overlay preservation for day/night themes,
style-load queueing, camera write-back loop guards, key-level paint diffing —
with no offsetting benefit, since the wrapper already exposes the raw map
everywhere we need it. Lock-in is low by design: components extend the
MapLibre spec types, and if the library ever stalls, migrating to vanilla is
mechanical (every component corresponds 1:1 to `addSource`/`addLayer` calls
we understand).

Why not dimfeld/svelte-maplibre: animated camera sync conflicts with test
determinism, the maplibre-gl `^4 || ^5` regular-dependency pin lags upstream
(v6 imminent), maintenance is visibly slower, and its value-adds (DOM
`MarkerLayer`, hover state, clustering) are features we'd avoid for
performance anyway or implement in WebGL layers.

Residual risks to accept and monitor:

- **Bus factor** on svelte-maplibre-gl (~1–2 core developers, MIERUNE-backed).
  Mitigated by low lock-in and by our hot path already being wrapper-free.
- **MapLibre v6** ships breaking changes (WebGL1 dropped, event classes
  reworked); the wrapper tracks pre-releases, but we should pin maplibre-gl 5.x
  until v6 stabilizes and the wrapper declares stable support.
- The wrapper deep-snapshots `paint`/`layout` objects on change — keep those
  objects small (they are, per layer) and keep bulk data out of `$state`,
  which the design already mandates.

## Appendix: Lessons from svelte-maplibre-gl's Playwright e2e suite

svelte-maplibre-gl added a Playwright e2e suite in May 2026 (v2.0 hardening).
Examined at commit `32031f4` (2026-07-06). It is small (~10 specs, ~300
lines) but directly relevant to Updraft's planned e2e harness
([design/testing.md](../design/testing.md)), because it is a working example
of testing real MapLibre WebGL rendering headlessly in GitHub Actions CI.

### What they do

**Headless WebGL needs zero special configuration.** The Playwright config is
five lines — build + `vite preview` as `webServer`, a `testMatch` glob,
nothing else. No SwiftShader flags, no `--use-gl`, no xvfb, no llvmpipe
setup; plain `playwright install --with-deps chromium` on `ubuntu-latest`.
Default headless Chromium's built-in software GL renders MapLibre well enough
that `queryRenderedFeatures()` returns correct results in CI. This settles
testing.md's open question ("headless MapLibre requires software GL …
validated when the e2e harness is established") in the best possible way:
there is nothing to validate beyond using stock Playwright Chromium.

**Assert on map state, not pixels.** There is no screenshot or visual
regression testing anywhere. Tests assert semantic map state instead:
`map.getStyle().layers` id ordering (layer z-order and `beforeId`
resolution), `map.getLayer(id).minzoom/maxzoom`,
`map.queryRenderedFeatures({ layers: [...] }).length` (is anything actually
rendered from this layer at this zoom), and — since MapLibre renders markers,
popups, and controls as DOM — ordinary Playwright locators and class
assertions for those. This style is fast, robust across GL rasterization
differences, and maps directly onto Updraft assertions like "the airspace
layer renders features at this altitude filter" or "the ownship marker moved
east".

**Test hooks on `window`.** Every test fixture page exposes the map and
scenario-specific mutators globally in an `$effect`:
`window.__map = map`, `window.__setStyleVariant = (v) => { variant = v }`.
Specs drive Svelte state via `page.evaluate(() => window.__setStyleVariant('b'))`
and synchronize via `page.waitForFunction()` over map-derived predicates
(e.g. "style contains layer X", "queryRenderedFeatures returns 0"). This is
the concrete shape of Updraft's planned `testMode`: expose `__map` (and
sim-control hooks) when the flag is set, and make tests await explicit map
state rather than time.

**Fully synthetic, network-free fixtures.** Fixture pages use inline
`{ version: 8, sources: {}, layers: [...] }` styles built from invisible
`background` layers (opacity 0 / `visibility: none`) as ordering anchors,
`data:` URL tile endpoints for raster/vector sources, and single-point
GeoJSON. Tests are deterministic and run offline. Only the separate
examples smoke test touches real tile servers, and it deliberately counts
only *page errors*, not failed tile requests. For Updraft: e2e styles and a
small committed PMTiles fixture should be served by the simulation-profile
`updraft-server`, never fetched from the internet in CI.

**A regression test per bug, as a named route.** Each fixture lives in the
app itself under `/test/<case>/` (e.g. `/test/style-swap-layer-order/`),
one page per historical bug, with a comment referencing the issue. The
`/test/` routes ship in the site build; Updraft can gate equivalents behind
`testMode` or a dev-only route group.

**Universal error invariant + auto-discovered smoke sweep.** Every spec
collects `page.on('pageerror')` and asserts the list is empty at the end — a
cheap catch-all that turns any uncaught exception into a test failure. A
smoke suite enumerates all example directories from the filesystem at
collection time and visits each page in parallel. The Updraft equivalent: a
smoke test over every route/dialog of the SPA with the pageerror invariant,
independent of feature-specific tests.

**CI tracks upstream MapLibre pre-releases without blocking PRs.** The
workflow runs a two-leg matrix: the pinned ("catalog") maplibre-gl version
as a required check, plus a `continue-on-error` leg that overrides
maplibre-gl to `next` (or any dist-tag via `workflow_dispatch` input) using
a script that temporarily rewrites the pnpm workspace override and restores
it afterwards. A weekly cron runs the `next` leg even without repo changes,
so new MapLibre pre-releases are exercised within a week. With maplibre v6
imminent, this pattern is worth copying wholesale — it gives early warning
of breakage in our layer stack while keeping PR CI green and pinned.
Smaller touches: Playwright browser binaries cached keyed on the lockfile,
and e2e runs against the production build (`build` + `preview`), not the
dev server — which matches our adapter-static SPA reality.

### What they don't do (also informative)

- **No unit tests for wrapper behavior.** Vitest is configured but contains
  a single demo spec; all real confidence comes from browser e2e. The
  implicit lesson: MapLibre behavior is not meaningfully testable in
  jsdom/happy-dom — don't budget for map unit tests, go straight to a real
  browser. (Updraft's frontend already runs Vitest in real-browser mode via
  `vitest-browser-svelte`, so component-level map tests could use that layer
  too; the pyramid in testing.md is consistent with this.)
- **A few `waitForTimeout()` sleeps remain** (50–500 ms for microtask
  flushes and `{#key}` re-render loops) — the one flaky-prone spot in an
  otherwise signal-driven suite. Updraft's planned explicit "map idle" /
  "data version N rendered" signals are strictly better; adopt their
  `waitForFunction`-on-map-state style and avoid the sleeps.
- **No test of high-frequency data updates** (nothing exercises `setData`
  at rate) and no camera-animation determinism handling — their reactive
  camera is already animation-free (`jumpTo`), which is exactly why
  Updraft's `testMode` should keep animations disabled rather than trying
  to await easing.

## Sources

- svelte-maplibre-gl: [GitHub][mierune-gh] · [docs][mierune-docs] ·
  [npm](https://www.npmjs.com/package/svelte-maplibre-gl) ·
  [CHANGELOG](https://github.com/MIERUNE/svelte-maplibre-gl/blob/main/svelte-maplibre-gl/CHANGELOG.md)
- svelte-maplibre: [GitHub][dimfeld-gh] ·
  [docs](https://svelte-maplibre.vercel.app/) ·
  [npm](https://www.npmjs.com/package/svelte-maplibre) ·
  [v1.0 release notes](https://github.com/dimfeld/svelte-maplibre/releases/tag/v1.0.0)
- MapLibre GL JS: [plugins list](https://maplibre.org/maplibre-gl-js/docs/plugins/) ·
  [GeoJSONSource.updateData](https://maplibre.org/maplibre-gl-js/docs/API/classes/GeoJSONSource/) ·
  [real-time update example](https://maplibre.org/maplibre-gl-js/docs/examples/update-a-feature-in-realtime/) ·
  [pmtiles protocol example](https://maplibre.org/maplibre-gl-js/docs/examples/pmtiles-source-and-protocol/) ·
  [April 2026 newsletter (v5 final / v6 pre-release)](https://maplibre.org/news/2026-05-02-maplibre-newsletter-april-2026/)
- Svelte 5: [attachments](https://svelte.dev/docs/svelte/@attach) ·
  [$effect](https://svelte.dev/docs/svelte/$effect) ·
  [$state / $state.raw](https://svelte.dev/docs/svelte/$state) ·
  [lifecycle hooks](https://svelte.dev/docs/svelte/lifecycle-hooks) ·
  [context](https://svelte.dev/docs/svelte/context)
- Tutorials using vanilla `onMount` integration:
  [Mapbox + Svelte](https://docs.mapbox.com/help/tutorials/use-mapbox-gl-js-with-svelte/) ·
  [MapTiler + Svelte](https://docs.maptiler.com/svelte/maplibre-gl-js/how-to-use-maplibre-gl-js/) ·
  [MIERUNE tutorial](https://dev.to/mierune/a-guide-to-building-a-map-application-with-svelte-58je)

[mierune-gh]: https://github.com/MIERUNE/svelte-maplibre-gl
[mierune-docs]: https://svelte-maplibre-gl.mierune.dev/
[dimfeld-gh]: https://github.com/dimfeld/svelte-maplibre
