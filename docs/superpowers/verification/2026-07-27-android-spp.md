# Android SPP physical verification

This record covers the Samsung S23 acceptance run and its evidence-retention
fix round on 2026-07-27. Measured coverage is complete except for the simulator
power-cycle case and the debug-level `Connecting` timing described below.

Raw device captures were not committed because they may contain device data.
This document is the standalone public record of the environment, commands,
measurements, limitations, and verification results.

## Test environment

- Phone: Samsung Galaxy S23, model `SM-S911B`, device `dm1q`.
- Android: version 16, API 36, build `BP4A.251205.006`.
- Build fingerprint:
  `samsung/dm1qxeea/dm1q:16/BP4A.251205.006/S911BXXSAFZF5:user/release-keys`.
- Android System WebView: `com.google.android.webview` version
  `150.0.7871.124`.
- Tested application configuration: connection 2 used Android SPP with the
  paired simulator's device address and the standard SPP UUID.
- Installed APK SHA-256:
  `b0293a3ab5299788b44b489dfa062b603caa563ffda30f58663234f5df1b9d76`.
- Installed APK size: 314,977,701 bytes.
- Later Step 7 APK SHA-256:
  `bbb702d1bb448ac27830abfd73d6ba1bbe5676c3162ab565eb94b2d1f006ba12`.
- Later Step 7 APK size: 314,977,765 bytes. This build was not substituted for
  the APK installed during the physical fix round.
- Package: `aero.updraft.debug`, version name 1.0, version code 1.
- Simulator `status` reported `scenario=builtin`, `batches=301`, and no
  uploaded scenario.
- The simulator firmware configuration enables Bluedroid, Classic
  Bluetooth, BLE, SPP, GATT server support, and dual-mode controller operation.
  Its default and maximum log levels are `WARN`.
- Simulator serial connection: USB serial at 115200 baud, 8N1.
- Paired device: `NMEA-SIM`, device address redacted, standard SPP UUID.

Android reported `NMEA-SIM` as bonded with a persistent BR/EDR link key and
SPP service discovery before the clean run.

## Clean lifecycle evidence

The original clean run used connection ID 2 and the configured MAC in every
recorded SPP lifecycle event.

- Android started the SPP foreground-service attempt at 14:23:51.415 CEST.
- `Connected` was recorded at 14:23:52.530, 1.115 seconds after that Android
  service attempt.
- `First bytes` was recorded at 14:24:16.060, 24.645 seconds after the service
  attempt and 23.530 seconds after `Connected`. Simulator playback had been
  restarted during that interval.
- A deliberate disconnect at 14:24:22.111 recorded 1,421 delivered bytes.
- Automatic reconnect reached `Connected` in 462 ms and `First bytes` 35 ms
  later.

`Connecting` is emitted at debug level while the production subscriber filters
at info level. It did not appear in the clean or supplemental captures, so a
`Connecting`-to-`Connected` or `Connecting`-to-`First bytes` duration was not
measured. The Android foreground-service attempt above is retained as the
nearest measured start boundary. No generated Android file or production
logging level was changed to manufacture that event.

## RFCOMM chunk boundaries

The commands were issued in this state-changing order:

```text
chunk spp 1
restart spp
chunk spp 512
restart spp
```

Status checks and deliberate disconnects were recorded between those commands
to associate each size with its effective simulator state and terminal Updraft
byte total.

- `spp.chunk=1` and `spp.effective=1` were retained at 15:36:59.326. Playback
  restarted at 15:37:01.295. The attempt ended at 15:37:30.073 with 2,950
  delivered bytes. Automatic reconnect reached `Connected` in 422 ms and
  `First bytes` 896 ms later.
- `spp.chunk=512` and `spp.effective=512` were retained at 15:37:57.736.
  Playback restarted at 15:37:59.710. The attempt ended at 15:38:35.593 with
  10,537 delivered bytes. Automatic reconnect reached `Connected` in 452 ms
  and `First bytes` 760 ms later.

The 1-byte run produced simulator `event overrun spp` diagnostics while still
delivering a positive total. No Updraft crash, overlapping SPP attempt, or
missing terminal lifecycle event accompanied those diagnostics.

## Background, screen-off, and activity lifecycle

The authoritative supplemental background and screen-off run used PID 31189,
connection ID 2, an effective 512-byte simulator chunk, and foreground-service
type mask `0x18`.

- Simulator playback restarted at 16:01:42.913 and status showed position 1 at
  16:01:43.747.
- Android hid the Updraft activity window at 16:01:49.303. This is the
  background start boundary.
- The final background poll was captured on the device at 16:07:30.194,
  340.891 seconds after that boundary.
- Intermediate polls retained PID 31189, foreground-service type `0x18`,
  `mWakefulness=Awake`, the launcher in front, and simulator positions 78, 145,
  217, and 278.
