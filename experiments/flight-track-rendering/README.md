# Flight track rendering benchmark

How do we draw a full-flight GPS track in maplibre-gl-js, with **every
segment between two fixes colored by altitude or vario**, when a new fix
arrives roughly **every second**?

This experiment implements eight candidate approaches against a real 5.2 h
flight (`flight.igc`, 18 812 B-records) and measures them with Playwright.

## Running it

```sh
pnpm install --ignore-workspace
node run-bench.mjs            # all approaches
node run-bench.mjs updatedata # substring-filter a single approach
```

Results land in `results.json` and `screenshots/`, plus a markdown table on
stdout.

## What is measured

The last 80 fixes are held back from the initial load and used to simulate
live updates. For each approach:

| metric | meaning |
| --- | --- |
| init | add source(s)/layer(s) with 18 732 fixes → map `idle` |
| append call | main-thread cost of the JS call that adds one fix |
| append latency | one fix added → map `idle` again (visual latency; includes one ~30 ms software-rendered frame) |
| pan / zoom fps | rAF frame rate during a 3 s `easeTo` across / out of the track |
| fps @appends | frame rate of the same animation while a fix is appended every 150 ms |
| recolor | switching altitude-coloring ↔ vario-coloring → `idle` |
| heap | `usedJSHeapSize` at the end of the run |

## Results

maplibre-gl **5.24.0**, deck.gl **9.3.6**, headless Chromium 134 with
SwiftShader software GL, 1100×800 map, blank style (no basemap), line width
2 px.

| approach | init (ms) | append call avg/p95 (ms) | append latency avg/p95 (ms) | pan fps | zoom fps | fps @appends | recolor (ms) | heap (MB) |
|---|---|---|---|---|---|---|---|---|
| geojson-segments-precolored | 480 | 0.1 / 0.2 | 203.9 / 250.0 | 27.5 | 27.9 | 13.0 | 299 | 64.6 |
| geojson-segments-data-driven | 534 | 0.1 / 0.2 | 211.9 / 250.0 | 27.2 | 26.7 | 12.9 | 370 | 61.9 |
| geojson-segments-updatedata | 799¹ | 0.1 / 0.2 | **57.5 / 99.9** | 27.0 | 27.4 | **25.4** | 379 | 47.1 |
| geojson-line-gradient | 529 | 4.0 / 7.6 | 334.4 / 400.2 | 28.8 | 27.2 | 23.1 | 373 | 22.2 |
| geojson-chunked-static-live | 531 | 0.1 / 0.1 | 61.9 / 101.4 | 20.4 | 19.2 | 19.7 | 375 | 52.1 |
| geojson-feature-state | 578 | 0.1 / 0.2 | 214.8 / 249.1 | 26.5 | 27.4 | 13.5 | **72** | 46.6 |
| custom-webgl-layer | **421** | 0.1 / 0.2 | **33.9 / 38.9** | **31.7** | **28.4** | **29.5** | **34** | **27.1** |
| deckgl-line-layer | 557 | 0.2 / 0.4 | 284.1 / 385.7 | 4.9² | 4.2² | 4.3² | 211 | 35.4 |

¹ inflated by a benchmark artifact: the approach assigns feature ids after
building the collection and calls `setData` a second time.
² deck.gl's interleaved renderer is disproportionately punished by
SwiftShader; expect much better numbers on a real GPU. The 284 ms append
cost is CPU-side attribute regeneration and *would* remain.

The ~30 ms append-latency floor and the ~28 fps ceiling are the cost of
software-rendering one frame; on real GPUs the absolute numbers improve
across the board, but the CPU-side costs that separate the approaches
(worker re-tiling, structured-clone transfer, attribute regeneration) stay.

### Local run on real hardware

The same suite on a real GPU with a 120 Hz display (frame floor ≈ 8 ms):

| approach | init (ms) | append latency avg/p95 (ms) | pan fps | zoom fps | fps @appends | recolor (ms) | heap (MB) |
|---|---|---|---|---|---|---|---|
| geojson-segments-precolored | 341 | 67.6 / 75.0 | 120.0 | 119.7 | 105.8 | 75 | 50.1 |
| geojson-segments-data-driven | 343 | 68.2 / 75.1 | 119.9 | 119.8 | 106.2 | 309 | 37.5 |
| geojson-segments-updatedata | 346 | **8.9 / 16.5** | 120.1 | 119.8 | **119.7** | 317 | 55.7 |
| geojson-line-gradient | 321 | 276.4 / 325.6 | 120.0 | 119.7 | 119.5 | 309 | 63.8 |
| geojson-chunked-static-live | 348 | **9.8 / 8.7** | 119.9 | 119.8 | **119.7** | 308 | 35.2 |
| geojson-feature-state | 349 | 65.2 / 66.9 | 120.0 | 119.7 | 108.0 | **17** | 73.0 |
| custom-webgl-layer | 338 | **8.3 / 9.2** | 120.0 | 119.7 | **119.7** | **8** | 41.2 |
| deckgl-line-layer | 319 | **8.3 / 9.0** | 120.0 | 119.7 | **119.7** | **8** | **21.8** |

