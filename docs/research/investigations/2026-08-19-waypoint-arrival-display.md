# Waypoint and Arrival-Altitude Display on MapLibre

Status: Technical investigation

This investigation examined how Updraft can render waypoints on the MapLibre
map and update arrival-altitude labels about once per second. It compared
update strategies with prototypes on 2026-08-19. It informs the
`waypoints-on-map` and `arrival-heights` roadmap items. It does not define
symbology, declutter rules, or the final-glide model.

## Environment

- Repository commit `c4bf997` with `maplibre-gl` 6.3.0.
- Browser measurements: headless Chromium 141.0.7390.37 through Playwright
  with SwiftShader software rendering, a 900 × 620 px map, and a local
  static server for the library, glyphs, and data. No basemap layers.
- Rust measurements: `rustc` 1.97.1 release builds of `updraft_geo`,
  `updraft_polar`, and `serde_json` on the same container (4 CPUs, x86-64).

The container CPU is desktop-class and the GPU path is software. Use the
numbers to compare strategies, not as absolute device performance.

## Conclusion

Split the data into a static and a dynamic map source:

1. Serve the full waypoint set as one static GeoJSON resource through
   `updraft://`, with a generation value, like the airspace dataset. A
   GeoJSON source loads 40,000 waypoints in about 1.3 s and costs nothing
   afterwards while its data does not change.
2. Render arrival altitudes from a second, dynamic GeoJSON source that
   carries only landable points. The per-update cost then scales with
   the dynamic source size and the visible symbols, not with the full
   waypoint set.
3. Update the dynamic source by URL, not through a topic: the frontend
   calls `setData()` once per second with an `updraft://` arrivals URL
   that carries the viewport bounds and a cache-busting timestamp as
   query parameters. The shell computes the response on demand from a
   core snapshot query. This measured equal to or better than pushing
   the same features through JavaScript, it keeps the main thread flat
   (the map worker fetches and parses the response), and it gives
   viewport scoping for free.
4. Compute arrival altitudes in Rust. The computation is not the
   bottleneck: a wind-corrected glide solution for 4,000 landables takes
   about 4 ms with exact geodesics. The payload is the quantity to
   bound, and viewport scoping bounds both it and the MapLibre update
   cost.

Three MapLibre constraints shape this design:

- `feature-state` cannot drive text. MapLibre rejects it for layout
  properties with: `"feature-state" data expressions are not supported
  with layout properties.` Label text must arrive through source data.
  `feature-state` remains the cheapest channel for paint-only updates
  such as reachability coloring.
- `updateData()` diffs are not a shortcut for this workload. Their cost
  grows with the retained source size and exceeded a full `setData()` of
  a small dedicated source in every measured case.
- With default symbol fading, every label whose text changes blinks: it
  disappears for about 300 ms and then fades back in over `fadeDuration`
  (default 300 ms). At a 1 Hz refresh the labels would be blank or
  half-faded for most of each second. The arrival-label layer must
  suppress this, either with map-wide `fadeDuration: 0` or with
  `text-allow-overlap: true` on the layer. Both options produced a
  same-frame swap with no gap (see "Label fade on text change").

## Requirements examined

- Waypoint sets change only on file import or removal. Realistic regional
  files hold 2,000–15,000 waypoints. 40,000 covers merged multi-country
  sets. 100,000 appeared only in the Rust scaling checks.
- A subset (landables, mountain passes) shows a value label next to the
  symbol. The value derives from ownship state and must refresh about
  once per second. The worst case changes every label every second.
- Labels are text rendered by MapLibre symbol layers with normal
  collision behavior. `text-field` is a layout property.

## Method

A standalone page generated deterministic waypoints over a 600 × 440 km
box (40% landables), one symbol layer for all waypoints (icon plus name)
and one for arrival labels. Each strategy ran 9 update rounds in which
every arrival label changed. The URL-update strategy requested a local
HTTP endpoint that generated and serialized the arrival collection on
demand per request, as a stand-in for a Rust `updraft://` resource
handler. The measurement did not include the Tauri scheme.