- Playback restarted before EOF at 16:06:41.903. Final status showed position
  50 after the restart.
- Playback restarted for the screen-off run at 16:08:00.774 and status showed
  position 1 at 16:08:01.611.
- Android recorded power-button sleep at 16:08:06.058 and `Dozing` at
  16:08:06.684.
- The final screen-off poll was captured on the device at 16:13:44.909,
  338.851 seconds after the sleep boundary.
- Intermediate polls retained PID 31189, foreground-service type `0x18`,
  `mWakefulness=Dozing`, and simulator positions 74, 141, 206, and 268.
- Playback restarted before EOF at 16:12:52.230. Final status showed position
  54 after the restart.
- The connected attempt spanning both intervals ended at 16:14:03.515 with
  228,341 delivered bytes.
- Automatic reconnect reached `Connected` in 548 ms and `First bytes` 226 ms
  later.

An earlier background setup began while the phone was locked. The HOME action
woke the phone rather than establishing the intended unlocked-app background
transition, so that attempt was excluded and repeated.

The original activity-destruction case remains independently supported:

- The inspected `Updraft` card was swiped from Recents without force stopping
  the package.
- Android recorded the activity as destroyed at 14:48:19.419.
- PID 19427 and foreground-service type `0x18` remained unchanged after the
  card disappeared.
- Relaunch created the new activity at 14:48:54.272, 34.853 seconds after
  destruction. Android classified it as a warm launch and displayed it in
  88 ms.
- The activity watcher rebuilt the webview window after waiting 7 ms.
- The subsequent connected attempt ended at 14:49:13.249 with 129,207
  delivered bytes.
- Automatic reconnect reached `Connected` in 507 ms and `First bytes` 183 ms
  later.

The original operator checkpoints of 14:25:43 for background entry and
14:31:07 for screen-off were not Android lifecycle boundaries. Android
recorded the corresponding events at 14:25:50.407 and 14:31:14.186. The
supplemental run above replaces those unsupported checkpoint times and retains
the serial commands that were missing from the original transcript.

## Independent permissions

Permission changes were made through Android Settings after inspecting and
retaining the visible labels and radio-button state. Direct permission grant
and revoke commands were not used.

The initial state at 16:24:20 used PID 31189 with Notifications, precise
Location, and Nearby Devices granted, foreground-service type `0x18`, and an
active one-second, high-accuracy GPS request.

For Location granted and Nearby Devices denied:

- Android Settings showed Location allowed and Nearby Devices denied.
- Revoking Nearby Devices replaced PID 31189 with PID 14106.
- After relaunch and denial of the inspected runtime prompt, Android still
  reported Location granted, Nearby Devices denied, and PID 14106.
- `SessionService` used location-only type mask `0x08`.
- Android retained the active one-second, high-accuracy GPS request for
  `aero.updraft.debug`.
- The permission capture recorded 25 bounded `[permissionDenied]` SPP retry
  failures. No SPP streaming claim is made for this state.

For Nearby Devices granted and Location denied:

- Android Settings showed Nearby Devices allowed and Location denied. The
  intermediate session with both permissions used type mask `0x18`.
- Revoking Location replaced PID 14106 with PID 16044.
- After relaunch and denial of the inspected runtime prompt, Android reported
  Nearby Devices granted, Location denied, PID 16044, and
  connected-device-only type mask `0x10`.
- Playback restarted at 16:34:22.090 and Updraft recorded `First bytes` at
  16:34:22.205.
- The attempt ended at 16:34:35.053 with 2,639 delivered bytes. Automatic
  reconnect reached `Connected` in 444 ms and `First bytes` 702 ms later.

For the final restored state:

- Android Settings showed Location allowed with precise location enabled.
- Granting Location restored all runtime grants but the existing PID 16044
  initially retained its previous type mask `0x10`, including after bringing
  the existing activity to the foreground.
- `adb shell am force-stop aero.updraft.debug` followed by relaunch produced
  PID 17704 to establish the final restored permission state. It is not
  evidence of ordinary lifecycle survival.
- Android then reported Notifications, precise Location, and Nearby Devices
  granted, foreground-service type `0x18`, and the active one-second,
  high-accuracy GPS request.
- Playback restarted at 16:40:55.169 and Updraft recorded `First bytes` at
  16:40:55.335.
- The attempt ended at 16:41:15.283 with 4,255 delivered bytes. Automatic
  reconnect reached `Connected` in 565 ms and `First bytes` 427 ms later.
- The temporary stay-awake setting was restored to its original value of zero.
  The subsequent settings readback returned zero.

## Automated verification

Every required Task 6 Step 7 command returned exit status zero:

