# Android Platform Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Keep Updraft navigating on Android after the user leaves it — a foreground service that survives backgrounding, activity destruction and relaunch, feeding internal GPS fixes to the core as typed values.

**Architecture:** One Tauri v2 mobile plugin holds the Kotlin the app cannot get from Rust: a foreground service, a wake lock, and the location provider. Control flows through `run_mobile_plugin` as JSON; fixes flow through a `tauri::ipc::Channel` into a Rust closure that turns them into `Input::InternalGps`. The core gains one input variant and stays pure.

**Tech Stack:** Rust 2024, Tauri 2.11, Kotlin, Android SDK 34/35, NDK 28.1.13356709, JDK 21.

This plan delivers milestone 2 of the six in [the architecture spec](../specs/2026-07-25-app-architecture-design.md).

## What earlier spikes established

Two emulator spikes were run on 2026-07-07 against API 34 and 35 with a scratch Tauri app. Their findings are why this milestone is shaped the way it is.

**A foreground service is not optional.** Without `startForeground()`, the message stream from Kotlin froze 58 seconds after backgrounding, with the process reported `isFrozen=true` by the cached-apps freezer. The exact timing is emulator-indicative, the behaviour is not.

**Stock Tauri kills the process when the activity is destroyed.** tao's Android event loop ends in `std::process::exit`, so when the activity goes away — whether by recents swipe or by don't-keep-activities — the process self-exits about two seconds after `onTaskRemoved` and takes the service with it. Logcat shows `Process ... has died: prcp FGS`. `RunEvent::ExitRequested` + `api.prevent_exit()` fixes it completely: same pid, activity `DESTROYED`, stream uninterrupted across a 12-minute soak. Task 4.

**`START_STICKY` resurrects the service with a null intent and no channel.** A restarted service therefore has nothing to feed and must `stopSelf()` rather than linger as a zombie holding a notification.

