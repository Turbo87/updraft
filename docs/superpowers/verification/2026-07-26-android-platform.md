# Android platform verification

This record describes the Android platform checks from 2026-07-26. The tests
used the completed Android platform milestone and its debug APK.

The checks covered these functions:

- The foreground service and its partial wake lock.
- GNSS fixes from the Android location provider.
- Process survival after activity destruction.
- Webview reconstruction after an activity relaunch.
- Permission revocation and force-stop behavior.

The checks used Android API 34 and API 35 emulators. The API 34 emulator used
Android System WebView 113. The API 35 emulator used Android System WebView
124. The application used Tauri 2.11.5.

## Automated checks

The Rust tests, Clippy, frontend build, frontend checks, frontend tests,
frontend lint, and Playwright test passed before the emulator checks.

The release build did not produce an APK. Android lint failed while it analyzed
the Kotlin Gradle scripts. The failure occurred before packaging and did not
identify application code. The R8 minification task passed separately.

R8 kept the mobile plugin commands through Tauri's consumer rules. AAPT2 kept
`SessionService` through the service entry in the Android manifest. The project
did not need an additional keep rule for these types.

## Lifecycle results

The API 35 emulator completed all visual and lifecycle checks:

- A cold start showed the map and an ownship position.
- Five minutes on the home screen kept the service, wake lock, and GNSS fixes.
- Five minutes with the screen off kept the same functions.
- A swipe from Recents destroyed the activity but kept the process and service.
- A later launch rebuilt the webview and showed the live map.
- A force-stop ended the process. A later launch started a new session.

The API 34 emulator produced the same service, wake-lock, GNSS, and lifecycle
results. WebView 113 did not composite the WebGL canvas on the first page load
of a new process. This issue blocked the visual ownship check after a cold
start and after a force-stop. The rebuilt webview after a Recents swipe painted
the map correctly.

The separate [blank-map investigation](../investigations/2026-07-26-android-webview-blank-map.md)
contains the WebView analysis.

Measured activity relaunches rebuilt the webview on the first or second offer.
No measured relaunch needed a third offer. The implementation makes ten offers
at 200 ms intervals. This gives the event loop two seconds to accept an offer.

## GNSS timing

The emulator location provider supplied fixes at approximately 1 Hz. The
median interval was approximately 1.00 seconds. The maximum measured interval
was 2.013 seconds. No measured interval exceeded five seconds.

Background operation did not increase the skipped-fix rate. The screen-off
checks reached light idle. They did not reach deep idle.

These results do not prove behavior during real Doze. They also do not prove
behavior under manufacturer-specific background limits.

## Permission revocation

Android killed the process when the location permission was revoked. Android
then restarted `SessionService` with a null intent because the service used
`START_STICKY`. The service detected that no session was available and stopped
itself. It did not keep a notification or wake lock without a session.

A later launch requested location permission again. An approximate-location
answer did not start the session. The application wrote the failure to its log,
but the user interface did not explain why no session started.

## Unverified items

The emulator checks did not verify these items:

- Real Doze and manufacturer-specific background limits.
- Cold-start time on a physical device.
- The WebView 113 canvas issue on physical hardware with an old pinned WebView.
- A release APK installed and run after minification.
- A user-visible explanation for a denied or revoked location permission.
