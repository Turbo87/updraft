# Android Platform

Status: Current behavior

Android is the primary mobile target. The Tauri application uses one Kotlin
plugin for foreground execution, internal GNSS, Bluetooth SPP, permissions, and
activity lifecycle events.

## Build boundary

The plugin has a minimum SDK of 24 and compiles against SDK 37. The generated
application and plugin use Java 8 bytecode. CI builds one debug APK for the
`aarch64` target and runs the plugin unit tests.

The Tauri and Android dependency versions are pinned. An upgrade must repeat
the lifecycle and generated-project checks that depend on those versions.

## Foreground session

Updraft starts `SessionService` while an activity is visible. The service can
use the `location` and `connectedDevice` foreground-service types. It displays
one ongoing low-priority notification.

The service holds one non-reference-counted partial wake lock for the active
session. The lock keeps the CPU scheduled while the screen is off. The service
releases it when the service stops.

A system restart with a null service intent does not contain the previous
session state. The service stops itself instead of keeping a notification and
wake lock for a session that it cannot resume.

Foreground-service startup reports success only after the service enters the
foreground and starts the requested source. A security or lifecycle failure is
returned to the Rust caller.

## Quit

A quit stops `SessionService`, removes the application task from Recents, and
ends the process. Android owns the exit because Rust cannot wake its event loop
after Android removes the window.

## Permissions

Internal GNSS requires fine and coarse location permission. Android 12 and
later Bluetooth connections require Nearby Devices permission through
`BLUETOOTH_CONNECT`. The manifest also declares the foreground-service, wake
lock, and notification permissions needed by the session.

Location and Bluetooth permissions are independent. A denied Bluetooth
permission does not disable internal GNSS. A denied location permission does
not invalidate a saved Bluetooth configuration.

The Settings interface reports whether bonded Bluetooth devices are
unsupported, unavailable because permission is denied, unavailable because
Bluetooth is disabled, or available.

## Internal GNSS

The plugin sends typed fixes through a Tauri channel. A fix contains latitude,
longitude, UTC time, and optional ellipsoid altitude, ground track, and ground
speed.

The Rust boundary rejects unknown fields. The core converts ellipsoid altitude
to mean-sea-level altitude. The plugin does not apply flight-domain source
selection or altitude policy.

## Bluetooth SPP

Each maintained SPP connection has an independent connection ID, request,
socket, reader thread, and event channel. Connections can run in parallel.

An attempt verifies that Bluetooth is available and enabled, permission is
granted, and the target is bonded. It then creates a secure RFCOMM socket for
the configured service UUID.

The plugin reports connected, byte, and terminal disconnected events. Byte
payloads use base64 across the Kotlin-to-Rust channel. Rust owns retry timing
and starts a new attempt after a terminal event when the device remains
enabled.

Cancellation targets one connection ID. It does not stop other attempts or the
shared foreground session. Service destruction drains pending and active
attempts and emits one terminal outcome for each attempt.

## Activity and screen lifecycle

`MainActivity` sets `FLAG_KEEP_SCREEN_ON`. This keeps the display awake while
the activity is visible. It does not replace the partial wake lock that keeps
the background session scheduled.

The foreground service can survive activity destruction. A later activity
reports its lifecycle through the plugin. The Rust shell rebuilds the configured
webview window after the new activity reaches the started state.

The rebuild uses bounded repeated offers because the Android event loop can
lose the first wake during an activity transition. It stops when a window
exists, the activity disappears, a build fails, or all offers are used.

## Validation limits

Automated tests cover typed plugin messages, registry behavior, service-state
transitions, activity rebuild policy, and the generated MainActivity flag.
Physical verification is still required for real background limits, Bluetooth
hardware, manufacturer behavior, and release builds.

The records under [verification](../verification/) are historical
physical evidence. They do not extend this platform contract.
