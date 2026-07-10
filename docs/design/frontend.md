# The Frontend

The UI is one **SvelteKit** application built as a static single-page app. The axum server or Tauri asset handler serves it, depending on the result of the lifecycle test (see [tauri.md](tauri.md)). The frontend renders state from the core and turns user actions into commands. Domain values are calculated in the core unless render performance requires client-side work. A data field therefore shows the same value on every platform and connected display.

Because the frontend speaks only the core's message protocol — and there is only one production transport, the axum server's HTTP + SSE surface (see [server.md](server.md)) — the same build runs inside the Tauri shell and in any browser, or against a mocked client in component tests.

**Nothing that must survive a restart lives in browser-origin storage** such as localStorage, IndexedDB, or OPFS. The embedded server may use a different origin after each start. Shared flight state belongs to the core. Saved layouts and other per-display settings live in Rust-side display-profile storage. The frontend owns only temporary presentation state such as the viewport, open dialogs, and unfinished edits.

## Stack

- **SvelteKit** with `adapter-static`
- **Svelte 5** runes
- **MapLibre GL JS** for the map, integrated via **svelte-maplibre-gl**
- **Paraglide JS** for i18n (see _i18n_ below)
- **UnoCSS** for icons, undecided for CSS

## State Model

A single `UpdraftClient` TypeScript interface wraps the transport (HTTP requests plus the SSE state stream). There is only one production implementation, because there is only one transport; the interface exists so tests can substitute a fake client. The stream client handles errors explicitly and surfaces **data age** — a stalled or dead stream must show as staleness in the UI, never as a silently frozen map.

On top of it, thin reactive stores: each change group becomes a rune-backed store (`$state` updated by the stream decoder, seeded from the snapshot), and components consume them declaratively. No component ever talks to the transport directly, so the whole UI is testable against a fake client. Commands are async functions with generated types. Optimistic UI only where harmless (e.g. settings toggles).

**Moving objects:** the core publishes the ownship and traffic as kinematic state vectors (see [core.md](core.md#outputs)). The frontend uses these values to estimate the render position between updates. It writes the position straight to the map at frame rate, outside Svelte reactivity. Smooth rendering therefore does not need frame-rate messages from the core.

## Map

One fullscreen MapLibre instance is the app's centerpiece. Layers, bottom-up:

1. vector basemap + hillshade/terrain (offline PMTiles)
2. airspace fills/lines with altitude-band filtering
3. waypoints/airfields with declutter
4. glide-reach polygon
5. own track
6. traffic symbols
7. ownship symbol

Bulk map data such as tiles and overlays arrives through resource URLs, never through the state stream (see [runtime.md](runtime.md#resource-storage)).

The map is integrated via [svelte-maplibre-gl](https://github.com/MIERUNE/svelte-maplibre-gl): declarative source/layer components for the stack above, with the raw map and sources reachable through `bind:map`/`bind:source` for the hot path. `<MapLibre>` must set `autoloadGlobalCss={false}` and import the MapLibre CSS locally, because the default loads it from a CDN at runtime, which violates offline-first.

Each source lives in one component with its layer(s) nested inside it. This keeps their add/remove order correct under HMR: hot-swapping a standalone source component would remove it while a separate layer still referenced it, which MapLibre rejects.

## Interaction Model

- **Tap on map opens a "What's here?" dialog:** a list of everything at or near the tap point (touch radius in px, converted to meters). Stacked airspaces with altitude bands, nearby waypoints/airfields, traffic, task points. Tapping an entry opens a detail dialog (airspace limits/class, airfield frequencies/runways/elevation, …). Hit-testing runs **Rust-side** via `query_at`, so results are consistent with the core's data rather than MapLibre's rendered-feature state. List items keep updating in real time. For example, a traffic symbol may be rotating, or distance values in the description may update.
- **Dialogs, not bottom sheets.** Every secondary surface is a dialog: a centered modal on large screens, automatically fullscreen on small screens, from one responsive component with a consistent header (title + back/close).
- **A structured settings tree.** Settings form a nested hierarchy (Flight / Map / Airspace / Devices / Units / System …), LX-9000-style. Fullscreen pages with back navigation on mobile, master-detail on wide screens. Search across all settings from the top level.
- **Map behavior modes:** north-up and track-up orientation, auto-zoom (context-dependent zoom, e.g. zoom in while circling). The map is always freely pannable. Panning away from ownship shows a "return to position" button and snaps back on tap.
- **Data bar:** slim and opinionated at first, configurability comes later.
- **Glove- and turbulence-friendly targets:** minimum touch-target size on the order of 48px for all in-flight controls, generous hit space on map symbols, no action available only via long-press. Critical actions reachable with one thumb in turbulence.

## Platform Behaviors

Wake-lock while flying, fullscreen/immersive mode, safe-area handling, portrait and landscape both first-class, high-contrast day/night themes with auto-switch by sun position.

**Sunlight readability** is a distinct requirement beyond day/night themes: explicit contrast targets for the day theme (map colors, data bar, warning banners must remain readable in direct cockpit sunlight), an anti-glare palette, and possibly a dedicated high-contrast mode. Validated on real devices outdoors, not just on calibrated monitors.

**Audio is native, not Web Audio.** Warning sounds are played on the Rust/native side so they keep working with the screen off or the app backgrounded (see [tauri.md](tauri.md)). The frontend may _trigger_ non-critical UI sounds, but nothing safety-relevant depends on the webview being alive.

**First-run disclaimer:** Updraft is not a certified navigation source. A first-run dialog and about-screen text state this explicitly.

## i18n

Launch languages are English and German. i18n scaffolding (Paraglide JS, tree-shaken, type-safe message keys) does not need to exist before the first string is written, but it must be introduced before hundreds of untranslated strings accumulate.

## Open Questions

- **UnoCSS:** likely used for icons, undecided whether it is also used for CSS.