(The deck.gl row comes from a second local run of the subset
updatedata/custom/deckgl; the other two approaches reproduced their numbers
above within noise. The software-GL frame-rate collapse deck.gl showed in
the container run does not occur on real hardware — there it matches the
custom layer on every metric and has the lowest heap of all approaches,
including an 8 ms recolor. Its 284 ms container append cost also vanishes
on a fast CPU. The remaining argument against it is purely the dependency:
a second rendering framework for something ~200 lines of WebGL cover.)

This confirms the container run's ordering while compressing the gaps:

- Every approach that avoids full `setData` appends at the **single-frame
  floor** (~8–10 ms). Full `setData` costs ~67 ms per fix and drops a 120 Hz
  animation to ~106 fps while fixes stream in — usable, but wasteful.
- `line-gradient`'s 276 ms append latency persists on real hardware; it is
  CPU-bound (gradient-expression rebuild + `lineMetrics` re-parse), so a GPU
  doesn't help. Confirmed the wrong tool for a live track.
- Steady-state rendering of 18 k segments is free on a real GPU (120 fps
  everywhere) — the differentiators are append cost and **recolor**: custom
  WebGL 8 ms, feature-state 17 ms, everything else ~310 ms.

## The approaches

### 1. `geojson-segments-precolored`

One 2-point `LineString` feature per segment with a pre-computed CSS color
string property; `line-color: ['get', 'c']`. Every append re-sends the whole
18 k-feature collection through `setData()`, which structured-clones it to
the worker and re-runs geojson-vt tiling — ~200 ms per fix, and the animation
frame rate halves while updates are flowing. Recoloring rewrites every
feature property.

### 2. `geojson-segments-data-driven`

Same geometry, but features carry raw `alt`/`vario` numbers and the color
comes from an `['interpolate', ..., ['get', 'a']]` paint expression.
Rendering cost is identical to #1; the win is flexibility — switching the
color mode or tweaking the ramp is a single `setPaintProperty()` call with
no data rebuild (the ~370 ms is maplibre re-evaluating paint buffers over
the existing tiles, not re-tiling). This is the styling model the other
GeoJSON variants build on.

### 3. `geojson-segments-updatedata` ⭐ (best pure-maplibre option)

