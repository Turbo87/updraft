# Android SPP Physical Verification

Status: Historical verification record

This record summarizes a physical Android SPP acceptance run from 2026-07-27.
Raw captures were not committed because they can contain device data. Personal
device identifiers, Bluetooth addresses, build fingerprints, and process IDs
are not part of this public record.

## Environment

- Physical Android phone with Android 16 and API 36
- Android System WebView 150
- Debug APK for the completed SPP milestone
- Paired NMEA simulator over Bluetooth Classic SPP
- Standard SPP service UUID
- Simulator USB serial control at 115200 baud, 8N1

The simulator used its built-in scenario. The checks used one enabled Android
SPP connection.

## Connection lifecycle

The clean connection reached `Connected` 1.115 seconds after the Android
foreground-service attempt. The first bytes arrived after simulator playback
started. A deliberate disconnect recorded a positive delivered-byte total.

Automatic reconnects reached `Connected` in 422 to 565 milliseconds. First
bytes arrived 35 to 896 milliseconds later.

The production subscriber did not capture the debug-level `Connecting` event.
The measurements therefore use the Android service attempt as the nearest start
boundary.

## RFCOMM chunk boundaries

The simulator ran once with one-byte chunks and once with 512-byte chunks. Both
runs delivered positive byte totals and completed the expected disconnect and
reconnect lifecycle.

The one-byte run produced simulator overrun diagnostics. Updraft did not crash,
start an overlapping attempt, or omit the terminal lifecycle event.

## Background and screen-off behavior

One connected attempt remained active through more than five minutes in the
background and more than five minutes with the screen off. The foreground
service and process stayed active. Simulator position advanced during both
intervals. The attempt later ended with a positive delivered-byte total and
reconnected.

Removing the activity from Recents kept the process and foreground service. A
later launch rebuilt the webview. The active SPP attempt continued across the
activity lifecycle.

## Independent permissions

The checks changed permissions through Android Settings.

With Location granted and Nearby Devices denied, the foreground service used
its location-only type. GNSS stayed active. SPP retries reported bounded
permission failures. No SPP streaming claim is made for this state.

With Nearby Devices granted and Location denied, the foreground service used
its connected-device-only type. SPP playback delivered bytes and reconnected
after a deliberate disconnect.

Restoring Location restored both runtime grants. The existing foreground
service kept its previous type until a force-stop and relaunch started a new
session with both service types.

## Automated checks

The acceptance run also completed the repository Rust, frontend, end-to-end,
Android build, and Kotlin plugin test commands that applied to the milestone.
The exact current validation commands are in the
[testing guide](../development/testing.md).

## Limits

- A simulator power cycle was not run. A reset or disconnect was not used as a
  substitute.
- The debug-level `Connecting` duration was not measured.
- The one-byte simulator run had overrun diagnostics.
- The checks did not test another phone manufacturer or Android version.
- Restoring Location did not update the running foreground-service type until a
  new application process started.
