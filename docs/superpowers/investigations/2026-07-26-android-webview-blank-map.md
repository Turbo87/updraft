# Blank map on Android cold start

Investigation of "the map does not paint on the first load of a fresh process on
Android". Conducted read-only at commit `5abaa08`, with the app installed from
`tauri/gen/android/app/build/outputs/apk/universal/debug/app-universal-debug.apk`.

**Headline: this is an Android WebView 113 defect, not our bug.** It reproduces
100% on `spike-api34` (WebView 113.0.5672.136) and 0% on API 35 (WebView 124) and
API 36 (WebView 134) with the same APK.

## Reproduction

```
adb shell am force-stop aero.updraft.debug
adb shell am start -n aero.updraft.debug/aero.updraft.MainActivity
```

Wait ~12 s, screenshot. Note the activity is `aero.updraft.MainActivity`, not
`.MainActivity` — `am start -n aero.updraft.debug/.MainActivity` fails.

- **API 34 (`spike-api34`): 13/13 cold starts blank.** Completely deterministic.
  "Fresh process" here means force-stop then launch; that is sufficient, no
  reinstall or reboot needed.
- **API 35 (`spike-api35`): 3/3 painted.** Not reproducible.
- **API 36 (`Medium_Phone_API_36.1`): 3/3 painted.** Not reproducible.

`location.reload()` in the webview fixes it permanently for that process, as
reported.

Blank/painted was scored mechanically: `adb exec-out screencap` (raw RGBA), then
the standard deviation of a sampled block over the map area. Blank gives sd 0.00
(uniform white); painted gives sd 25-43. Every verdict was also eyeballed — one
early API 35 "PAINTED" was actually the location-permission dialog, which is why
permissions are granted before scoring.

## Root cause

**Settled.**

On Android WebView 113, a `<canvas>` that acquires a WebGL context during a
window early in the *first* page load of a fresh WebView process is created
successfully, renders correctly, and is never composited. Its pixels never reach
the screen. MapLibre's canvas is created ~185-245 ms into the first load and
falls inside that window.

Evidence, in the order it was established:

1. **The network story in the original report was wrong.** MapLibre requests
   tiles normally and they return HTTP 200
   (`https://tiles.openfreemap.org/planet/20260621_080001_pt/11/1059/687.pbf`
   etc.). The "zero tiles" observation came from reading
   `performance.getEntriesByType('resource')` on the main thread; MapLibre
   fetches vector tiles inside its web worker, so those requests never appear
   there. Captured via CDP `Network.enable` plus `Target.setAutoAttach`, which
   sees worker requests.

2. **MapLibre renders fine.** Wrapping the WebGL context in a Proxy that counts
   calls, from a `Page.addScriptToEvaluateOnNewDocument` hook installed before
   the canvas exists: 81 frames, 8553 `drawElements` calls, first draw at 279 ms,
   zero shader compile failures, zero link failures, zero incomplete
   framebuffers. A swipe adds 76 more frames and 10 000 more draws — and still
   nothing appears.

3. **The failure is compositing, not drawing.** Taking MapLibre's own context and
   clearing it to opaque red every frame for 200 frames produces no visible
   change. Nothing drawn into that canvas can ever be seen.

4. **It is not the element, the layout or the stacking.** The canvas is
   412x915 CSS px at (0,0), `display:block`, `visibility:visible`, `opacity:1`,
   no transform/filter/mix-blend-mode, and `document.elementFromPoint(centre)`
   returns it. Every ancestor is opaque and visible.

5. **It is not our page or WebGL in general.** Injecting a 2-D canvas, a WebGL
   canvas and a plain div into the same broken page composites all three
   perfectly.

6. **The discriminator is *when the context is created*.** Creating identical
   WebGL canvases at a schedule of times after navigation start, each clearing to
   blue every frame, gives a hard boundary. Four independent runs:

   | run | dead (never composite) | alive | first-paint |
   |---|---|---|---|
   | 1 | 92, 123, 260 ms | 311, 421, 561, 863, 1007, 1403, 2002, 3004 ms | - |
   | 2 | 95, 118, 134, 184 ms | 357, 396, 413, 430, 474, 503, 606 ms | - |
   | 3 | 97, 125, 167 ms | 288, 347, 371, 388, 405, 445, 532, 701 ms | 194 ms |
   | 4 | 94, 105, 166 ms | 296, 522, 532, 539, 550, 576, 590, 716 ms | 172 ms |

   Dead canvases render the Chromium broken-content glyph. The boundary sits at
   roughly the document's first paint, i.e. the WebView's first composited
   frame. Both sides of the boundary occur *within a single page load*, which
   rules out anything about "first load vs second load" at the document level.

