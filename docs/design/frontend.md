# The Frontend

The UI is a single **SvelteKit** application built as a pure SPA (`adapter-static`, no SSR), served by Tauri's asset handler or by `updraft-server`. It contains presentation logic only: it renders application state and translates user interactions into commands. It does not compute domain values itself, unless strictly required for performance reasons. The number shown in a data field is computed in Rust, so it is identical on every platform and every connected device.

Because the frontend speaks only the application protocol, the same codebase runs inside the Tauri shell, served by the axum server, or against a fake client in component tests. Only the host binding differs per build (see _State Model_ below).

## Stack

- **SvelteKit** with `adapter-static`
- **Svelte 5** runes
- **MapLibre GL JS** for the map, integrated via **svelte-maplibre-gl**
- **Paraglide JS** for i18n (see _i18n_ below)
- **UnoCSS** for icons, undecided for CSS

## State Model

A single `UpdraftClient` TypeScript interface abstracts the host binding (Tauri IPC or HTTP plus state stream). The implementation is selected at **build time**, so each build contains exactly one binding and the other is tree-shaken away.

The state stream begins with a complete snapshot and then carries ordered change batches. Thin rune-backed stores apply those changes, and components consume them declaratively. No component talks to a host binding directly, so the whole UI is testable against a fake client. Commands are async functions with generated types. Optimistic UI is limited to harmless interactions such as settings toggles.

**Hot path exception:** ownship position updates bypass Svelte reactivity and write straight to the map at frame rate. Reactivity is for UI chrome, not the 10 Hz path.

## Map

One fullscreen MapLibre instance is the app's centerpiece. Layers, bottom-up:

1. vector basemap + hillshade/terrain (offline PMTiles)
2. airspace fills/lines with altitude-band filtering
3. waypoints/airfields with declutter
4. glide-reach polygon
5. own track
6. traffic symbols
7. ownship symbol

Bulk geodata arrives as opaque resource references. `UpdraftClient` resolves them to host-specific map-source URLs, and the geometry never enters JSON state messages (see [core.md](core.md)).

The map is integrated via [svelte-maplibre-gl](https://github.com/MIERUNE/svelte-maplibre-gl): declarative source/layer components for the stack above, with the raw map and sources reachable through `bind:map`/`bind:source` for the hot path. `<MapLibre>` must set `autoloadGlobalCss={false}` and import the MapLibre CSS locally, because the default loads it from a CDN at runtime, which violates offline-first.

Each source lives in one component with its layer(s) nested inside it. This keeps their add/remove order correct under HMR: hot-swapping a standalone source component would remove it while a separate layer still referenced it, which MapLibre rejects.

## Interaction Model

- **Tap on map opens a "What's here?" dialog:** a list of everything at or near the tap point (touch radius in px, converted to meters). Stacked airspaces with altitude bands, nearby waypoints/airfields, traffic, task points. Tapping an entry opens a detail dialog (airspace limits/class, airfield frequencies/runways/elevation, …). Hit-testing runs **Rust-side** via `query_at`, so results are consistent with authoritative data rather than MapLibre's rendered-feature state. List items keep updating in real time. For example, a traffic symbol may be rotating, or distance values in the description may update.
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
