# Android Platform Verification

Status: Historical verification record

This record describes Android platform checks from 2026-07-26. The tests used
API 34 and API 35 emulators and the completed debug APK for that milestone.

## Verified behavior

The checks covered:

- the foreground service and partial wake lock
- GNSS fixes from the Android location provider
- process survival after activity destruction
- webview reconstruction after activity relaunch
- permission revocation and force-stop behavior

The API 35 emulator completed all visual and lifecycle checks:

- A cold start showed the map and ownship position.
- Five minutes in the background kept the service, wake lock, and GNSS fixes.
- Five minutes with the screen off kept the same functions.
- Removing the activity from Recents kept the process and service.
- A later launch rebuilt the webview and showed the live map.
- A force-stop ended the process. A later launch started a new session.

The API 34 emulator produced the same service, wake-lock, GNSS, and lifecycle
results. Its WebView 113 did not composite the WebGL canvas on the first page
load of a new process. The rebuilt webview after an activity relaunch painted
the map correctly. The separate
[blank-map investigation](../research/investigations/2026-07-26-android-webview-blank-map.md)
contains that analysis.

Measured activity relaunches rebuilt the webview on the first or second offer.
No measured relaunch needed a third offer. The implementation made ten offers
at 200-millisecond intervals.

## GNSS timing

The emulator supplied fixes at approximately 1 Hz. The median interval was
approximately 1.00 seconds. The maximum measured interval was 2.013 seconds.
No measured interval exceeded five seconds.

Background operation did not increase the skipped-fix rate. The screen-off
checks reached light idle. They did not reach deep idle.

## Permission revocation

Android killed the process when location permission was revoked. It then
restarted the sticky foreground service with a null intent. The service found
no active session and stopped itself. It did not keep a notification or wake
lock.

A later launch requested location permission again. An approximate-location
answer did not start the session. The application logged the failure, but the
user interface did not explain the missing session.

## Automated checks

Rust formatting, tests, Clippy, frontend build, frontend checks, frontend
tests, frontend lint, and Playwright tests passed before the emulator checks.

The release build did not produce an APK because Android lint failed while it
analyzed Kotlin Gradle scripts. The failure occurred before packaging and did
not identify application code. R8 minification passed separately. R8 and AAPT2
kept the Tauri plugin commands and foreground service without additional keep
rules.

## Limits

The checks did not verify:

- deep Doze or manufacturer-specific background limits
- cold-start time on physical hardware
- the WebView 113 canvas issue on physical hardware
- an installed release APK after minification
- a user-visible explanation for denied or revoked location permission