**Relaunching after activity destruction shows a blank webview.** JS never executes, a second relaunch does not recover, only killing the process does — while the Rust core and the service keep running underneath. Root cause: tao maps one Android Activity per tao `Window` and `Window::new` claims the "next available activity"; a launcher-relaunched activity gets a fresh `hashCode()` id that nothing matches, so no webview is built. The missing signal is in `tauri-runtime-wry`, which dispatches mobile `Resumed` only per existing window and therefore drops it entirely when there are zero windows. Reported as [tauri#15671](https://github.com/tauri-apps/tauri/issues/15671) with a fix in [tauri#15678](https://github.com/tauri-apps/tauri/pull/15678); both open two weeks with no movement. Tasks 6 and 7.

**Foreground service type behaviour, all verified identically on API 34 and 35:**

- Repeated `startForeground()` with a changed type mask applies in place: new mask visible in `dumpsys`, a single `onCreate`, the emitter thread never interrupted.
- Using a type bit absent from the manifest attribute throws `IllegalArgumentException: foregroundServiceType 0x… is not a subset of foregroundServiceType attribute 0x… in service element of manifest file`. The service keeps running with its previous mask.
- Requesting the location bit without location permission throws `SecurityException: Starting FGS with type location … requires permissions: all of [FOREGROUND_SERVICE_LOCATION] any of [ACCESS_COARSE_LOCATION, ACCESS_FINE_LOCATION]`. Notably the failed call did **not** trigger the "did not then call startForeground()" ANR: the service stayed alive as a plain started service.
- Starting the service from the background throws `ForegroundServiceStartNotAllowedException: … not allowed due to mAllowStartForeground false`. The service must be started while the activity is visible.
- `Service.onTimeout()` never fired across 31 minutes of continuous foreground plus 13 minutes task-removed.
- A notification built with `setOngoing(true)` is not user-dismissible on the emulator; swipe bounces. Whether it should be dismissible on a device is a deliberate decision left for later.

**Force-stop** kills the process instantly with no service callbacks at all — `onDestroy` never runs — and a fresh relaunch recovers cleanly.

## Global Constraints

- The core stays pure: no clock, no I/O, no threads, no Tauri, no Android. `Input::InternalGps` is a plain data variant. Domain conversions such as the geoid correction belong in the core, not the shell.
- Fixes are **typed values, never synthesised NMEA**. The plugin hands over structured positions; only real devices speak NMEA.
- **No `ACCESS_BACKGROUND_LOCATION`.** The service is started from a visible activity under the while-in-use rule. Requesting background location would trigger a Play policy review the app does not need.
- Declare only the permissions and service types actually used. `connectedDevice` and the type-switching command arrive in milestone 3 along with Bluetooth; adding them now would be declaring capability we do not exercise. `ACCESS_COARSE_LOCATION` is the exception the platform forces: Android 12 ignores a request for `ACCESS_FINE_LOCATION` unless coarse is declared and requested with it, so the pair counts as one permission the app does use. An Approximate answer is still treated as a refusal.
- The plugin's permissions and its `SessionService` declaration live in the plugin's own `AndroidManifest.xml`, not the app's. Manifest merger unions them into the app, and `tauri/gen/android/` is generated output that loses hand edits.
- Rust edition 2024, toolchain 1.97.1. Workspace lints apply. Versions pinned with `=`.
- Use `claims` assertions where one fits: `assert_some!`, `assert_some_eq!`, `assert_none!`, `assert_ok!`. `assert_matches!` comes from `std` and needs `use std::assert_matches;`. Float comparisons use `approx::assert_abs_diff_eq!`.
- Test names carry no `a_` or `the_` prefix.
- Test and doc output must be pristine. Every commit leaves the build and tests green.
- No `#[allow(...)]` to silence a clippy lint.
- `lib.rs` keeps one alphabetical block of `mod` lines, a blank line, then one alphabetical block of `pub use` lines.
- Dependencies land in the task that first uses them. A commit that adds a dependency it does not yet call is noise.
- **The code in this plan was written, not compiled.** Milestone 1 hit ten defects in its own plan: a type that did not exist, a method named wrongly, a version never published, an attribute with an unwanted side effect. Treat every snippet, signature and version here as a claim to verify, and stop and ask when one does not hold rather than working around it silently. Catching one before writing code is the cheapest outcome available.
- Android build environment: `ANDROID_HOME=~/Library/Android/sdk`, `NDK_HOME=$ANDROID_HOME/ndk/28.1.13356709`, `JAVA_HOME="/Applications/Android Studio.app/Contents/jbr/Contents/Home"`. AVDs `spike-api34` and `spike-api35` already exist.

## The plugin

`libs/tauri_plugin_updraft`, Android package `aero.updraft.mobile`, Kotlin class `UpdraftMobilePlugin`, Tauri plugin identifier `updraft`.

It lives in `libs/` rather than a new `plugins/` directory because there is exactly one plugin and `libs/*` is already a workspace member glob. `libs/updraft_sprites` is likewise a build-time tool rather than a domain library, so this is not the first non-domain crate there.

It depends on `tauri`, which on Linux pulls in webkit2gtk. The workspace CI job installs no such system dependencies — that is why it already excludes `updraft_tauri` — so this crate needs the same exclusion, and its tests run in the `tauri` job instead.

---

## File Structure

**New** (`libs/tauri_plugin_updraft/`)

| File | Responsibility |
| --- | --- |
| `Cargo.toml`, `build.rs` | Plugin crate and its Android build hook |
| `src/lib.rs` | Plugin registration and the extension trait |
| `src/mobile.rs` | Android handle, `run_mobile_plugin` calls, the fix `Channel` |
| `src/desktop.rs` | A no-op implementation so the crate builds off Android |
| `src/models.rs`, `src/error.rs` | Command arguments and typed errors |
| `permissions/default.toml` | Plugin permission set |
| `android/` | Gradle module, manifest, Kotlin |
| `android/.../UpdraftMobilePlugin.kt` | The Tauri plugin entry point |
| `android/.../SessionService.kt` | The foreground service, wake lock, notification |
| `android/.../GpsSource.kt` | The location provider, emitting fixes to the channel |

**Modified**

| File | Change |
| --- | --- |
| `libs/updraft_core/src/fix.rs` | New: the `Fix` type |
| `libs/updraft_core/src/input.rs` | `Input::InternalGps(Fix)` |
| `libs/updraft_core/src/core.rs` | Apply a fix to instruments, with the geoid correction |
| `libs/updraft_core/Cargo.toml` | `updraft_egm96`, `updraft_geo`, `updraft_units` |
| `tauri/src/lib.rs` | `prevent_exit()`, webview re-creation, plugin registration, session start |
| `tauri/src/session.rs` | New: session lifecycle and the fix-to-input adapter |
| `libs/tauri_plugin_updraft/android/src/main/AndroidManifest.xml` | Permissions and the service declaration |
| `tauri/capabilities/default.json` | The plugin's permission |
| `.github/workflows/ci.yml` | Exclude the plugin from the workspace job, test it in the `tauri` job |

---

### Task 1: `Fix` and `Input::InternalGps` in the core

The core half, testable entirely on macOS. Doing it first means the Android work has a finished contract to target.

**Files:**

- Create: `libs/updraft_core/src/fix.rs`
- Modify: `libs/updraft_core/src/input.rs`, `src/core.rs`, `src/lib.rs`, `Cargo.toml`, `tests/scenario.rs`

**Interfaces:**

- Produces: `Fix { position: LatLon, altitude_ellipsoid_meters: Option<f64>, track_degrees: Option<f64>, ground_speed_meters_per_second: Option<f64> }` and `Input::InternalGps(Fix)`

The variant is named for its provenance, not its content. A position from the device's own GNSS receiver is a different thing from one decoded off a connected instrument, and source arbitration will need to tell them apart.

The altitude field is named for what Android actually gives: height above the WGS84 ellipsoid. Calling it MSL would be a lie worth roughly 47 m in central Europe. The core converts.

A `Fix` carries only what `Instruments` publishes today. Accuracy, GPS timestamp and satellite count are real properties of a fix and all will be needed for arbitration, but nothing consumes them yet.

- [x] **Step 1: Add the dependencies**

In `libs/updraft_core/Cargo.toml`, under `[dependencies]`:

```toml
updraft_egm96 = { path = "../updraft_egm96" }
updraft_geo = { path = "../updraft_geo" }
updraft_units = { path = "../updraft_units" }
```

- [x] **Step 2: Write the failing tests**

Create `libs/updraft_core/src/fix.rs`:

```rust
use crate::topic::LatLon;

/// A position report from the device's own GNSS receiver.
///
/// Distinct from a fix decoded out of NMEA: it arrives already structured,
/// from a source the operating system vouches for, so it never passes
/// through the framer.
///
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Fix {
    pub position: LatLon,
    pub altitude_ellipsoid_meters: Option<f64>,
    pub track_degrees: Option<f64>,
    pub ground_speed_meters_per_second: Option<f64>,
}
```

Add to the test module in `libs/updraft_core/src/core.rs`:

```rust
    fn fix(latitude_degrees: f64, longitude_degrees: f64) -> Fix {
        Fix {
            position: LatLon {
                latitude_degrees,
                longitude_degrees,
            },
            altitude_ellipsoid_meters: Some(247.0),
            track_degrees: Some(90.0),
            ground_speed_meters_per_second: Some(30.0),
        }
    }

    #[test]
    fn internal_gps_emits_instruments_immediately() {
        let mut core = Core::new(config());

        let effects = core.apply(Input::InternalGps(fix(50.823, 6.186)), at(100));

        assert_matches!(effects.as_slice(), [Effect::Emit(Topic::Instruments(_))]);
        let [Effect::Emit(Topic::Instruments(instruments))] = effects.as_slice() else {
            unreachable!()
        };
        let position = assert_some!(instruments.position);
        assert_abs_diff_eq!(position.latitude_degrees, 50.823, epsilon = 1e-9);
        assert_some_eq!(instruments.track_degrees, 90.0);
    }

    #[test]
    fn internal_gps_altitude_is_converted_to_msl() {
        let mut core = Core::new(config());

        core.apply(Input::InternalGps(fix(50.823, 6.186)), at(100));

        // A range, not an exact value: this asserts a correction happened,
        // and `updraft_egm96` owns testing its magnitude.
        let [Topic::Instruments(instruments)] = core.topics().as_slice() else {
            unreachable!()
        };
        let altitude = assert_some!(instruments.altitude_msl_meters);
        assert!(
            (150.0..=230.0).contains(&altitude),
            "expected a geoid-corrected altitude, got {altitude}"
        );
    }

    #[test]
    fn repeated_identical_fixes_emit_only_once() {
        let mut core = Core::new(config());
        let mut emissions = 0;

        for _ in 0..5 {
            emissions += core
                .apply(Input::InternalGps(fix(50.823, 6.186)), at(100))
                .len();
        }

        assert_eq!(emissions, 1, "only the first fix changed any value");
    }
```

- [x] **Step 3: Run the tests to verify they fail**

Run: `cargo test -p updraft_core`
Expected: FAIL, `no variant named 'InternalGps' found for enum 'Input'`.

- [x] **Step 4: Implement**

Add to `Input` in `libs/updraft_core/src/input.rs`, with `use crate::fix::Fix;` in that file's imports:

```rust
    /// A fix from the device's own GNSS receiver rather than a connected
    /// instrument. Which source a position came from is what later lets
    /// them be ranked against each other.
    InternalGps(Fix),
```

In `libs/updraft_core/src/core.rs`, add the arm to `apply`:

```rust
            Input::InternalGps(fix) => self.apply_fix(fix),
```

and the method beside `decode`:

```rust
    fn apply_fix(&mut self, fix: Fix) -> Vec<Effect> {
        let before = self.instruments;

        self.instruments.position = Some(fix.position);
        if let Some(ellipsoidal) = fix.altitude_ellipsoid_meters {
            self.instruments.altitude_msl_meters = Some(msl_meters(fix.position, ellipsoidal));
        }
        if let Some(track) = fix.track_degrees {
            self.instruments.track_degrees = Some(track);
        }
        if let Some(speed) = fix.ground_speed_meters_per_second {
            self.instruments.ground_speed_meters_per_second = Some(speed);
        }

        if self.instruments == before {
            return Vec::new();
        }

        vec![Effect::emit(Topic::Instruments(self.instruments))]
    }
```

and the conversion as a free function in the same module:

```rust
/// Android reports height above the WGS84 ellipsoid. The geoid differs from
/// it by up to about 107 m, far more than any altimetry the app will do can
/// tolerate.
fn msl_meters(position: LatLon, ellipsoidal_meters: f64) -> f64 {
    let at = updraft_geo::LatLon::from_degrees(position.latitude_degrees, position.longitude_degrees);
    let ellipsoidal = updraft_units::EllipsoidAltitude::new(updraft_units::Length::from_meters(
        ellipsoidal_meters,
    ));

    updraft_egm96::ellipsoidal_to_msl(at, ellipsoidal)
        .into_inner()
        .as_meters()
}
```

A `None` field leaves the previous value in place, matching how a partial NMEA sentence behaves. Wire `mod fix;` and `pub use fix::Fix;` into `lib.rs` in alphabetical position.

- [x] **Step 5: Run the tests to verify they pass**

Run: `cargo test -p updraft_core --all-features`
Expected: PASS.

- [x] **Step 6: Pin that both position sources agree**

`Input::InternalGps` is a second path into the same state and nothing yet checks the two agree. Add to `libs/updraft_core/tests/scenario.rs`, reusing the existing `describe` helper rather than writing a second formatter:

```rust
/// A GNSS fix and the equivalent NMEA sentence must leave the core in the
/// same state, or the two position sources disagree about what the aircraft
/// is doing.
#[test]
fn gnss_fix_and_an_equivalent_sentence_agree() {
    let mut from_sentence = Core::new(CoreConfig {
        connections: vec![(LINK, ConnectionSpec::tcp("127.0.0.1", 4353))],
    });
    let effects = from_sentence.apply(
        Input::bytes(
            LINK,
            b"$GPRMC,120000.00,A,5049.38,N,00611.16,E,45.0,270.0,010126,,,A\r\n".as_slice(),
        ),
        Timestamp::from_millis(0),
    );

    let mut from_fix = Core::new(CoreConfig::default());
    let equivalent = from_fix.apply(
        Input::InternalGps(Fix {
            position: LatLon {
                latitude_degrees: 50.823,
                longitude_degrees: 6.186,
            },
            // RMC carries no altitude, so neither may this fix.
            altitude_ellipsoid_meters: None,
            track_degrees: Some(270.0),
            ground_speed_meters_per_second: Some(45.0 * 1852.0 / 3600.0),
        }),
        Timestamp::from_millis(0),
    );

    let rendered = |effects: &[Effect]| effects.iter().map(describe).collect::<Vec<_>>();
    assert_eq!(rendered(&effects), rendered(&equivalent));
}
```

Add `Fix` and `LatLon` to the file's `use updraft_core::{…}` list. `describe` rounds to quantity-appropriate precision, which is what makes comparing a parsed sentence against a floating-point fix meaningful rather than brittle.

- [x] **Step 7: Run and commit**

Run: `cargo test -p updraft_core --all-features && cargo clippy -p updraft_core --all-targets --all-features -- -D warnings`
Expected: PASS, clean.

```bash
git add libs/updraft_core Cargo.lock
git commit -m "core: Accept fixes from the device's GNSS receiver"
```

---

### Task 2: Plugin scaffold

An empty plugin that builds for both Android and desktop and is registered in the app. No behaviour yet — this task exists so the next one starts from a known-good build.

**Files:**

- Create: `libs/tauri_plugin_updraft/` (whole crate)
- Modify: `tauri/Cargo.toml`, `tauri/src/lib.rs`, `tauri/capabilities/default.json`, `.github/workflows/ci.yml`

**Interfaces:**

- Produces: `tauri_plugin_updraft::init()` and an extension trait on `AppHandle` exposing `start_session` and `stop_session`

- [x] **Step 1: Scaffold**

```bash
pnpm tauri plugin new updraft-mobile --android --no-api --no-example --directory libs
```

`--no-api` because the frontend never calls this plugin — the shell does, and the frontend sees only topics. `--no-example` because the repository is the example.

The scaffolded crate is named `tauri-plugin-updraft` — the conventional Tauri plugin crate name, matching `tauri-plugin-*` naming used across the Tauri ecosystem — in the directory `libs/tauri_plugin_updraft`, following the repository's convention that a `libs/*` directory name equals its package name with underscores. Set the Android package to `aero.updraft.mobile` with the plugin class `UpdraftMobilePlugin`. The Tauri plugin identifier is `updraft`, derived from the crate name with its `tauri-plugin-` prefix stripped, and `updraft:default` is the corresponding capability entry.

- [x] **Step 2: Reduce the scaffold to two commands**

The generated plugin ships a `ping` example. Replace it with `startSession` and `stopSession`, both stubs returning `Ok` for now. Keep the generated `desktop.rs` as a no-op so the crate builds on macOS, and keep `error.rs`'s error type — task 3 gives it real variants.

`src/mobile.rs` registers the Android side:

```rust
let handle = api.register_android_plugin("aero.updraft.mobile", "UpdraftMobilePlugin")?;
```

- [x] **Step 3: Register in the app**

Add the plugin as a dependency of `updraft_tauri`, register it with `.plugin(tauri_plugin_updraft::init())`, and add `updraft:default` to `tauri/capabilities/default.json`.

- [x] **Step 4: Keep CI honest**

The plugin depends on `tauri`, so the workspace job cannot build it without webkit system dependencies. In `.github/workflows/ci.yml`, add `--exclude tauri-plugin-updraft` to the three `--workspace` cargo invocations in the first job, and add `cargo test -p tauri-plugin-updraft --all-features` to the `tauri` job beside the existing `updraft_tauri` test step.

Milestone 1 excluded a crate from CI and ended up with four tests that ran nowhere. The second half of this step is the part that matters.

- [x] **Step 5: Verify both targets build**

```bash
cargo check --workspace --exclude tauri-plugin-updraft
cargo check -p tauri-plugin-updraft
cargo check -p tauri-plugin-updraft --target aarch64-linux-android
```

Expected: all succeed. The last needs `NDK_HOME` set.

- [x] **Step 6: Verify the Android app assembles**

```bash
pnpm tauri android build --debug --target aarch64
```

Expected: an APK is produced. Slow the first time.

- [x] **Step 7: Commit**

```bash
git add libs/tauri_plugin_updraft Cargo.lock tauri .github
git commit -m "mobile: Add the plugin scaffold"
```

---

### Task 3: The foreground service

The Kotlin that keeps the process alive. No location yet: the service starts with the `location` type bit while nothing produces fixes, which the spike confirmed is legal as long as the permission is held.

**Files:**

- Create: `libs/tauri_plugin_updraft/android/src/main/java/SessionService.kt`
- Modify: `libs/tauri_plugin_updraft/android/src/main/java/UpdraftMobilePlugin.kt`, `libs/tauri_plugin_updraft/android/src/main/AndroidManifest.xml`

- [x] **Step 1: Declare exactly what this task uses**

In `libs/tauri_plugin_updraft/android/src/main/AndroidManifest.xml`, above `<application>`:

```xml
    <uses-permission android:name="android.permission.ACCESS_FINE_LOCATION" />
    <uses-permission android:name="android.permission.ACCESS_COARSE_LOCATION" />
    <uses-permission android:name="android.permission.FOREGROUND_SERVICE" />
    <uses-permission android:name="android.permission.FOREGROUND_SERVICE_LOCATION" />
    <uses-permission android:name="android.permission.WAKE_LOCK" />
    <uses-permission android:name="android.permission.POST_NOTIFICATIONS" />
```

and inside `<application>`:

```xml
        <service
            android:name="aero.updraft.mobile.SessionService"
            android:exported="false"
            android:foregroundServiceType="location" />
```

These live in the plugin's manifest rather than the app's because the plugin owns the `SessionService` class and every one of these permissions serves plugin functionality, so the plugin carries its own requirements into any app that uses it. `tauri/gen/android/` is generated, so declarations placed there are lost whenever `tauri android init` runs again. Manifest merger unions the library's entries into the app.

`WAKE_LOCK` because this task acquires one, `POST_NOTIFICATIONS` because a foreground service must show one. `connectedDevice` and its permission arrive in milestone 3 with Bluetooth.

`ACCESS_COARSE_LOCATION` is required alongside the fine permission, not a widening of it: since Android 12 a runtime request for `ACCESS_FINE_LOCATION` alone is ignored, and lint enforces the pair at error severity. The pilot answers one Precise-or-Approximate dialog either way. Approximate must still be treated as a refusal — a flight computer needs precise fixes — which the plugin gets by aggregating both strings under a single permission alias, so the alias is granted only when both are.

Deliberately absent: `ACCESS_BACKGROUND_LOCATION`.

- [x] **Step 2: Write the service**

`SessionService.kt` needs:

- A companion object holding static slots for the `app.tauri.plugin.Channel` and start listener handed over by the plugin before the service starts. Neither can travel in the `Intent`, which is why the handover is static.
- `onStartCommand()` handling a start action: enter the foreground, start GPS fixes on the pending channel, acquire the wake lock, and report the result to the listener.
- `doStartForeground()` creating a `NotificationChannel` at `IMPORTANCE_LOW`, building an ongoing `Notification`, and calling `startForeground(NOTIF_ID, notification, FOREGROUND_SERVICE_TYPE_LOCATION)` on SDK ≥ 29, plain `startForeground(NOTIF_ID, notification)` below.
- A partial wake lock from `PowerManager`, released in `onDestroy()`.
- `START_STICKY` after a successful start. A null intent means the system restarted us with no session to resume, so call `stopSelf()` and return `START_NOT_STICKY`.
- `onDestroy()` stopping GPS, clearing the channel, and releasing the wake lock. `SessionService.stop()` delegates control to `Context.stopService()`.

Return the failure from `doStartForeground` rather than swallowing it. The spike found a failed `startForeground` does **not** trigger the usual ANR — the service stays alive as a plain started service — so a swallowed `SecurityException` looks exactly like a working session that never produces fixes.

- [x] **Step 3: Verify on the emulator**

```bash
~/Library/Android/sdk/emulator/emulator -avd spike-api34 -no-snapshot-load &
```

Wait for boot, install, grant location permission, start a session, then:

```bash
adb shell dumpsys activity services aero.updraft | grep -i 'isForeground\|foregroundServiceType'
```

Expected: `isForeground=true` and a type mask containing the location bit (`0x8`).

- [x] **Step 4: Verify the failure paths**

Both are silent-degradation traps, so check them deliberately:

- Revoke location permission and start a session. Expected: `SecurityException` naming `FOREGROUND_SERVICE_LOCATION`, surfaced as a typed error rather than a service that quietly is not foreground.
- Background the app, then start a session. Expected: `ForegroundServiceStartNotAllowedException`. This is why task 5 starts the session while the activity is visible.

- [x] **Step 5: Commit**

```bash
git add libs/tauri_plugin_updraft tauri
git commit -m "mobile: Add the Android foreground service"
```

---

### Task 4: `prevent_exit()` and surviving activity destruction

Without this the previous task's service dies two seconds after the user swipes the app away. Five lines of Rust and a careful verification.

**Files:**

- Modify: `tauri/src/lib.rs`

- [x] **Step 1: Handle `ExitRequested`**

`run()` currently ends with `.run(tauri::generate_context!())`. Change it to build, then run with an event handler:

```rust
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|_app, event| {
            if let tauri::RunEvent::ExitRequested { api, .. } = event {
                // tao's Android event loop calls `std::process::exit` when the
                // last window closes, which kills the foreground service with
                // it. A session has to outlive the activity that started it.
                api.prevent_exit();
            }
        });
```

- [x] **Step 2: Verify the process survives a recents swipe**

Start a session, note `adb shell pidof aero.updraft`, swipe the app from recents, then re-check both the pid and:

```bash
adb shell dumpsys activity services aero.updraft | grep -i isForeground
```

Expected: unchanged pid, still foreground. Before this task the process would be gone.

- [x] **Step 3: Verify the other destruction path**

```bash
adb shell settings put global always_finish_activities 1
```

Repeat step 2, then set it back to `0`. Both paths destroy the activity and both must now leave the process alive.

- [x] **Step 4: Commit**

```bash
git add tauri
git commit -m "tauri: Keep the process alive when the activity is destroyed"
```

---

### Task 5: GNSS fixes into the core

The data path: `GPS_PROVIDER` to Kotlin to `Channel` to `Input::InternalGps` to a topic to the map.

**Files:**

- Create: `libs/tauri_plugin_updraft/android/src/main/java/GpsSource.kt`, `libs/tauri_plugin_updraft/src/models.rs`, `tauri/src/session.rs`
- Modify: `libs/tauri_plugin_updraft/src/mobile.rs`, `tauri/src/lib.rs`

- [x] **Step 1: Emit fixes from Kotlin**

`GpsSource.kt` requests updates from `LocationManager` with **`GPS_PROVIDER`**, not `FUSED_PROVIDER` and not Google Play Services' `FusedLocationProviderClient`.

Every open-source peer does the same: XCSoar hard-codes `GPS_PROVIDER` (`android/src/InternalGPS.java:36`), LK8000 likewise with `NETWORK_PROVIDER` commented out beside it (`InternalGPS.java:65-66`), and Enroute reaches raw `LocationManager` through Qt Positioning with no GMS involvement. Both fusion implementations are tuned for pedestrian and road use and apply smoothing that lags during sustained turns and vertical rate changes — which is what thermalling is — and both can blend in network-derived positions when GNSS is weak. A cell-derived fix can be kilometres off and would corrupt track, ground speed and every glide calculation with no obvious "no fix" signal. The GMS client additionally does not exist on de-Googled devices.

One caveat worth knowing: some OEMs throttle raw `GPS_PROVIDER` callbacks under Doze more aggressively than they throttle GMS-privileged apps. If device testing shows that, the fallback is AOSP's `LocationManager.FUSED_PROVIDER`, never the GMS client.

Post each fix to the session channel as JSON:

```json
{ "latitudeDegrees": 50.823, "longitudeDegrees": 6.186, "altitudeEllipsoidMeters": 247.0, "trackDegrees": 270.0, "groundSpeedMetersPerSecond": 23.15 }
```

**Check every `has*()` before reading its value.** `Location` returns `0.0` rather than null for anything unset, so `getAltitude()`, `getBearing()`, `getSpeed()` and `getAccuracy()` each need their `hasAltitude()`, `hasBearing()`, `hasSpeed()` and `hasAccuracy()` guard, sending `null` when absent. Skip this and a stationary glider reports a confident track of due north at sea level.

- [x] **Step 2: Turn channel messages into inputs**

Create `tauri/src/session.rs` holding the adapter: it builds the `Channel` whose closure deserializes a fix and calls `handle.send(Input::InternalGps(fix))` on the `DriverHandle`. Same shape as the TCP transport feeding `Input::bytes`, and for the same reason — the shell converts wire to domain, the core stays pure.

- [x] **Step 3: Start the session when the app is ready**

In `run()`'s `setup`, on Android only, start a session after the driver is spawned. It must be started while the activity is visible.

Request `ACCESS_FINE_LOCATION` before starting and surface a denial rather than swallowing it. A session that silently fails to start is indistinguishable from a GPS with no signal, and the pilot will be looking at a map that never moves.

- [x] **Step 4: Verify end to end**

With a session running:

```bash
adb emu geo fix 6.186 50.823
```

Expected: the ownship symbol appears at that position. Move it and confirm the symbol follows.

- [x] **Step 5: Verify fixes survive backgrounding**

Press home, wait two minutes, confirm from logcat that fixes still arrive. Then swipe from recents and confirm the same. This is the milestone's deliverable.

- [x] **Step 6: Commit**

```bash
git add libs/tauri_plugin_updraft tauri
git commit -m "mobile: Feed GNSS fixes into the core"
```

---

### Task 6: Spike the public-API path for webview re-creation

A timeboxed investigation, not a feature. It decides whether task 7 is a few lines of Kotlin or a patched dependency.

**The question:** can the app learn about a relaunch and rebuild its window using only public APIs?

The known-good fix needs a `tauri-runtime-wry` patch because the mobile `Resumed` event is dropped when no windows exist. The untried alternative: the plugin's Kotlin registers `Application.ActivityLifecycleCallbacks`, notices the activity being created, and calls into Rust, which rebuilds the window through `AppHandle::run_on_main_thread`. Both halves are public API.

**The uncertainty:** tao's `Window::new` claims the "next available activity" via `ndk_glue::next_available_activity()`. Whether a window built from a `run_on_main_thread` closure — rather than from inside the event loop — attaches to the relaunched activity is exactly what nobody has tested.

- [x] **Step 1: Reproduce the bug on the current build**

Start a session, swipe from recents, relaunch from the launcher. Expected: blank white webview, no JS execution, and `adb shell dumpsys activity top | grep -c ViewRoot` reporting zero webview views. Confirm the Rust side is alive underneath by checking fixes still appear in logcat.

- [x] **Step 2: Try the public-API path**

Register `ActivityLifecycleCallbacks` in `UpdraftMobilePlugin.kt`, invoke a Rust command on activity creation, and from it:

```rust
let _ = app.run_on_main_thread(move || {
    use tauri::Manager;
    if app_handle.webview_windows().is_empty() {
        let _ = tauri::WebviewWindowBuilder::new(
            &app_handle,
            "main",
            tauri::WebviewUrl::default(),
        )
        .build();
    }
});
```

- [x] **Step 3: Record the outcome**

Write the result into this plan, directly under task 7, as the decision that determines which branch task 7 takes. Include what was observed, not just the verdict. A negative result is the valuable one: it justifies carrying a patched dependency, and the justification needs to live where the next person will find it.

- [x] **Step 4: Commit**

```bash
git add -u
git commit -m "docs: Record whether webview re-creation works through public APIs"
```

---

### Task 7: Webview re-creation

**Task 6 answered: the public-API path works. Do not fork `tauri-runtime-wry`.**

Measured on `spike-api35` (API 35, WebView 124), package `aero.updraft.debug`. The working shape is four pieces, all public API:

1. The plugin's Kotlin registers `Application.ActivityLifecycleCallbacks` and reports each stage on a `tauri::ipc::Channel`. The `Application` outlives every activity, and the channel reaches Rust through `PluginManager.sendChannelData`, a JNI native — no webview involved, which is why it still works while the UI is dead.
2. Rust acts on **`onActivityStarted`, never `onActivityCreated`** — see the abort below. This is a hard constraint, not a preference.
3. Rust offers the rebuild to `AppHandle::run_on_main_thread` **repeatedly** until `webview_windows()` is non-empty, rather than once.
4. **`onActivityDestroyed` withdraws permission to build**, and the offer re-checks that permission on the event loop before it builds anything. Without it, offers 2..n of the retry re-open the very abort that (2) closes.

The window built from that closure attaches to the relaunched activity: `webview_windows()` reads 0 before and 1 after, `RustWebView` appears in the activity's view hierarchy, a new DevTools page appears beside the old one, and the pid never changes. `next_available_activity()` is not the obstacle. On a destroy-and-relaunch the old activity's context is removed, so the new one is the only candidate left — but that removal is conditional on `!is_changing_configurations` (`ndk_glue.rs:686-694`), and what keeps a single candidate in the configuration-change case instead is the reused `ActivityId`: `WryActivity.onCreate` restores its `id` from the saved instance state, so `CONTEXTS.insert` overwrites its own entry. (`window_created`, the field that filter reads, is never set `true` anywhere in tao 0.35.3.)

**Verified live, not merely present.** Moving the mocked position *while the app was destroyed* and then relaunching produced fresh tiles of the new city with the ownship on the new position, three cycles running (Paris, Munich, Vienna). That needs the webview to run JS, fetch tiles, re-subscribe over IPC, and receive the driver's topic replay — a rebuilt-but-dead webview cannot fake it. The fix stream was uninterrupted across every cycle.

#### The constraint: `onActivityCreated` aborts the process

That callback fires from inside `Activity.onCreate`'s own `super` call, before `WryActivity.onCreate` reaches `Rust.onActivityCreate` — which is what registers the activity with wry. tao's `Window::new` reads `ndk_glue::CONTEXTS`, which `onActivityCreate` fills first (`ndk_glue.rs:426-435`), so it succeeds; wry then reads `ACTIVITY_PROXY`, which the same call fills a few JNI hops later at `register_activity_proxy` (`ndk_glue.rs:437` → `wry/src/android/mod.rs:152`), and finds nothing:

```
panicked at wry-0.55.1/src/android/mod.rs:189:54: no available activity
F/libc: Fatal signal 6 (SIGABRT) in tid 3909 (Thread-2)
#03 _RINvCs..._17updraft_tauri_lib11stop_unwindNvB2_3run
I/ActivityManager: Process aero.updraft.debug (pid 3859) has died: fg TOP
```

tauri wraps the mobile entry point in `stop_unwind`, so the panic becomes `process::abort()` and takes the foreground service with it — the exact failure milestone 2 exists to prevent. Observed once in the handful of trials run before the variant was abandoned; no rate was measured, and none is needed, because the severity settles it. `onActivityStarted` cannot run before `onCreate` has returned, so it cannot race the registration: zero panics in every trial since.

#### Why the retry is needed, and what it costs

A single `run_on_main_thread` is not enough. It returns `Ok(())` and the closure is sometimes never called: tao's `send_event` discards the `try_send` result (`tao/.../android/mod.rs:541-545`), and the loop parks in `ALooper_pollAll`, which drops a wake that shares a poll iteration with an ident-based fd response (`ndk-0.9.0/src/looper.rs:173`) — exactly what the ndk_glue pipe burst of a relaunch produces. With no window, nothing else wakes the loop. Before the retry existed, one such stall left the screen blank for **51 seconds**, ending only when an unrelated foregrounding woke the loop:

```
11:47:15.682  INFO Activity transition stage=started        <- dispatch accepted, closure queued
              (no "on the main thread" line, screen blank, RustWebView=0 for 51 s)
11:48:07.063  INFO Activity transition stage=started        <- unrelated foregrounding wakes the loop
11:48:07.063  INFO Activity started, on the main thread windows=0   <- the stale closure, finally
11:48:07.064  INFO Rebuilt the webview window
11:48:07.064  INFO Activity started, on the main thread windows=1   <- the fresh one
```

Those last three lines are also the fix. The loop's `while let Ok(event) = self.receiver.try_recv()` (`tao/.../android/mod.rs:417`) drains *everything* queued as soon as any wake lands, so a later offer at a quiet moment drags the stale one through with it. Re-offering every 200 ms is therefore all that is required.

**Measured, 36 in-process relaunches, 24 of them cold** (force-stop, cold start, swipe from recents, relaunch — the sequence where the pipe burst is worst):

| time to rebuild | cold (24) | warm (12) | all (36) |
|---|---|---|---|
| 0-2 ms, first offer landed | 14 | 10 | 24 |
| 201-203 ms, second offer | 10 | 2 | 12 |
| third offer or later | 0 | 0 | 0 |
| never | 0 | 0 | 0 |

Zero panics, zero pid changes, zero `Gave up`/`Could not rebuild`/`Could not reach` lines. Worst case observed: **203 ms**, imperceptible. The cold sequence loses the wake more often (10/24 vs 2/12), which is the predicted signature of the pipe burst.

The cost when a window already exists is nil: the loop exits on its first `webview_windows()` check, with zero dispatches and zero log lines.

#### Why the retry has to be cancelled, and why patience is short

`started` only constrains when the *first* offer is made. Offers 2..n execute whenever the loop next wakes, which can be inside a *later* activity's `onCreate` — the window between `CONTEXTS.insert` and `register_activity_proxy` described above. `onActivityCreate` runs on the Java UI thread while the event loop runs on the thread spawned at `ndk_glue.rs:352`, so the two are genuinely concurrent (task 6's own SIGABRT is tid 3909 in pid 3859). The reachable sequence: a rebuild fails, or the activity goes away before a window is built, so the loop keeps offering; the pilot relaunches inside that window; a pending offer lands mid-`onCreate` and aborts the process, taking the foreground service with it. The existing `webview_windows()` guards do not help — during that window the count *is* empty, which is the loop's own precondition.

