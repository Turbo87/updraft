# 0001: Use svelte-maplibre-gl to integrate MapLibre GL JS

- **Status:** accepted
- **Date:** 2026-07-08
- **Research:** [research/maplibre-svelte-integration.md](../research/maplibre-svelte-integration.md)

## Context

The frontend is a Svelte 5 / SvelteKit SPA whose centerpiece is one
fullscreen MapLibre GL JS map with a deep custom layer stack (basemap,
airspace, waypoints, glide-reach, track, traffic, ownship — see
[design/frontend.md](../design/frontend.md)). MapLibre itself is
framework-agnostic and imperative, so the integration question is how much
of its surface to wrap in declarative, rune-reactive components — and who
maintains that wrapper. Hard constraints from the design docs: a 10 Hz
ownship/traffic hot path that must bypass UI reactivity, offline-first
operation (PMTiles, no CDN fetches), day/night basemap switching that must
not destroy the overlay layers, and deterministic animation-free e2e tests
(`testMode`).

## Decision

Use **[svelte-maplibre-gl](https://github.com/MIERUNE/svelte-maplibre-gl)**
(MIERUNE) for the declarative shell — map, sources, layers, markers,
controls as Svelte components with runes driving `filter`/`paint` — and
MapLibre's raw APIs via the library's first-class escape hatches
(`bind:map`, `bind:source`, `getMapContext()`) for the imperative hot path.

What tipped the decision:

- Svelte 5-native and spec-faithful: component props extend maplibre-gl's
  own types, so the API tracks upstream exactly and there is little wrapper
  vocabulary to learn or outgrow.
- Reactive updates are key-level diffs (`setPaintProperty`/`setFilter` for
  changed keys only), and camera bindings use animation-free `jumpTo` —
  which matches `testMode` determinism by default.
- Runtime `setStyle` merges user layers back into the new base style with
  their anchor positions preserved — exactly the day/night basemap switch,
  and genuinely fiddly to hand-roll.
- Escape hatches make the wrapper free on the hot path: `bind:source` hands
  us the live `GeoJSONSource` for direct `setData()`/`updateData()` calls.
- Actively maintained (corporate-backed by MIERUNE, listed as the official
  Svelte 5 binding on the MapLibre plugins page), already tracking
  maplibre-gl v6 pre-releases, Playwright e2e suite upstream, and
  dual-licensed MIT OR Apache-2.0 — identical to Updraft's own licensing.

## Alternatives considered

**Vanilla maplibre-gl with an in-house thin wrapper.** Full control, zero
third-party risk, immediate MapLibre v6 readiness. Discarded because we
would re-implement the genuinely tricky plumbing the wrapper has already
debugged — style-switch overlay preservation, style-load queueing, camera
write-back loop guards, paint diffing — with no offsetting benefit, since
svelte-maplibre-gl already exposes the raw map everywhere we need it.
Remains the documented fallback: because the components map 1:1 onto
`addSource`/`addLayer` calls, migrating to vanilla is mechanical if the
library ever stalls.

**svelte-maplibre (dimfeld).** The older community wrapper, also
runes-native since v1.0 and with the larger install base. Discarded
because: camera sync animates via `easeTo` (conflicts with `testMode`
determinism), maplibre-gl is a regular dependency pinned `^4 || ^5` with no
v6 work visible, pmtiles v3 is a hard dependency, maintenance is visibly
slower (single maintainer, docs incomplete), and its distinguishing
conveniences (DOM `MarkerLayer`, hover-state management, clustering) are
features we would avoid for performance reasons anyway.

## Consequences

- `autoloadGlobalCss={false}` is **mandatory** on `<MapLibre>`, with
  `maplibre-gl/dist/maplibre-gl.css` imported locally — the default injects
  CSS from the unpkg CDN at runtime, which violates offline-first.
- Ownship/traffic updates go through `bind:source` →
  `setData()`/`updateData()` directly from the topic decoder; bulk data
  stays out of `$state` (already mandated by the design).
- PMTiles via `@svelte-maplibre-gl/pmtiles` (or a manual `addProtocol` at
  module scope — five lines, either works).
- Pin maplibre-gl to 5.x until v6 is stable and the wrapper declares stable
  support; a non-blocking CI canary tracks `maplibre-gl@next` (see
  [design/testing.md](../design/testing.md)).
- Accepted risk: the wrapper's bus factor is ~1–2 core developers.
  Mitigated by low lock-in (spec-faithful API, wrapper-free hot path) and
  the vanilla fallback above.
