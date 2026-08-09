# Android WebView 113 Blank Map

Status: Historical investigation

This investigation examined a blank MapLibre canvas during the first page load
of a fresh Android application process. It used the same debug APK on Android
API 34, 35, and 36 emulators at commit `5abaa08`.

## Conclusion

Android System WebView 113 created and rendered the WebGL canvas, but did not
composite it. The problem reproduced on every tested API 34 cold start with
WebView 113. It did not reproduce on API 35 with WebView 124 or API 36 with
WebView 134.

The defect affected a WebGL context that was created before the WebView process
had produced its first composited frame. A later WebGL canvas worked in the
same page. Reloading the page also worked because the process had already
produced a frame.

This evidence did not identify an Updraft, MapLibre, network, or Tauri data-flow
defect.

## Reproduction matrix

- API 34 with WebView 113: 13 of 13 cold starts showed a blank canvas.
- API 35 with WebView 124: 3 of 3 cold starts painted the map.
- API 36 with WebView 134: 3 of 3 cold starts painted the map.

A cold start force-stopped the package and started its main activity. A reload
made the map visible for the rest of that process.

## Evidence

The investigation established these facts:

1. MapLibre tile requests completed with HTTP 200 in its web worker.
2. The WebGL context reported successful shader and framebuffer operations.
3. MapLibre issued thousands of draw calls, including more calls after a map
   gesture.
4. Repeated opaque clears on MapLibre's context did not change the screen.
5. The canvas and its ancestors had visible layout and stacking state.
6. New 2-D and WebGL canvases created later in the same page displayed
   correctly.
7. Timed WebGL canvases showed a boundary near the process's first composited
   frame. Canvases created before that boundary never became visible.
8. A forced `WEBGL_lose_context` loss and restore rebuilt the drawing buffer and
   made the MapLibre canvas visible.
9. The same APK worked with newer WebView versions.

These results locate the failure between the early WebGL drawing buffer and the
WebView compositor.

## Eliminated causes

The evidence ruled out:

- failed tile requests
- failed topic subscription or ownship updates
- a zero-sized or covered canvas
- an ordinary WebGL context loss
- an idle stale surface
- a MapLibre-only drawing failure
- a general WebGL failure in the process
- the Tauri activity-resume defect that creates no webview

The exact Chromium-internal cause was not identified. That would require a
debug build of the affected WebView version.

## Tauri activity distinction

The Tauri activity-resume defect and this WebView defect can both show a blank
screen, but they have different signatures.

For the Tauri defect, no webview or page exists after activity relaunch. For the
WebView 113 defect, the page and DevTools target exist. DOM controls and map
attribution render, topic updates arrive, and only the WebGL canvas stays blank.

## Mitigation assessment

The investigation considered four mitigations:

- Require a newer Android System WebView.
- Use a newer emulator image for visual and lifecycle checks.
- Delay MapLibre construction until after the first composited frame.
- Force one WebGL context loss and restore after MapLibre loads.

Reloading the page also worked, but it repeated application startup and was not
a suitable product fix.

No timing signal reliably exposed the process compositor boundary. A delayed
construction workaround would therefore depend on an empirical race. A forced
context restore was deterministic but would discard and rebuild GPU resources.

No workaround was implemented during this investigation. The practical test
environment moved to a newer WebView. Product support for an old or pinned
WebView requires an explicit warning or a verified recovery strategy.

## Limits

The checks used emulators. They did not test physical hardware with an old
pinned WebView. They also did not build Chromium or identify the internal layer
ownership defect.

The investigation used the dependency versions at commit `5abaa08`. Recheck
the behavior before applying these findings to a different WebView or MapLibre
version.