Two bounds close it:

- **`onActivityDestroyed` clears the flag that permits building, and the offer re-checks the flag on the event loop.** Cancelling only the loop is not enough: an offer already queued in tao's channel runs whenever the loop next wakes, no matter what the loop decided afterwards. The check has to be inside the closure, and it is the one guard that closes the abort.

  The clear wins the race against the next `onCreate` **by ordering, not by margin**. `Channel.sendObject` calls its handler inline (tauri's `mobile/android/.../plugin/Channel.kt`); the handler is `PluginManager.sendChannelData`, an `external fun`; its JNI entry point calls `send_channel_data` (tauri's `src/lib.rs`), which reaches `Channel::send` and from there the app's channel closure on the calling thread (`src/plugin/mobile.rs`, `src/ipc/channel.rs`). Nothing in that chain posts to a queue, so the flag is cleared inside `onActivityDestroyed`, on the Java UI thread, before that thread can enter any later `onCreate`.

  Do not restate this as a timing margin. The seconds between a swipe and a launcher relaunch are incidental, and they vanish for the recreations this manifest still permits: `configChanges` omits `density`, `fontScale` and `layoutDirection`, so a display- or font-size change destroys and recreates the activity back to back on that one thread. That one case is independently harmless — `isChangingConfigurations` is true for it, so tao emits no `Destroyed` event and keeps its `CONTEXTS` entry, `webview_windows()` stays non-empty, and the offer returns on its first predicate — but the ordering is what covers the general case, and it is the only thing that does. Moving the channel handler onto a task, or making the Kotlin `report()` post to a background handler, would satisfy a margin argument and silently reopen a process abort.