```text
# From the repository root:
cargo fmt --all --check
cargo test --workspace --exclude tauri-plugin-updraft --all-features
cargo test -p tauri-plugin-updraft --all-features
cargo clippy --workspace --exclude tauri-plugin-updraft --all-targets --all-features -- -D warnings
cargo clippy -p tauri-plugin-updraft --all-targets --all-features -- -D warnings
cargo doc --workspace --exclude tauri-plugin-updraft --no-deps --all-features
cargo doc -p tauri-plugin-updraft --no-deps --all-features
pnpm lint
pnpm build
pnpm build:storybook
pnpm check
pnpm test
pnpm test:e2e
NDK_HOME="$ANDROID_HOME/ndk/28.1.13356709" pnpm tauri android build --target aarch64 --apk --debug

# From tauri/gen/android:
./gradlew :tauri-plugin-updraft:testDebugUnitTest
```

Results include 2 plugin Rust tests, 17 frontend unit tests, 1 Playwright test,
0 Svelte errors, 0 Svelte warnings, one debug APK, and a passing plugin JVM
unit-test task. Both Clippy commands passed with `-D warnings`.

The first fresh `pnpm lint` attempt exited 1 because Prettier traversed a
temporary package file inside the ignored local `.pnpm-store`. Moving the
ignored cache aside made the exact command pass, after which the cache was
restored. This was environmental and not branch-caused. An earlier sandboxed
workspace-test attempt also failed three TCP tests because local binds returned
`Operation not permitted`. The exact command passed outside that sandbox.

The frontend, Storybook, end-to-end, and Android builds retain the existing
large-chunk advisory. Storybook retains its existing default-Svelte-config
notice. Playwright retains the existing `NO_COLOR` and `FORCE_COLOR` warning.
The Android build and JVM task retain generated Android Gradle Plugin
deprecated-option and variant-API warnings, pinned Tauri Kotlin deprecation
warnings, the Java 8 source and target warning, and the Gradle 10 compatibility
notice. None originates in the documentation-only evidence fix.

## Failures, deviations, and unrun work

- The phone initially held a stale Classic Bluetooth link key for `NMEA-SIM`.
  The first setup attempt triggered a Just Works pairing prompt, timed out, and
  removed the stale bond. The simulator reported SPP authentication status 9.
  Pairing was repaired through Android Settings at 14:21:12 without a PIN or
  passkey before the clean acceptance run began.
- The clean and supplemental runs did not expose the debug-level `Connecting`
  event. Its requested timing remains unmeasured.
- The 1-byte simulator run emitted overrun diagnostics. Updraft still recorded
  first bytes and a positive terminal byte total.
- The original continuous host-side `adb logcat` capture stopped at 14:29:30.
  A later `logcat -d` dump recovered the lifecycle tail from Android's ring
  buffer. Independent phone, process, service, and simulator polling covered
  the original timed intervals, and the supplemental intervals used fresh
  continuous captures.
- A simulator power cycle was not run. The verified UART `help` interface has
  no power command, the WCH USB bridge was not on a verified controllable hub
  path, and `uhubctl` was unavailable. A disconnect or reset was not
  substituted for loss and restoration of power.
- Android replaced the application process when Nearby Devices and Location
  were revoked. Restoring Location did not upgrade the existing foreground
  service mask until the explicit force-stop and relaunch. The post-relaunch
  states above include both the PID and service mask. Immediately after each
  revocation, Android exposed the replacement PID before the service restarted.
- The first supplemental timed-background setup was invalid because the phone
  was locked. It was excluded and repeated after retaining unlocked readiness.
- Permission-matrix readiness initially found the launcher rather than
  Updraft. A later ADB reconnect briefly made three readiness captures fail.
  The device reconnected and the unlocked Updraft precondition was checked
  again before testing continued.
- Simulator configuration was verified from its tracked `sdkconfig.defaults`
  rather than a generated `sdkconfig` file.
- The first supplemental launch used
  `aero.updraft.debug/.MainActivity`, which did not exist. Resolving the
  launcher produced `aero.updraft.debug/aero.updraft.MainActivity`, and the
  retry passed.
- The original acceptance tail contains 83 expected connection-1 failures for
  the unused local TCP endpoint `127.0.0.1:4353`. The fresh physical,
  lifecycle, and permission captures contain 105, 84, and 109 respectively.
  These connection-refused warnings continued independently while
  connection 2 delivered SPP bytes and did not affect the SPP measurements.

The map's displayed ownship position was not used as evidence. Raw acceptance
logs were not committed. A privacy scan found zero recognized NMEA sentence
prefixes and zero Updraft encoded-payload lifecycle markers across the original
and supplemental lifecycle captures. The diagnostics contained lifecycle
metadata and delivered-byte totals, not byte chunks or encoded payload.
