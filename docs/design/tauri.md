# The Tauri Shell

The **Tauri** application is the primary shipping artifact for all five platforms. It embeds `CoreRuntime` in-process, hosts the frontend in the system webview, and bridges the two directly over Tauri IPC using the shared application protocol (see [core.md](core.md)). It resolves opaque bulk-resource IDs through a custom URI scheme. It is also where platform-specific concerns live: permissions, background execution, screen keep-awake, and access to native device APIs.

The shell is kept deliberately thin. Everything above it (core, protocol, frontend) is host-agnostic and covered by the main test layers, so the shell itself only needs a small smoke-test checklist (see [testing.md](testing.md)).

## Safety Constraints

A soaring computer gives warnings the pilot relies on, so safety must be independent of the most fragile layer, the webview.

- **Safety-critical logic runs natively.** Airspace and traffic warning generation runs in `App`. Warning audio and IGC writes are native effects executed by `CoreRuntime`, never JavaScript. Android suspends webview JavaScript when the screen is off or the app is backgrounded. The foreground service keeps the runtime alive, not the webview. If the frontend is suspended, warnings still sound and the flight is still logged.
- **Audio is native.** Warning effects drive native playback directly, so warning sounds survive webview suspension and respect platform audio focus and ducking rules. The frontend may trigger non-critical UI sounds, but nothing safety-relevant depends on it being alive.
- **Not a certified navigation source.** A first-run disclaimer and about-screen text state this explicitly, both an honest safety message and a smoother path through app-store review (UI side in [frontend.md](frontend.md)).
- **Power assumption.** Long soaring flights are assumed to run on external power (powerbank or ship power). Battery drain is not a primary design constraint, but rendering performance on low-end devices still is.

## Mobile Plugins (Kotlin first, Swift later)

All platform-specific native functionality lives in Tauri mobile plugins, but feeds data into the devices module through the same adapters (see [devices.md](devices.md)). `App` receives normalized observations and never knows whether they originated from SPP, BLE, TCP, serial, internal sensors, or replay.

1. **Bluetooth plugin:** SPP via `BluetoothSocket` and BLE via GATT, modeled on the SimpleBluetoothTerminal / SimpleBluetoothLeTerminal projects. Scan/connect/disconnect APIs, byte streams bridged into Rust transports. Capacitor community BT plugins serve as additional reference for permission edge cases.
2. **Foreground service plugin:** an Android foreground service of type `location|connectedDevice`, started **only** when GPS tracking or a BT connection is active and stopped when neither is. Persistent notification shows connection status (later: logging state). Handles Doze, battery-optimization exemption prompts, and connection supervision (auto-reconnect with backoff). **Play Store policy planning:** Android 14+ requires declared use cases per foreground-service type, and the Play Console requires written justification plus a demo video for `location`/`connectedDevice` services. Prepare this material as part of the first Android release, do not discover it at submission.
3. **Audio plugin:** native playback of **warning sounds** from `PlayWarning` effects, so audio survives webview suspension. Ships with the first release so airspace warnings are audible from day one. **Continuous audio vario is explicitly deferred.** It needs a different mode: parameter-driven tone synthesis on the native audio thread rather than event-triggered playback.

## iOS Scope & Risks

iOS ships with reduced capability: internal GPS (with background-location mode) plus BLE only. SPP is impossible on iOS, recorded as a **permanent platform limitation**, mitigated by BLE-capable hardware (modern FLARM and LXNav BLE bridges). Android is prioritized over iOS purely because the primary developer uses Android and has no iOS device to test with.

Known risks to plan around:

- **WKWebView background throttling** is stricter than Android's webview suspension. The safety constraints above (safety-critical logic native) are even more essential here. Background execution relies on the `location` and `bluetooth-central` background modes for the native side.
- **Tauri v2 iOS support** is the least mature part of the chosen stack. Treat the iOS port as carrying real platform-integration risk and validate early with a spike rather than assuming parity with Android.
- **App Store review:** navigation-adjacent apps attract scrutiny. The liability disclaimer, background-mode justifications, and BLE usage descriptions must be prepared for review.

## Open Questions

- **Native audio stack:** Rust vs Kotlin for the audio implementation, and the synthesis architecture for the (deferred) audio vario.