7. **The window is process-wide and closes once.** After a reload the map canvas
   is created at 128 ms — also before *that* document's first paint at 143 ms —
   and it composites fine. So it is not "before this document's first paint", it
   is "before the WebView has ever produced a frame". That is exactly why
   `location.reload()` cures it: by then the window has long since closed.

8. **Forcing the drawing buffer to be recreated cures it.**
   `gl.getExtension('WEBGL_lose_context').loseContext()` followed by
   `restoreContext()` on the map's context makes the map paint immediately, fully
   and permanently, including the ownship glider. MapLibre's own
   `webglcontextrestored` path rebuilds the painter and re-applies the style.
   This pins the broken state to the compositing resource created alongside the
   original drawing buffer, not to the element, the context object or MapLibre.

9. **It is version-bound.** Same APK, same host: API 34 / WebView 113 fails
   every time, API 35 / WebView 124 and API 36 / WebView 134 never fail.

The one thing that is *not* nailed down is the precise Chromium-internal reason
the layer is orphaned — that would need a WebView 113 debug build. Everything
above pins the behaviour, its boundary and its cure, and the version matrix makes
further Chromium archaeology low-value.

## Hypotheses tested and eliminated

| Hypothesis | Ruled out by |
|---|---|
| Network unreachable from the emulator / tiles not fetched | CDP worker-session capture: six tile requests, all HTTP 200, at +365-382 ms |
| `subscribe` fails silently on Android (milestone 1's flagged risk) | On the blank first load, `adb emu geo fix 8.5417 47.3769` moves the map centre to exactly 47.37690, 8.54170 in the debug overlay. Topics flow fine; the map is driven correctly, it just is not visible |
| Ownship fails while the basemap works, or vice versa | Neither — nothing on the canvas is visible. After the context-restore fix both basemap and ownship appear together |
| WebGL context lost during the splash-to-activity handoff | A `getContext` hook installed at document start logged the context creation and registered `webglcontextlost` / `webglcontextrestored` / `webglcontextcreationerror` listeners on it. None ever fired. `gl.isContextLost()` is `false` throughout |
| WebGL is broken in the process | A fresh WebGL canvas created 1.5 s and 6.6 s in clears to blue, reads back `[0,0,255,255]` and composites |
| Zero-sized container / missing resize (the classic MapLibre trap) | Canvas is 1081x2401 backing, 412x915 CSS, `transform.width/height` correct enough that MapLibre computes and requests the right tiles. `window.dispatchEvent(new Event('resize'))` changes nothing |
| Stale surface — frames composited to a surface discarded at splash dismissal, with the map then idle | A swipe produces 76 further frames well after startup; still blank |
| Compositing-layer nudge would recover it | `transform: translateZ(0)` and a `display:none` -> reflow -> `display:''` cycle both do nothing |
| The sticky `GL_INVALID_ENUM` (1280) on the context is the cause | Brand-new contexts that composite perfectly also report 1280. It is noise from the emulator's GL translator |
| The emulator's GL translator is the culprit | Attempted to force SwiftShader via `/data/local/tmp/webview-command-line`; WebView ignored the flag (renderer stayed "Android Emulator OpenGL ES Translator"), so this test is **inconclusive**. The version matrix is the stronger evidence: same translator on all three images, only WebView 113 fails |
| Splash-screen dismissal is the boundary | Splash exit landed at t+183 ms in one run and t+78 ms in another, while the boundary stayed near 200-300 ms. Correlated with first paint, not splash |
| A MapLibre bug | A plain blue-cleared canvas with no MapLibre involvement fails identically in the same window |
| Our code regressed it in milestone 1 | See below |

## Is this a regression?

**No evidence that it is.** The map has been MapLibre since `55da94b`
(2026-07-08, "frontend: Add MapLibre map page with online basemap"), the Android
target landed the next day in `06416a8`, and the only structural move was
`3f2acac` (2026-07-18, "frontend: Move map surface into root layout"). In a CSR
SvelteKit SPA the root layout and the `/` route page mount at effectively the
same moment, so that move does not meaningfully change when the canvas is
created. Nothing in milestone 1 plausibly moved map creation across the ~200 ms
boundary.

That this has not been seen before is well explained by the version
matrix rather than by a regression: it only happens on WebView 113, which is a
mid-2023 build shipped in the API 34 system image. Any physical device or newer
image would not show it.

I did not attempt to build and run historic revisions on API 34 to confirm
empirically — given the mechanism is a fixed ~200 ms window that our startup has
always been inside, it would almost certainly have been broken there too.

## Does it reproduce on desktop?

The desktop binary builds cleanly (`cargo build -p updraft_tauri --bin updraft`,
exit 0). I did not run a visual desktop check, because the mechanism cannot
apply: macOS Tauri uses WKWebView, a different engine with no Chromium layer
compositor, and the defect is specific to one Chromium version's handling of
WebGL canvas layers before the first frame. The same frontend also paints
correctly on Android on the second load and on API 35/36 on the first, so the app
code is not implicated.

## Upstream issue search

Searched `maplibre/maplibre-gl-js` for the signature (blank map, Android WebView,
canvas sized but nothing painted, cured by reload). **No matching issue.** The
closest hits are unrelated: [#4451](https://github.com/maplibre/maplibre-gl-js/issues/4451)
(tiles dropped when served from the Android asset folder, a worker origin check —
we serve from `http://tauri.localhost`, not `file://`),
[#1368](https://github.com/maplibre/maplibre-gl-js/issues/1368) and
[#4487](https://github.com/maplibre/maplibre-gl-js/issues/4487) (3-D terrain).

This repo pins `maplibre-gl@5.24.0` and `svelte-maplibre-gl@2.1.0`. No affected
range applies, because the defect is not in MapLibre — a canvas with no MapLibre
involvement fails identically.

The genuinely relevant prior art is
[visgl/react-map-gl#851](https://github.com/visgl/react-map-gl/issues/851)
(closed, 2019, Android WebView inside Unity). Same family, different symptom: for
that reporter `canvas.getContext('webgl')` returned **null** rather than a
working-but-uncompositable context. The reporter's diagnosis: *"this only happens
if the web view has been loaded, but not been displayed to the user yet ... Once
a page has been displayed (it does not matter which one), `canvas.getContext`
returns a valid WebGL context."* Their fix was to defer map construction — *"I
could workaround this problem by setting a timeout in my component that delays
rendering of the map. Even a timeout of 0 was sufficient."* That is the same
"canvas created before the WebView is on screen" hazard, and it validates the
defer-creation family of fixes.

## WeGlide Copilot

Nothing usable. WeGlide has a public GitHub org (`weglide`, 15 repos) but Copilot
itself is not among them — the repos are libraries and tooling (`aerofiles`,
`aeroscore-rs`, `casper`, `translation`, `protocol-rs`, a
`maplibre-3d-linestring-demo`). One useful inference: `weglide/capacitor-barometer`
is a Capacitor plugin, so Copilot is a web app in a native shell, architecturally
the same family as ours and almost certainly Android WebView + MapLibre. But
with no public source and no public write-up of their Android map setup, there is
nothing to copy. Negative result.

## Relationship to the task 6 bug

**Independent mechanisms, but they will collide in the same screenshot.**

Task 6 targets [tauri#15671](https://github.com/tauri-apps/tauri/issues/15671):
after the activity is destroyed and the app relaunched, `tauri-runtime-wry` drops
the mobile `Resumed` event when no windows exist, so no webview is created and
the screen is blank. That is a Rust-side event-routing bug — no webview, no page,
nothing loaded.

This bug is a Chromium compositing defect inside a fully working webview: the
page loads, the DOM renders, the network fetches, MapLibre draws 8553 triangles.
Evidence they are unrelated: this reproduces on a plain first cold start with no
activity destruction anywhere in the sequence, and it is confined to WebView 113
while the Tauri bug is version-independent Rust code.

They are trivially distinguishable in practice, and task 6 does not need this
fixed first:

- **Task 6's bug**: the whole page is gone. `adb shell cat /proc/net/unix | grep
  webview_devtools_remote` finds no socket, or the CDP target list is empty.
- **This bug**: the DevTools target exists, the page is
  `http://tauri.localhost/`, the language switcher and the MapLibre attribution
  render, and only the map area is white.

The cleanest control for task 6 is to run it on `spike-api35` or
`Medium_Phone_API_36.1`, where this bug does not exist at all. Failing that, "is
the attribution visible?" is a one-glance discriminator.

## Options to fix

**1. Do nothing; drop API 34 from the support matrix.** (size: zero code, plus a
doc line)

WebView 113 is from mid-2023. Google Play requires a current WebView on
essentially every live device, and WebView auto-updates independently of the OS,
so real API-34 phones ship WebView 130+. This is an artefact of a stale emulator
system image.
*Trade-off:* nothing changes for real users. *Risk:* if the defect also exists on
some real device with an old pinned WebView (kiosk hardware, some e-ink
devices — relevant given the project targets those), we ship a blank map there
and would not know. Unverified either way; I have no physical device here.

**2. Move the spike/CI emulator to API 35 or 36.** (size: small; AVD config plus
milestone doc)

Removes the confound from tasks 6-8 immediately and makes the acceptance
checklist honest. Complements option 1 rather than competing with it.
*Trade-off:* loses coverage of the oldest API level we nominally support.
*Risk:* if we later need API 34 coverage, this bug comes back as a surprise.

**3. Defer map creation until after the WebView's first frame.** (size: ~5 lines
in `frontend/src/lib/map/Map.svelte` or `FlightView.svelte`)

Gate the `<MapLibre>` render on a flag set after two `requestAnimationFrame`s
following `first-paint` (via `PerformanceObserver({type:'paint'})`). This is what
react-map-gl#851 landed on.
*Trade-off:* a real behavioural fix that costs a frame or two of startup and
needs no platform branch. *Risk:* it is a race against a Chromium-internal
condition with no exposed signal — my data shows `requestAnimationFrame` alone
firing at 112 ms while canvases created at 167 ms are still dead, so rAF is *not*
a sufficient signal. Any threshold we pick is empirical and could be wrong on
slower or faster hardware. It also adds startup complexity permanently for a
defect that only exists on one obsolete WebView.

**4. Recover after the fact via a forced context loss/restore.** (size: ~15 lines,
Android-only, gated)

On `map.on('load')`, once, call
`gl.getExtension('WEBGL_lose_context').loseContext()` then `restoreContext()`.
Verified working: the map paints immediately and completely.
*Trade-off:* deterministic rather than a timing race, and it self-heals whenever
the canvas is in the bad state. *Risk:* it is a deliberate context loss on every
startup on every platform unless gated, it throws away and rebuilds all GPU
resources (a visible hitch and wasted work), and it depends on MapLibre's
restore path staying correct. Ugly to justify in code that outlives the defect.

**5. `location.reload()` once at startup.** (size: 3 lines)

*Trade-off:* known to work. *Risk:* doubles startup, re-runs `subscribe` (benign
— the driver replays every topic on subscribe), and is the crudest possible
signal to future readers that we did not understand the problem. Not recommended.

## Recommendation

**Options 1 + 2: do nothing in the app, and move the spike emulator off API 34.**

The defect is in WebView 113's compositor, not in our code, and it does not exist
on WebView 124 or 134. Writing a permanent workaround into `Map.svelte` for a
2023 WebView build means carrying either a timing race (option 3) or a
deliberate GPU-context bounce (option 4) forever, in a file where the next reader
will have no idea why. Both are worse than the bug for real users, who will never
see it.

What I would do concretely:

1. Re-point the milestone's emulator at `spike-api35` (WebView 124) or
   `Medium_Phone_API_36.1` (WebView 134). Task 6 then gets a clean signal, and
   the milestone's first acceptance row passes.
2. Record in the Android platform doc that API 34's bundled WebView 113 shows a
   blank WebGL canvas on first load, that this is a WebView defect, and that the
   emulator matrix starts at API 35 because of it.
3. Leave a note to re-check on the first physical device we get hold of,
   especially any e-ink or kiosk hardware with a pinned WebView. If such a device
   turns out to ship an old WebView, revisit option 4 — it is the deterministic
   one, and by then we would have a real device to validate against.

Options 3-5 are not worth a task now.

## Open Questions

- Whether to carry a belt-and-braces fix in the app regardless. Option 4 is the
  better of the two candidates, because it is deterministic rather than a race.

## Instrumentation and cleanup

- **No production code was changed at any point.** Everything was done through
  the Chrome DevTools Protocol against the already-installed debug APK
  (`Page.addScriptToEvaluateOnNewDocument`, `Runtime.evaluate`, `Network.enable`,
  `Target.setAutoAttach`). All scripts live in the session scratchpad, outside
  the repo.
- `git status --porcelain` in
  `.claude/worktrees/app-rearchitect-kiss-b4b1c2` is **empty**; `HEAD` is still
  `5abaa08`. No commits, branches, checkouts or stashes.
- `cargo build -p updraft_tauri --bin updraft` was run once (desktop build
  check); it only writes to the gitignored `target/`.
- `/data/local/tmp/webview-command-line` was written on the emulator during the
  SwiftShader test and has been **removed** (`/data/local/tmp` now contains only
  `dalvik-cache`).
- `spike-api35` and `Medium_Phone_API_36.1` were booted for the version matrix
  and have been **shut down**. `spike-api34` is still running on
  `emulator-5554` with `aero.updraft.debug` installed, as required by tasks 6-8.
  Note the app is installed on API 35 and 36 as well, and their GNSS is left at a
  Zurich fix; harmless, but worth knowing if either AVD is reused.
- `spike-api34`'s emulated GNSS is left at the Zurich fix
  (`adb emu geo fix 8.5417 47.3769 500`) from the subscribe-channel test. Any
  later task that sets its own fix will override it.