Each round measured the blocking time of the update call, the time from
the call to the next `idle` event (worker re-tiling, symbol layout, and
placement complete), and the longest animation frame. The tables report
the median time to `idle` at map zoom 9 unless stated. The timing
scenarios ran with `fadeDuration: 0`. A separate test examined the fade
behavior. It sampled pixels in each frame and preserved the drawing
buffer. The Rust benchmark timed the per-second computation and JSON
payloads for the same generated data.

## Measurements

### MapLibre initial load

Time from `addSource` plus `addLayer` to `idle` for the full waypoint set
in one GeoJSON source:

| Waypoints | Zoom | Load to idle |
| --------- | ---- | ------------ |
| 2,000     | 9    | 0.50 s       |
| 10,000    | 9    | 0.54 s       |
| 10,000    | 6.5  | 0.93 s       |
| 40,000    | 9    | 1.35 s       |

An earlier 42 s figure for 40,000 waypoints was a quadratic bug in the
benchmark's own data preparation, not MapLibre cost. A CPU profile located
the bug. The corrected generator produced the numbers above.

### MapLibre per-second update strategies

Median time to `idle` per update. "Dynamic" is the feature count the
update rewrites. 2,000 waypoints have 806 landables, 10,000 have 4,047,
and 40,000 have 16,012.

| Strategy | 2,000 | 10,000 | 40,000 | 10,000 at zoom 6.5 |
| -------- | ----- | ------ | ------ | ------------------ |
| One source, full `setData` | 180 ms | 346 ms | 1,214 ms | 540 ms |
| Split source, `setData` all landables | 79 ms | 167 ms | 421 ms | 339 ms |
| Split source, `setData` viewport only | 69 ms (92 dynamic) | 82 ms (445) | 208 ms (1,747) | 307 ms (3,799) |
| One source, `updateData` all landables | 107 ms | 245 ms | 2,431 ms | 511 ms |
| One source, `updateData` 10% of landables | 80 ms | 194 ms | 810 ms | 408 ms |
| `setFeatureState` all landables (paint only) | 32 ms | 40 ms | 50 ms | 52 ms |

The update calls themselves blocked the main thread for at most 9 ms for
full `setData` with 40,000 waypoints. The remaining work runs in the map
worker and in placement. Observations:

- The split source wins at every scale. Its cost tracks the dynamic
  source size, and viewport scoping caps that size. The 40,000-waypoint
  viewport case carried 1,747 features only because the prototype used a
  generous 0.5° overscan margin.
- `updateData` cost grows with the retained source size even for small
  diffs. At 40,000 waypoints a 10% diff cost twice a full split-source
  rewrite, and a 100% diff cost twice a full `setData`. `updateData`
  fits the traffic source with tens of features. It does not fit bulk
  per-second property updates on a large source.
- Every source-data strategy pays a shared floor of roughly 70–100 ms in
  this environment: MapLibre re-runs global symbol placement over all
  visible symbols after any source change. The zoom 6.5 column shows the
  floor growing with visible symbols (about 320 visible), which makes
  zoom-dependent label gating part of the performance design, not only a
  declutter choice.
- `setFeatureState` avoids re-tiling and placement entirely and stays
  near-constant, but it can only feed paint properties.

### Label fade on text change

MapLibre identifies a symbol across data reloads by the murmur3 hash of
its rendered text plus its rounded anchor position
(`CrossTileSymbolIndex`, where the symbol-layout key is
`murmur3(shaping.text)`). A reloaded feature with unchanged text and
position is therefore the same symbol. It keeps its placement opacity.
A feature whose label text changed is a new symbol. The old one vanishes
with the replaced tile. The new one starts at opacity zero until the next
placement pass. It then fades in over the map's `fadeDuration`.

A pixel-sampling test measured the summed text darkness in a crop box
around one arrival label on every frame after a `setData()` that
changed the label values, and around one static waypoint name from the
untouched source:

| Configuration | Behavior of the changed arrival label |
| ------------- | ------------------------------------- |
| `fadeDuration: 300` (default), default collision | Old text visible until about 150 ms, fully blank from about 200 ms to 450 ms, new text fades in and reaches full opacity at about 800 ms. |
| `fadeDuration: 300`, unchanged label text | No change in any frame. Cross-tile matching keeps the symbol stable through the reload. |
| `fadeDuration: 0`, default collision | Old text replaced by the new text within one frame. No blank frame. |
| `fadeDuration: 300`, `text-allow-overlap: true` on the label layer | Old text replaced by the new text within one frame. No blank frame. |
| `fadeDuration: 300`, `text-ignore-placement: true` only | Still blank from about 115 ms to 500 ms. `text-ignore-placement` alone does not help. |