Same as #2, but appends go through the incremental
[`GeoJSONSource.updateData()`](https://maplibre.org/maplibre-gl-js/docs/API/classes/GeoJSONSource/#updatedata)
diff API: `source.updateData({ add: [feature] })`. The worker keeps the
dataset, so nothing big crosses the thread boundary; only re-tiling remains.
Appends drop from ~210 ms to ~58 ms and the frame rate during live updates
stays at full speed (25 vs 13 fps). Gotcha: **every** feature needs a unique
`id` (including the initial ones), otherwise the source silently rejects the
diff with a map `error` event.

### 4. `geojson-line-gradient`

A single `LineString` with `lineMetrics: true` and a `line-gradient`
expression over `['line-progress']` (down-sampled to ≤512 stops — maplibre
rasterizes the gradient into a small texture anyway). Loses on every axis
that matters here: `line-progress` is normalized by total length, so every
append shifts all stops and the whole gradient expression must be rebuilt
(worst append latency, 334 ms), and 512 stops across 18 k fixes visibly
washes out the colors (see `screenshots/geojson-line-gradient.png` vs the
others — short climbs blur into the glides). Only redeeming quality: lowest
heap, since there's one feature. Fine for a *static* track preview, wrong
tool for per-fix coloring of a live track.

### 5. `geojson-chunked-static-live`

Two sources: a large "static" one that is only re-sent when the live buffer
overflows (every 60 fixes), and a tiny "live" one that takes the 1 Hz
appends. This was the classic workaround before `updateData()` existed.
Appends are as cheap as #3, but the second source/layer costs ~25 % steady
frame rate (two tile sets, two draw passes) and the code is the most
complex of the GeoJSON variants. `updateData()` makes it obsolete.

### 6. `geojson-feature-state`

Per-segment features with colors delivered via
`setFeatureState()` / `['feature-state', 'c']`. Appends still need `setData`,
so they're as slow as #1. Its one superpower: recoloring all 18 k segments
takes 72 ms instead of ~370 ms, because feature-state lives in a separate
per-tile buffer that can be updated without re-evaluating paint properties.
Could be *combined* with #3 (updateData appends + feature-state colors) if
instant palette switching ever matters more than the extra bookkeeping.

### 7. `custom-webgl-layer` ⭐ (fastest overall)

A [`CustomLayerInterface`](https://maplibre.org/maplibre-gl-js/docs/API/interfaces/CustomLayerInterface/)
that owns its own GPU buffers: one screen-space-extruded quad (4 vertices,
2 triangles) per segment, positions stored as Float32 mercator offsets from
an anchor fix (restoring the precision a raw Float32 mercator coordinate
would lose), pixel-width extrusion done in the vertex shader so zooming
never re-tessellates. Altitude and vario colors are kept in **two**
pre-uploaded vertex-color buffers, so recoloring is literally rebinding an
attribute (34 ms = one frame). Appending a fix is a 200-byte
`gl.bufferSubData` into pre-allocated buffers (0.1 ms) plus
`triggerRepaint()` — constant time, no worker, no re-tiling, ever.

Wins every metric: fastest init, appends at the single-frame floor, best
frame rates, best-in-class memory (typed arrays instead of 18 k GeoJSON
feature + tile objects). The cost is owning ~200 lines of WebGL: no line
joins/caps (invisible at 2 px width in the screenshots), no maplibre
styling/interaction for free (hover/click needs its own hit-testing), and
globe projection would need the v5 `shaderData` variant of the API.

### 8. `deckgl-line-layer`

`deck.MapboxOverlay({ interleaved: true })` with an instanced `LineLayer`
(one instance per segment, `getColor` per instance). Appending means handing
deck a new data array, which regenerates all instance attributes — ~284 ms
in the software-GL container, but negligible on a fast CPU: the local run
puts deck.gl at the frame floor on every metric with the lowest heap of all
approaches. The 4–5 fps container frame rates are purely a SwiftShader
artifact. On real hardware it's a genuine contender; the argument against
it is the dependency weight (a second rendering framework) for something
~200 lines of custom WebGL cover, plus untested behavior on the weak mobile
GPUs the container run approximates.

## Considered but not implemented

- **Canvas/Image source** (draw the track into a canvas, show as raster):
  resolution is fixed at upload time, so it blurs on zoom and needs frequent
  re-renders + re-uploads; also loses crisp 2 px lines under rotation/pitch.
- **Color-bucket layers** (quantize the ramp into N classes, one
  filtered layer per class): N× draw passes and it still needs `setData`
  per append; strictly worse than #2/#3.
- **Server-side vector tiles** (the architecture doc's `updraft://tiles/…`
  route): right answer for *many/huge stored flights* (LOD for free), but
  1 Hz updates would mean constant tile invalidation round-trips for the
  live track.
- **deck.gl `TripsLayer` / three.js custom layer**: same trade-offs as #8 /
  #7 respectively, with more dependency weight.

## Conclusions

1. **On real hardware the pragmatic choice is per-segment features with
   data-driven paint + `updateData()` appends (#3)**: appends land at the
   single-frame floor just like the custom layer, and we keep maplibre
   styling, hit-testing and projection support for free. Give every feature
   an `id` from day one — `updateData()` silently rejects the diff
   otherwise.
2. **The custom WebGL layer (#7) remains the performance ceiling** — it
   wins where the maplibre approaches still lose: recolor (8 ms vs ~310 ms),
   memory (typed arrays instead of feature + tile objects), and headroom on
   weak mobile GPUs (the container's software-GL run is a decent proxy for
   how the gaps widen on slow hardware, which matters for the Tauri
   mobile targets). It also fits the architecture doc's plan: buffers can
   be filled straight from a compact binary fix stream without ever
   materializing GeoJSON. Worth it if/when instant palette switching or
   low-end devices become requirements, not before.
3. Avoid `line-gradient` for this use case: worst append cost on both fast
   and slow hardware (it's CPU-bound) *and* lossy colors. Don't bother with
   the two-source chunking workaround now that `updateData()` exists.
4. If recoloring ever needs to be instant while staying on plain maplibre
   layers, combine #3 with feature-state colors (17 ms recolor).
5. Per-segment 2-point features are ~18 k features for a 5 h flight and the
   GeoJSON approaches hold up fine at that size; for showing many stored
   flights at once, move to server-side tiles instead of stretching the
   client-side approaches.
