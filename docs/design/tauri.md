# The Tauri Shell

The **Tauri** application is the primary shipping artifact for all five platforms. It embeds the Rust core in-process, hosts the frontend in the system webview, and connects the two through the embedded axum server bound to loopback (see [server.md](server.md) and _The Embedded Server_ below) — the same transport, routes, and auth as everywhere else. There is no separate IPC bridge and no custom URI scheme. The shell is where platform-specific concerns live: permissions (location, Bluetooth), background execution, screen-keep-awake, and access to native device APIs.

The shell is kept deliberately thin. Everything above it (core, protocol, frontend) is host-agnostic and covered by the main test layers, so the shell itself only needs a small smoke-test checklist (see [testing.md](testing.md)).

## The Embedded Server

The shell starts the axum server on a loopback port and points the webview at `http://localhost:<port>`, with the session token injected at page load. One transport instead of two is a deliberate trade:

- The alternative — Tauri's custom URI scheme — cannot stream response bodies (wry #1404), and Android's webview fails byte-range requests against custom protocols (a Chromium webview bug), which breaks PMTiles range reads exactly on the primary platform. The established community workaround for offline maps in Tauri on Android is an embedded loopback HTTP server, so the "fallback" _is_ this design.
- With the page itself served from a localhost origin, SSE works plainly and none of the secure-context restrictions that webviews apply to custom-scheme origins get in the way.
- The protocol, its contract tests, and the Playwright suite cover the shipping transport exactly, on all five platforms.

Platform footwork this requires:

- **Android:** release builds block cleartext HTTP since Android 9. Loopback needs `usesCleartextTraffic` scoped to `127.0.0.1` via a network security config — debug builds set it implicitly, so this belongs on the release checklist rather than being discovered in the first release build.
- **iOS:** an ATS loopback exemption, if the default localhost carve-out proves insufficient.
- **Security:** a loopback listener is reachable by every local process, so all routes require the session token (see [server.md](server.md)).

**Validation spike (early):** the one genuinely unproven part is lifecycle — the server and its port across Android suspend/resume alongside the foreground service, and iOS backgrounding. This is validated before the transport decision becomes load-bearing; the fallback if it fails is the custom-scheme protocol with its known Android limitations.

## Safety Constraints

A soaring computer gives warnings the pilot relies on, so safety must be independent of the most fragile layer, the webview.

- **Safety-critical logic runs natively.** Airspace and traffic warning generation runs in the Rust core, and warning audio and IGC logging are performed by native adapters driven directly by the core, never in JavaScript. Android suspends webview JavaScript when the screen is off or the app is backgrounded. The foreground service keeps the _core_ alive, not the webview. The frontend is pure presentation. If it is suspended, warnings still sound and the flight is still logged.
- **Audio is native.** Warning sounds are played on the native side and driven directly from the core, so they survive webview suspension and respect platform audio focus/ducking rules (warnings must be heard over music or other apps). The frontend may trigger non-critical UI sounds, but nothing safety-relevant depends on it being alive.
- **Not a certified navigation source.** A first-run disclaimer and about-screen text state this explicitly, both an honest safety message and a smoother path through app-store review (UI side in [frontend.md](frontend.md)).
- **Power assumption.** Long soaring flights are assumed to run on external power (powerbank or ship power). Battery drain is not a primary design constraint, but rendering performance on low-end devices still is.

## Mobile Plugins (Kotlin first, Swift later)

All platform-specific native functionality lives in Tauri mobile plugins, but feeds data into the core through the same device abstractions (see [devices.md](devices.md)). The core never knows whether an NMEA stream came from SPP, BLE, TCP, a serial port, or a replay file.

1. **Bluetooth plugin:** SPP via `BluetoothSocket` and BLE via GATT, modeled on the SimpleBluetoothTerminal / SimpleBluetoothLeTerminal projects. Scan/connect/disconnect APIs, byte streams bridged into Rust transports. Capacitor community BT plugins serve as additional reference for permission edge cases.
2. **Foreground service plugin:** an Android foreground service of type `location|connectedDevice`, started **only** when GPS tracking or a BT connection is active and stopped when neither is. Persistent notification shows connection status (later: logging state). Handles Doze, battery-optimization exemption prompts, and connection supervision (auto-reconnect with backoff). **Play Store policy planning:** Android 14+ requires declared use cases per foreground-service type, and the Play Console requires written justification plus a demo video for `location`/`connectedDevice` services. Prepare this material as part of the first Android release, do not discover it at submission.
3. **Audio plugin:** native playback of **warning sounds** (airspace, traffic), driven directly from the Rust core so audio survives webview suspension. Ships with the first release so airspace warnings are audible from day one. **Continuous audio vario is explicitly deferred.** It is a nice-to-have at the very end of the roadmap, and when it comes it needs a different mode: parameter-driven tone synthesis on the native audio thread (core streams climb rate, audio thread modulates frequency and beep rate) rather than event-triggered playback.

## iOS Scope & Risks

iOS ships with reduced capability: internal GPS (with background-location mode) plus BLE only. SPP is impossible on iOS, recorded as a **permanent platform limitation**, mitigated by BLE-capable hardware (modern FLARM and LXNav BLE bridges). Android is prioritized over iOS purely because the primary developer uses Android and has no iOS device to test with.

Known risks to plan around:

- **WKWebView background throttling** is stricter than Android's webview suspension. The safety constraints above (safety-critical logic native) are even more essential here. Background execution relies on the `location` and `bluetooth-central` background modes for the native side.
- **Tauri v2 iOS support** is the least mature part of the chosen stack. Treat the iOS port as carrying real platform-integration risk and validate early with a spike rather than assuming parity with Android.
- **App Store review:** navigation-adjacent apps attract scrutiny. The liability disclaimer, background-mode justifications, and BLE usage descriptions must be prepared for review.

## Open Questions

- **Native audio stack:** Rust vs Kotlin for the audio implementation, and the synthesis architecture for the (deferred) audio vario.