- **Patience is 10 offers, two seconds.** The measured distribution is bimodal at the first and second offer and never reached a third; ten is an order of magnitude past the worst case. Longer patience buys nothing a pilot notices and keeps offers pending across a relaunch. Giving up is not a dead end either: the queued offers stay in tao's channel and the loop drains all of them at its next wake from any source, so a rebuild that ran out of patience still lands on its own — the 51-second stall recovered by exactly that route, before any retry existed. A fresh `started` is a second way back, not the only one.

A build that fails stops the loop rather than repeating it. `WebviewWindowBuilder::build()` failing for "no available activity" (the `Err` path of tao's `Window::new`, `mod.rs:657`) is persistent, not a dropped wake, so re-offering only buries the reason under its own repetitions. The next `started` retries it.

#### Corrections to the steps below

- The `dumpsys activity top | grep -c ViewRoot` check in task 6 does not discriminate. The relaunched activity has a `ViewRoot` either way, and the process keeps a DevTools target for the *old* webview, so `@webview_devtools_remote_<pid>` and a `tauri.localhost` page both survive the bug. What does discriminate: the activity's `android:id/content` `ContentFrameLayout` has no children, and `dumpsys activity top | grep -c RustWebView` reads 0.
- `from_config` does not fix an Android layout problem — it cannot. The bare `WebviewWindowBuilder::new` produced a window pixel-identical to the first launch's, because `tauri.conf.json` sets only `width`/`height`/`resizable`/`fullscreen` and tao's `Window::new` carries an explicit `// FIXME this ignores requested window attributes` (`tao/.../android/mod.rs:637`). Use `from_config` anyway, for the reason that survives: a window setting added later is then not silently dropped.
- **Rotation is not exposed, and the earlier note claiming a stale-context hazard was wrong twice over.** The manifest declares `android:configChanges="orientation|...|screenSize|..."`, so Android does not recreate the activity on a rotation at all: rotating produced **zero** lifecycle transitions, left `RustWebView` attached to the same activity and the webview live and re-laid-out. Nothing reaches `onActivityDestroy`, so no context goes stale. Even without `configChanges` the hazard would not exist: `WryActivity.onCreate` restores its `id` from `savedInstanceState` (`ACTIVITY_ID_KEY`), so a recreated activity reuses the same `ActivityId` and `CONTEXTS.insert` **overwrites** the entry rather than leaving `next_available_activity` a second candidate. And `remove_activity_proxy` is not dead code: `WryActivity.onDestroy` calls `Rust.onWebviewDestroy`, which routes `WebViewMessage::OnDestroy` to `main_pipe.rs:487-497`. Measured end to end: rotate → swipe away → relaunch rebuilt in 206 ms onto a landscape `RustWebView{0,0-2400,1080}` showing fresh tiles of a city set while the app was destroyed, same pid; rotate → rotate back → swipe → relaunch rebuilt in 201 ms.

Take the public-API path:

- [x] **Step 1: Report activity transitions and offer the rebuild**

Build the shape the spike measured. The plugin's Kotlin registers `Application.ActivityLifecycleCallbacks` and reports each stage on a `tauri::ipc::Channel`, the plugin exposes `watch_activities`, and the app turns a transition into repeated offers of the rebuild to `AppHandle::run_on_main_thread`.

Trigger on `started`, never `created`. The `created` abort is the spike's own finding, and it is why the trigger is not the obvious one.

- [x] **Step 2: Bound the retry**

An unbounded loop offers until it succeeds and repeats a persistent build failure every time. Cancel on `destroyed` — in the closure, not only in the loop — stop on a build error, and cut patience to ten offers. See the section above for why each is load-bearing.

- [x] **Step 3: Build from the window configuration**

The scaffolding uses a bare `WebviewWindowBuilder::new`. Switch it to the configured windows so a setting added later is not silently dropped:

```rust
for config in &app.config().app.windows {
    if config.create
        && let Err(error) = tauri::WebviewWindowBuilder::from_config(app, config)
            .and_then(|builder| builder.build())
    {
        tracing::error!(%error, "failed to recreate the webview");
    }
}
```

This changes nothing visible on Android today — tao ignores window attributes there — so verify it did not regress rather than expecting an improvement.

- [x] **Step 4: Verify three destroy/relaunch cycles**

Swipe from recents and relaunch three times in one process. Each time the UI must render, the map must show the ownship, and the pid must not change. The frontend re-subscribes on load and the driver replays current topics, so the map should repopulate without a gap in the fix stream.

Move the mocked position between cycles. A map showing the *new* position proves the rebuilt webview is live rather than merely present, which window count alone does not.

- [x] **Step 5: Keep the hazards in the record**

Whatever gets refactored, preserve the reasons: the trigger stays on `onActivityStarted` or later, the offer stays a retry rather than a single dispatch, and `destroyed` stays a cancellation the offer itself honours. All three are load-bearing and none is obvious from the code.

The six stage names are a wire contract between `UpdraftMobilePlugin.kt` and `tauri/src/activity.rs` with no type to enforce it. Both sides carry a comment naming the other, and the Rust matches all six so an unrecognised stage warns instead of being dropped. Without that, renaming `"destroyed"` on one side leaves the build permission stuck on — a process abort with nothing in the log.

The per-transition log line stays at `debug!`: six lines per foreground cycle, forever, on a device that logs to file is not worth `info!`.

[tauri#15671](https://github.com/tauri-apps/tauri/issues/15671) and [tauri#15678](https://github.com/tauri-apps/tauri/pull/15678) remain the upstream fix for the same bug. When 15678 merges, the app-level `RunEvent::Resumed` handler becomes the simpler implementation and this scaffolding can go — but there is no reason to fork the crate to get there early.

- [x] **Step 6: Commit**

```bash
git add -u
git commit -m "tauri: Rebuild the webview when the activity is relaunched"
```

---

### Task 8: Milestone verification

A checklist, not new code.

- [x] **Step 1: Full repository verification**

Run: `cargo test --workspace --exclude tauri-plugin-updraft --all-features && cargo test -p tauri-plugin-updraft --all-features && cargo clippy --workspace --exclude tauri-plugin-updraft --all-targets --all-features -- -D warnings && pnpm build && pnpm check && pnpm test && pnpm lint && pnpm test:e2e`
Expected: all pass.

The rebuild's per-transition lines are `debug!`, so run with `UPDRAFT_LOG=debug` when tracing a relaunch. At the default `info` level only `"Watching activity transitions"` and `"Rebuilt the webview window"` appear, and the absence of the rest is the intended quiet rather than a fault. The Kotlin side logs every transition to logcat regardless, under `Tauri/UpdraftMobilePlugin`.

- [x] **Step 2: Emulator matrix**

On `spike-api34`, then `spike-api35`:

| Case | Expected |
| --- | --- |
| Cold start, permission granted | Ownship appears on `adb emu geo fix` |
| Home, 5 min | Fixes still arriving |
| Screen off, 5 min | Fixes still arriving |
| Swipe from recents | Same pid, still foreground |
| Relaunch after swipe | UI renders, ownship present |
| Permission revoked mid-session | Surfaced, not silently dead |
| Force stop, relaunch | Clean recovery |

- [x] **Step 3: Record what the emulator cannot tell us**

Append the results to this plan, keeping failures. Note the known emulator limits: the ongoing notification is not user-dismissible there, and freezer timings are indicative only. Device verification stays open, as does whether the notification should be dismissible at all.

- [x] **Step 4: Commit**

```bash
git add -u
git commit -m "docs: Record the milestone 2 emulator verification"
```

---

## Verification results

Run against the completed milestone, with the debug APK rebuilt from it and
installed on both AVDs. Step 1 is green end to end: the two `cargo test` invocations, `cargo
clippy -D warnings`, `pnpm build`, `pnpm check`, `pnpm test` (17 tests), `pnpm lint`
and `pnpm test:e2e` (1 Playwright test, headless, against a `vite preview` server it
starts itself) all exit 0.

### The matrix

| Case | spike-api35 (WebView 124) | spike-api34 (WebView 113) |
| --- | --- | --- |
| Cold start, permission granted | pass — ownship over Zurich | service, wake lock and fixes pass; **ownship not observable**, blank-canvas limitation below |
| Home, 5 min | pass — 291 fixes / 299.2 s | pass — 280 fixes / 300.0 s |
| Screen off, 5 min | pass — 290 fixes / 299.9 s | pass — 280 fixes / 300.0 s |
| Swipe from recents | pass — pid 13168 both sides, `isForeground=true types=0x8` | pass — pid 16031 both sides, same |
| Relaunch after swipe | pass — `waited_ms=1`, live Paris map | pass — `waited_ms=1`, live Paris map |
| Permission revoked mid-session | pass with a gap, below | pass with a gap, below |
| Force stop, relaunch | pass — new pid, session and ownship back | service, wake lock and fixes pass; **ownship not observable**, same limitation |

### Fix inter-arrival

The emulator's `GPS_PROVIDER` runs at 1 Hz and occasionally skips a fix, which puts a
minority of gaps at 2 s in every window including the foreground baselines. The
skipped-fix fraction is 8.5 % for the API 35 baseline, 7.2 % for the API 34 baseline
and for both API 34 backgrounded windows, and only 2.4 % (home) and 3.1 % (screen
off) for the two API 35 backgrounded windows. So backgrounding does not raise it, and
on API 35 it is the backgrounded windows that skip least — consistent with the
foreground baseline carrying the map's render load while the backgrounded ones do
not. That fraction is what the p95 column reads: 2.00 s where it exceeds 5 %, and
~1.01 s where it does not.

| Window | n | duration | median | p95 | max | gaps > 5 s |
| --- | --- | --- | --- | --- | --- | --- |
| API 35 foreground baseline | 166 | 179.6 s | 1.004 s | 2.002 s | 2.011 s | 0 |
| API 35 home | 291 | 299.2 s | 1.008 s | 1.014 s | 2.013 s | 0 |
| API 35 screen off | 290 | 299.9 s | 1.007 s | 1.015 s | 2.009 s | 0 |
| API 34 foreground baseline | 168 | 179.6 s | 1.003 s | 2.000 s | 2.008 s | 0 |
| API 34 home | 280 | 300.0 s | 1.004 s | 2.002 s | 2.009 s | 0 |
| API 34 screen off | 280 | 300.0 s | 1.004 s | 2.002 s | 2.008 s | 0 |

Task 5's numbers (median 1.003 s, max 2.006 s backgrounded) reproduce.

### What the two soak rows actually exercise

Neither is a Doze test. "Home" leaves `mWakefulness=Awake`, so it exercises
app-standby and the process freezer only. "Screen off" reaches `mWakefulness=Asleep`
and drives light idle to `IDLE` within the window, but deep idle stays `INACTIVE`:
five minutes is far short of the deep-idle thresholds, and an emulator has no
motion sensor to hold it out of them either. Real Doze remains unverified.

### Permission revoked mid-session

Android kills the process on revocation — that is the platform's behaviour, not the
app's. What follows is the app's: `SessionService` is restarted by `START_STICKY`
with a null intent, and the guard fires (`Tauri/SessionService: Restarted without a
session to resume, stopping`), so no session runs without a permission behind it.
Reproduced identically on both API levels.

The pilot sees the whole app vanish to the launcher and the ongoing notification go
with it, which is not subtle. But nothing states the cause.

**The rest of this row was exercised on API 35 only.** Relaunching re-requests the
pair, and Android shows the "approximate to precise" upgrade dialog because
`ACCESS_COARSE_LOCATION` is still granted; answering "Keep approximate location" is
correctly treated as a refusal — no session starts, and the only account of it is
one log line:

```
ERROR updraft_tauri_lib: Failed to start the background session
    error=[permissionDenied] - location permission prompt-with-rationale
```

On screen that reads as a map at the default centre with no ownship symbol and no
explanation.

**Open question:** whether a refused or revoked location permission should say so
in the UI. Today the log is the only place it is named.

The refusal path is API-independent code, so there is no reason to expect API 34 to
differ, but nobody has watched it there.

### API 34's blank canvas

Reproduced exactly as the root-cause analysis in
`docs/superpowers/investigations/2026-07-26-android-webview-blank-map.md` describes:
on the first load of a fresh process the WebView-113 image never composites the WebGL
canvas, so the map area is white while the MapLibre attribution and the language
overlay render normally. It blocked the ownship half of two rows, cold start and
force-stop relaunch — both start a fresh process. It did **not** block the
relaunch-after-swipe row: the rebuilt webview's canvas is created long after that
process produced its first frame, and the Paris map painted with the ownship on it.
Every non-visual criterion on API 34 was observable and passed.

### Emulator limits

- The ongoing notification renders and is not user-dismissible: `flags=0x62`
  (`ONGOING_EVENT|NO_CLEAR|FOREGROUND_SERVICE`), shown under Silent as "Updraft —
  Navigating in the background" with no dismiss affordance. Whether it *should* be
  dismissible on a device is still open.
- Freezer and Doze timings here are indicative only.
- `UPDRAFT_LOG` cannot be set for an emulator-installed app: `setprop
  wrap.aero.updraft.debug` is refused by SELinux. The `debug!` transition lines are
  therefore unavailable, and the Kotlin `Tauri/UpdraftMobilePlugin` lines carry the
  lifecycle evidence instead.

### Still needs a physical device

- Real Doze and OEM background throttling against `GPS_PROVIDER`. Nothing measured
  here speaks to either.
- Whether the WebView-113 canvas bug reaches real hardware. Play-updated devices are
  long past 113; de-Googled, kiosk and e-ink hardware with a pinned WebView are not.
- **Cold start to first map paint.** One API 35 force-stop relaunch took ~12 s from
  launch to a painted map, against `am start -W TotalTime: 5304` for the activity
  alone and ~490 ms cold starts elsewhere in the same run. This was an emulator on a
  loaded host, so the number is not a device number — but it is the one pilot-facing
  latency in this milestone that nobody has budgeted, and only a device can say what
  it really is.

### The release build

`pnpm tauri android build --target aarch64` **fails**, reproducibly, in 16-37 s:

```
Execution failed for task ':app:lintVitalAnalyzeUniversalRelease'
> Unexpected failure during lint analysis (this is a bug in lint or one of the
  libraries it depends on)
  Message: `findFirCompiledSymbol` only works on compiled declarations, but the
  given declaration is not compiled.
  Stack: ... SymbolLightClassForScript.getOwnFields ... UastGradleVisitor
         .visitBuildScript ... LintDriver.checkBuildScripts
```

The crash is inside lint's Kotlin analysis API while it walks the project's
`.gradle.kts` scripts. It is not our code, and it stops the build before packaging.
No release APK has been produced in this tree.

Minification itself is fine, which settles the `SessionService` keep-rule question
carried from task 3 — and settles it more strongly than "it happens to work today".
`:app:minifyUniversalReleaseWithR8 --rerun-tasks` succeeds, and every
reflectively-reached name survives unrenamed:

```
aero.updraft.mobile.SessionService     -> aero.updraft.mobile.SessionService:
aero.updraft.mobile.UpdraftMobilePlugin -> aero.updraft.mobile.UpdraftMobilePlugin:
  ... -> startSession / stopSession / watchActivities
      / locationPermissionResult / notificationPermissionResult
```

**What does the keeping.** Neither project ProGuard file contributes anything: both
`tauri/gen/android/app/proguard-rules.pro` and
`libs/tauri_plugin_updraft/android/proguard-rules.pro` are entirely comment lines.
The rules come from two places, both visible in
`app/build/outputs/mapping/universalRelease/configuration.txt`:

- The plugin's commands and callbacks are kept **by annotation**, from tauri's own
  consumer ProGuard file (`tauri-2.11.5/mobile/android`, `configuration.txt:221-231`):

  ```
  -keep @app.tauri.annotation.TauriPlugin public class * {
    @app.tauri.annotation.Command public <methods>;
    @app.tauri.annotation.PermissionCallback <methods>;
    @app.tauri.annotation.ActivityCallback <methods>;
    @app.tauri.annotation.Permission <methods>;
    public <init>(...);
  }
  -keep @app.tauri.annotation.InvokeArg public class * { *; }
  ```

  `UpdraftMobilePlugin` carries `@TauriPlugin`, its three commands `@Command` and its
  two permission callbacks `@PermissionCallback`, and both argument classes carry
  `@InvokeArg`, so all of them are covered by construction rather than by luck.
- `SessionService` is kept by AAPT2's manifest-derived rules
  (`configuration.txt:723`, `-keep class aero.updraft.mobile.SessionService {
  <init>(); }`), because the plugin's `AndroidManifest.xml` declares it.

So the `consumer-rules.pro` that task 3 found missing was never needed: the framework
ships its own consumer rules, and the manifest generates the service's. Adding a
keep rule to either project file would be redundant, and removing an annotation or
the manifest entry is what would actually break a release build.

`GpsSource` is renamed to `fp`, which is harmless — nothing reaches it by name. The
`Companion` objects and the private `promptForNotifications` are inlined away.

Two things remain untested because the lint crash blocks them: a release APK
installed and run, and therefore whether `app.tauri.Logger`'s `BuildConfig.DEBUG`
gate leaves the release build without the Kotlin log lines this verification leaned
on throughout.

---

## Milestone complete

Updraft keeps navigating on Android after the pilot leaves the app: the foreground service holds the process, GNSS fixes reach the core as typed values with the geoid correction applied, and destroying or relaunching the activity loses neither the session nor the UI.

### Ending a session is deferred

The milestone's contract is survival, and it delivers that. It deliberately does not deliver the other half: **nothing calls `stop_session`.** It exists in all three layers — `libs/tauri_plugin_updraft/src/mobile.rs`, `src/desktop.rs` and `UpdraftMobilePlugin.kt` — and has no caller.

The consequence is worth stating plainly rather than discovering later. Once a session starts, the process holds a partial wake lock and a 1 Hz GNSS subscription for as long as it lives. `prevent_exit()` means swiping the app away does not end it, and the ongoing notification is not user-dismissible and carries no stop action, only a `contentIntent` that reopens the app. Force Stop in system settings is the only way out.

This is a scope decision, not an oversight. A stop action with no settings screen to reach it from is half a feature, so teardown lands with the settings UI in milestone 5, which is also where the device list gains a way to turn a source off. The `stop_session` path stays in place for that caller rather than being deleted and rebuilt.

Until then, treat `stop_session` as **unexercised code across three languages**. Nothing has ever run it, so milestone 5 should verify it rather than assume it works.

### Tauri's activity binding goes stale after a relaunch

`prevent_exit()` buys process survival, and this is what it costs. `run()` never runs a second time, so the plugin instance built from the first activity outlives that activity. Tauri does not re-point itself either: `PluginManager.onActivityCreate` is called from `TauriActivity.onCreate` on every activity creation, including the relaunched one, and returns early once its slot is set.

```kotlin
fun onActivityCreate(activity: AppCompatActivity) {
    // TODO: on destroy, we should change to a different activity
    if (::activity.isInitialized) { return }
```

`PluginManager.onDestroy` forwards to plugins but clears neither the slot nor the three `ActivityResultLauncher`s registered alongside it. Those launchers were created through `activity.registerForActivityResult`, which binds them to that activity's lifecycle and unregisters their keys on `ON_DESTROY`. `PluginHandle.validatePermissions` reads the same stale slot for `shouldShowRequestPermissionRationale`, so the reference reaches the result path as well as the launch.

The blast radius is wider than permissions. `startActivityForResult` and `startIntentSenderForResult` are registered the same way, which is what makes this a milestone 3 problem: the enable-Bluetooth intent and companion device pairing both go through them.

Milestone 2 cannot reach any of it. `startSession` runs once from `setup`, under the first activity, and it is the only caller that requests a permission. The plugin no longer starts the service through the activity either, so nothing here holds a destroyed context.

Verified against tauri 2.11.5, which the workspace pins exactly. What has **not** been verified is the runtime failure itself: the reasoning above is from tauri's source and the androidx API surface, and nobody has watched a permission request fail after a relaunch on a device or an emulator. That check is worth doing before choosing a fix, because it also settles whether the call throws or hangs.

**Open question:** which of these to take, and when.

1. **Register a launcher against the live activity.** `ActivityResultRegistry` carries a `register(String, ActivityResultContract, ActivityResultCallback)` overload that takes no `LifecycleOwner` and therefore no "register before STARTED" restriction, and `ActivityResultLauncher.unregister()` is public. Both are present in the `androidx.activity:activity-ktx:1.13.0` the app resolves to. `watchActivities` already tracks the current activity. A stable key matters: the registry parks a result for an unregistered key and delivers it once one registers again, which covers a configuration change while the dialog is up. Self-contained, needs no upstream change and no edit to generated code, at the price of owning a slice of what `requestPermissionForAlias` does today.
2. **Re-point `PluginManager.activity`.** It is a public `lateinit var`, and the plugin sees `onActivityCreated`. It is also half a fix: the launchers are private and only assigned inside the guarded block, so the object would look consistent while `launch()` still failed. Reaching the private fields by reflection would need R8 keep rules and would break on any upstream change. Rejected.
3. **Fix it upstream.** Drop the early return, re-register the launchers, clear the slot in `onDestroy` when the destroyed activity is the current one. `TauriActivity.onCreate` already runs before STARTED, so the change is small, and the `TODO` says upstream knows. The right home for it, but a PR, a release and a pin bump do not arrive on milestone 3's schedule.
4. **Use `ActivityCompat.requestPermissions` with `onRequestPermissionsResult` in `MainActivity.kt`,** which comes from a tauri-cli template rather than the `generated/` tree and is therefore editable. Sidesteps the registry entirely, but moves plugin logic into the app and gives up the property this plugin is built around: that an app gets a working session from the dependency alone.
5. **Collect every permission up front, while the first activity is alive.** Does not fix anything, but it makes the broken path rare, and asking a pilot for permissions on the ground rather than mid-flight is better behaviour regardless.

Options 1 and 3 compose: 3 is the fix, 1 is what carries milestone 3 until 3 lands. Option 5 stands on its own merits either way.

Milestone 3 adds the Bluetooth SPP transport and FLARM traffic, joining the same plugin, adding the `connectedDevice` type bit and the command to switch the mask at runtime.