`text-allow-overlap` is the operative flag: it forces the symbol to be
placed immediately, so it skips the collision-driven placement pass that
otherwise starts at opacity zero. `text-ignore-placement` controls only
whether the symbol blocks others, so it neither causes nor cures the
blank interval.

The static waypoint name from the other source stayed pixel-identical
in every configuration: updating one source does not disturb the other
source's symbols until their collision relationships change.

The behavior is known upstream and has no per-layer fix in the style
specification:

- The fade-on-`setData` behavior dates to the introduction of global
  symbol placement and the cross-tile index in Mapbox GL JS 0.42
  ([mapbox/mapbox-gl-js#5716](https://github.com/mapbox/mapbox-gl-js/issues/5716),
  [#5730](https://github.com/mapbox/mapbox-gl-js/issues/5730)). An
  equivalent MapLibre report is open
  ([maplibre/maplibre-gl-js#6531](https://github.com/maplibre/maplibre-gl-js/issues/6531)).
- A MapLibre maintainer recommends the allow-overlap workaround for
  per-second `setData` updates
  ([maplibre/maplibre-gl-js discussion #6695](https://github.com/maplibre/maplibre-gl-js/discussions/6695)),
  which matches the measurement above. `fadeDuration` remains a global
  map option. No per-layer fade control exists in the style
  specification.
- An independent inspection of a deployed production MapLibre soaring
  platform (WeGlide, checked 2026-08-19) found the same choice: its
  dynamic arrival-height label layer sets `text-allow-overlap: true`
  and keeps the default `fadeDuration`, on a separate in-memory GeoJSON
  source refreshed by `setData()`. No feature id or feature-state path
  is involved. This corroborates the per-layer flag over global
  zero-duration fading. (Its arrival labels update during flight
  replay, not on a live 1 Hz path, but the changing-text case is the
  same.)
- The index matches by text and position by design, not by feature id
  (documented in the
  [collision-detection design notes](https://github.com/mapbox/mapbox-gl-native/wiki/Collision-Detection)).
  Its purpose is visual continuity of an unchanged label across zoom
  transitions. Feature ids are optional in both vector tiles and
  GeoJSON, one feature can produce many symbol instances (repeated
  line labels, tile-boundary clones), and under the index's goal a
  changed text is a different label that should fade in. Id-based
  matching for value labels would be an upstream feature, not a
  configuration.

Consequences:

- The default configuration is unusable for per-second value labels.
  At 1 Hz the blink cycle repeats continuously.
- `fadeDuration: 0` is the map-wide fix. It also removes basemap label
  fading from all symbol layers. It does not change raster cross-fading
  ([MapLibre `MapOptions`](https://maplibre.org/maplibre-gl-js/docs/API/type-aliases/MapOptions/#fadeduration)).
- `text-allow-overlap: true` on the arrival layer is the per-layer fix,
  and the minimal one: such symbols skip the placement opacity
  animation. The trade-off is that arrival labels no longer declutter
  against each other or other labels in dense areas. Leaving
  `text-ignore-placement` at its default keeps the labels in the
  collision index, so they still displace lower-priority basemap
  labels rather than sitting under them.
- Values that do not change do not flicker, so rounding the displayed
  value (for example to 10 m steps) reduces churn but cannot replace
  the fix: a climb or sink changes every label in the same second.

### Push against pull for the dynamic source

A separate session compared the two transports for the split dynamic
source: `setData()` with a feature collection built in JavaScript
(push), and `setData()` with a URL that an on-demand endpoint answers
(pull). Same-session pairs, median time to `idle` per update:

| Variant | 10,000 | 40,000 | 10,000 at zoom 6.5 |
| ------- | ------ | ------ | ------------------ |
| Push, all landables | 249 ms | 566 ms | 359 ms |
| Pull by URL, all landables | 183 ms | 506 ms | 353 ms |
| Push, viewport only | 100 ms | 187 ms | 333 ms |
| Pull by URL, viewport only | 90 ms | 189 ms | 348 ms |

The rendering cost is the same within noise: re-tiling and placement
dominate, and both variants feed the same worker. The pull variant
additionally keeps the main thread flat (its `setData(url)` call blocked
for 0.1 ms at every size, while building and handing over 16,012
features as objects blocked for 3–6 ms), because the map worker fetches
and parses the response without the data ever existing as JavaScript
objects. The pull response also includes the endpoint's on-demand
generation and serialization, so the parity is conservative.

### Rust computation and payloads

Per-second work for a wind-corrected arrival altitude (MacCready speed to
fly against the along-bearing wind component, LS8 polar) for every
landable, plus payload sizes:

| Waypoints (landables) | Geodesic distance+bearing | Haversine fast path | Viewport filter + geodesic | JSON all landables | JSON viewport |
| --------------------- | ------------------------- | ------------------- | -------------------------- | ------------------ | ------------- |
| 2,000 (806)     | 0.9 ms  | 0.14 ms | 0.01 ms (16 in view)   | 26 KiB   | 0.5 KiB |
| 10,000 (4,047)  | 4.4 ms  | 0.7 ms  | 0.10 ms (89 in view)   | 133 KiB  | 2.8 KiB |
| 40,000 (16,012) | 16.8 ms | 2.9 ms  | 0.43 ms (327 in view)  | 539 KiB  | 10.7 KiB |
| 100,000 (39,814)| 43.3 ms | 7.7 ms  | 1.25 ms (880 in view)  | 1,347 KiB| 28.9 KiB |

Serializing the full static waypoint `FeatureCollection` (the one-time
resource response per generation) took 73 ms and 1.9 MB at 10,000
waypoints, and 291 ms and 7.6 MB at 40,000.

The exact WGS84 geodesic costs about 0.4 µs per waypoint and fits the
one-second budget at every realistic size, so the haversine fast path is
an optimization to keep in reserve, not a requirement. The viewport
filter makes computation trivial and, more importantly, keeps the
per-second IPC payload in the low kilobytes. Deltas do not replace
viewport scoping here: a climb shifts every arrival altitude together,
so "only changed values" degenerates to "all values" exactly when the
pilot climbs.

## Options compared

1. **One GeoJSON source, full `setData` per second.** Simplest frontend:
   one source, one topic with every waypoint. Rejected: it re-tiles the
   complete set every second (1.2 s per update at 40,000), and it moves
   the static data out of the `updraft://` resource path into the topic
   channel, against the current architecture.
2. **Split static resource plus dynamic arrival source, pushed through a
   topic.** The static source follows the proven airspace pattern
   (`updraft://localhost/waypoints.geojson?v=<generation>`). A 1 Hz
   topic carries id, position, and `arrivalMeters` for landables. A
   frontend store rebuilds the dynamic source with `setData()` and
   formats labels with the unit settings, like traffic altitude labels.
   This option needs a new topic, generated types, and a store. Viewport
   scoping also needs a frontend-to-core input.
3. **Split static resource plus dynamic arrival source, pulled by URL.**
   Chosen. The frontend timer calls
   `setData('updraft://localhost/arrivals.geojson?bbox=…&t=…')` about
   once per second. The shell resource handler takes a core snapshot
   (waypoints in the bounds plus current glide inputs), computes the
   arrival altitudes, and serializes the collection, like the airspace
   resource does per generation. Measured equal to or better than
   option 2, with a flat main thread. Viewport scoping needs no new
   protocol because the bounds are part of the request. MapLibre does not
   cancel an active request when the next `setData()` call occurs. It
   applies the active result first and then applies the newest pending
   update. It discards older pending updates
   ([MapLibre 6.3 `GeoJSONSource`](https://github.com/maplibre/maplibre-gl-js/blob/v6.3.0/src/source/geojson_source.ts#L399-L490)).
   The frontend owns the update cadence. A throttled timer in a hidden
   webview pauses updates. This is acceptable because the map is not
   visible then. A failed request leaves the previous labels until the
   next tick. The computation must stay a core-owned query. Later
   consumers can then reuse it instead of duplicating it in the shell.
4. **One source plus `updateData` property diffs.** Attractive on paper
   (no duplicated geometry, one source for hit-testing). Rejected by
   measurement: diff application scales with the retained source size
   (see table), so it is the worst strategy exactly where efficiency
   matters.
5. **`feature-state` for the per-second values.** Rejected as the label
   mechanism because layout properties cannot read feature state.
   Retained as a complement: reachability coloring of static waypoint
   icons through paint properties costs about 40 ms per update with no
   re-tiling and needs no duplicated data.
6. **Vector tiles from the shell.** Serving waypoints as pre-cut vector
   tiles through `updraft://` would remove the client-side GeoJSON
   indexing. Not needed: the measured initial load (1.35 s at 40,000)
   happens once per import, and per-second updates never touch the
   static source. Reconsider only together with the planned MBTiles
   basemap path if far larger datasets become a requirement.
7. **DOM markers.** Rejected without prototyping: one DOM element per
   waypoint bypasses MapLibre collision and symbol batching and does not
   scale to thousands of points.

## Suggested integration shape

- The core owns the waypoint store (`waypoint-db`) and publishes a
  status topic with a generation, like airspace. The shell serves
  `updraft://localhost/waypoints.geojson?v=<generation>` from the
  canonical store.
- The shell serves `updraft://localhost/arrivals.geojson?bbox=…&t=…` on
  demand. The handler sends one typed driver query for the landables in
  the bounds and the current glide inputs, then computes and serializes
  outside the driver task, like the airspace resource. The arrival
  calculation itself is a core-owned pure function so nearest lists and
  emergency mode reuse it. At the measured cost there is no reason to
  route this through the planned compute-worker path.
- The frontend keeps the two-source split. The static symbol layers read
  the versioned resource URL. An arrival-label symbol layer reads a
  small source that a 1 Hz timer refreshes with `setData(url)`, passing
  the current viewport bounds with an overscan margin. The response
  carries `arrivalMeters`. The shell can format the label text from the
  core-owned unit settings. The frontend can instead convert the value
  in the style expression.
- Gate name and arrival labels by zoom. Placement cost scales with
  visible symbols, so the label gating threshold is also the update-cost
  control for zoomed-out views.
- Suppress symbol fading for the arrival labels: set
  `text-allow-overlap: true` on the arrival layer (the minimal fix), or
  set `fadeDuration: 0` on the map. Without one of these, every changed
  label blinks for most of a second on each refresh. The per-layer flag
  is preferable because it leaves basemap label fading untouched.
  Decide map-wide `fadeDuration` once, together with the future
  basemap.
- Viewport scoping comes with the URL pull for free. Requesting all
  landables without a `bbox` stays acceptable below roughly 2,000
  landables, so the parameter can also be omitted until profiling on
  target hardware demands it.

## Limits

- Software rendering and a desktop-class CPU. Android WebView numbers
  will differ. The strategy ranking should hold because it follows from
  where MapLibre does the work (worker re-tiling, global placement), not
  from device speed. Verify the placement floor on target hardware.
- The benchmark style had no basemap layers. A real basemap adds its own
  symbols to every placement pass, which raises the shared floor and
  strengthens the case for the split source and label gating.
- Medians of 9 update rounds per scenario in one browser session.
  Variance between sessions was within about 30–50%, so only same-run
  numbers compare strategies directly.
- The pull variant used a local HTTP endpoint. The production path adds
  the Tauri custom-scheme hop (an in-process async dispatch, already
  used per airspace load). The test did not measure this hop. It adds one
  request per second.
- The fade test sampled frames under software rendering, so its
  millisecond boundaries are approximate. The blank interval and the
  fade-in follow from the cross-tile matching and placement design, so
  the qualitative behavior does not depend on the device. The
  mitigations were verified in the same test, not in the application.
- The arrival-altitude formula was a stand-in (no terrain, no safety-MC
  variants). It only had to make the per-waypoint cost realistic.
- `maplibre-gl` 6.3.0 behavior. Recheck the `updateData` scaling and the
  `feature-state` layout restriction before reusing these conclusions on
  a different major version.
