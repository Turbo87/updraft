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

- [ ] **Step 1: Scaffold**

```bash
pnpm tauri plugin new updraft-mobile --android --no-api --no-example --directory libs
```

`--no-api` because the frontend never calls this plugin — the shell does, and the frontend sees only topics. `--no-example` because the repository is the example.

The scaffolded crate is named `tauri-plugin-updraft` — the conventional Tauri plugin crate name, matching `tauri-plugin-*` naming used across the Tauri ecosystem — in the directory `libs/tauri_plugin_updraft`, following the repository's convention that a `libs/*` directory name equals its package name with underscores. Set the Android package to `aero.updraft.mobile` with the plugin class `UpdraftMobilePlugin`. The Tauri plugin identifier is `updraft`, derived from the crate name with its `tauri-plugin-` prefix stripped, and `updraft:default` is the corresponding capability entry.

- [ ] **Step 2: Reduce the scaffold to two commands**

The generated plugin ships a `ping` example. Replace it with `startSession` and `stopSession`, both stubs returning `Ok` for now. Keep the generated `desktop.rs` as a no-op so the crate builds on macOS, and keep `error.rs`'s error type — task 3 gives it real variants.

`src/mobile.rs` registers the Android side:

```rust
let handle = api.register_android_plugin("aero.updraft.mobile", "UpdraftMobilePlugin")?;
```

- [ ] **Step 3: Register in the app**

Add the plugin as a dependency of `updraft_tauri`, register it with `.plugin(tauri_plugin_updraft::init())`, and add `updraft:default` to `tauri/capabilities/default.json`.

- [ ] **Step 4: Keep CI honest**

The plugin depends on `tauri`, so the workspace job cannot build it without webkit system dependencies. In `.github/workflows/ci.yml`, add `--exclude tauri-plugin-updraft` to the three `--workspace` cargo invocations in the first job, and add `cargo test -p tauri-plugin-updraft --all-features` to the `tauri` job beside the existing `updraft_tauri` test step.

Milestone 1 excluded a crate from CI and ended up with four tests that ran nowhere. The second half of this step is the part that matters.

- [ ] **Step 5: Verify both targets build**

```bash
cargo check --workspace --exclude tauri-plugin-updraft
cargo check -p tauri-plugin-updraft
cargo check -p tauri-plugin-updraft --target aarch64-linux-android
```

Expected: all succeed. The last needs `NDK_HOME` set.

- [ ] **Step 6: Verify the Android app assembles**

```bash
pnpm tauri android build --debug --target aarch64
```

Expected: an APK is produced. Slow the first time.

- [ ] **Step 7: Commit**

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

- [ ] **Step 1: Declare exactly what this task uses**

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

- [ ] **Step 2: Write the service**

`SessionService.kt` needs:

- A companion object holding static slots for the `app.tauri.plugin.Channel` and start listener handed over by the plugin before the service starts. Neither can travel in the `Intent`, which is why the handover is static.
- `onStartCommand()` handling a start action: enter the foreground, start GPS fixes on the pending channel, acquire the wake lock, and report the result to the listener.
- `doStartForeground()` creating a `NotificationChannel` at `IMPORTANCE_LOW`, building an ongoing `Notification`, and calling `startForeground(NOTIF_ID, notification, FOREGROUND_SERVICE_TYPE_LOCATION)` on SDK ≥ 29, plain `startForeground(NOTIF_ID, notification)` below.
- A partial wake lock from `PowerManager`, released in `onDestroy()`.
- `START_STICKY` after a successful start. A null intent means the system restarted us with no session to resume, so call `stopSelf()` and return `START_NOT_STICKY`.
- `onDestroy()` stopping GPS, clearing the channel, and releasing the wake lock. `SessionService.stop()` delegates control to `Context.stopService()`.

Return the failure from `doStartForeground` rather than swallowing it. The spike found a failed `startForeground` does **not** trigger the usual ANR — the service stays alive as a plain started service — so a swallowed `SecurityException` looks exactly like a working session that never produces fixes.

- [ ] **Step 3: Verify on the emulator**

```bash
~/Library/Android/sdk/emulator/emulator -avd spike-api34 -no-snapshot-load &
```

Wait for boot, install, grant location permission, start a session, then:

```bash
adb shell dumpsys activity services aero.updraft | grep -i 'isForeground\|foregroundServiceType'
```

Expected: `isForeground=true` and a type mask containing the location bit (`0x8`).

- [ ] **Step 4: Verify the failure paths**

Both are silent-degradation traps, so check them deliberately:

- Revoke location permission and start a session. Expected: `SecurityException` naming `FOREGROUND_SERVICE_LOCATION`, surfaced as a typed error rather than a service that quietly is not foreground.
- Background the app, then start a session. Expected: `ForegroundServiceStartNotAllowedException`. This is why task 5 starts the session while the activity is visible.

- [ ] **Step 5: Commit**

```bash
git add libs/tauri_plugin_updraft tauri
git commit -m "mobile: Add the Android foreground service"
```

---

### Task 4: `prevent_exit()` and surviving activity destruction

Without this the previous task's service dies two seconds after the user swipes the app away. Five lines of Rust and a careful verification.

**Files:**

- Modify: `tauri/src/lib.rs`

- [ ] **Step 1: Handle `ExitRequested`**

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

- [ ] **Step 2: Verify the process survives a recents swipe**

Start a session, note `adb shell pidof aero.updraft`, swipe the app from recents, then re-check both the pid and:

```bash
adb shell dumpsys activity services aero.updraft | grep -i isForeground
```

Expected: unchanged pid, still foreground. Before this task the process would be gone.

- [ ] **Step 3: Verify the other destruction path**

```bash
adb shell settings put global always_finish_activities 1
```

Repeat step 2, then set it back to `0`. Both paths destroy the activity and both must now leave the process alive.

- [ ] **Step 4: Commit**

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

- [ ] **Step 1: Emit fixes from Kotlin**

`GpsSource.kt` requests updates from `LocationManager` with **`GPS_PROVIDER`**, not `FUSED_PROVIDER` and not Google Play Services' `FusedLocationProviderClient`.

Every open-source peer does the same: XCSoar hard-codes `GPS_PROVIDER` (`android/src/InternalGPS.java:36`), LK8000 likewise with `NETWORK_PROVIDER` commented out beside it (`InternalGPS.java:65-66`), and Enroute reaches raw `LocationManager` through Qt Positioning with no GMS involvement. Both fusion implementations are tuned for pedestrian and road use and apply smoothing that lags during sustained turns and vertical rate changes — which is what thermalling is — and both can blend in network-derived positions when GNSS is weak. A cell-derived fix can be kilometres off and would corrupt track, ground speed and every glide calculation with no obvious "no fix" signal. The GMS client additionally does not exist on de-Googled devices.

One caveat worth knowing: some OEMs throttle raw `GPS_PROVIDER` callbacks under Doze more aggressively than they throttle GMS-privileged apps. If device testing shows that, the fallback is AOSP's `LocationManager.FUSED_PROVIDER`, never the GMS client.

Post each fix to the session channel as JSON:

```json
{ "latitudeDegrees": 50.823, "longitudeDegrees": 6.186, "altitudeEllipsoidMeters": 247.0, "trackDegrees": 270.0, "groundSpeedMetersPerSecond": 23.15 }
```

**Check every `has*()` before reading its value.** `Location` returns `0.0` rather than null for anything unset, so `getAltitude()`, `getBearing()`, `getSpeed()` and `getAccuracy()` each need their `hasAltitude()`, `hasBearing()`, `hasSpeed()` and `hasAccuracy()` guard, sending `null` when absent. Skip this and a stationary glider reports a confident track of due north at sea level.

- [ ] **Step 2: Turn channel messages into inputs**

Create `tauri/src/session.rs` holding the adapter: it builds the `Channel` whose closure deserializes a fix and calls `handle.send(Input::InternalGps(fix))` on the `DriverHandle`. Same shape as the TCP transport feeding `Input::bytes`, and for the same reason — the shell converts wire to domain, the core stays pure.

- [ ] **Step 3: Start the session when the app is ready**

In `run()`'s `setup`, on Android only, start a session after the driver is spawned. It must be started while the activity is visible.

Request `ACCESS_FINE_LOCATION` before starting and surface a denial rather than swallowing it. A session that silently fails to start is indistinguishable from a GPS with no signal, and the pilot will be looking at a map that never moves.

- [ ] **Step 4: Verify end to end**

With a session running:

```bash
adb emu geo fix 6.186 50.823
```

Expected: the ownship symbol appears at that position. Move it and confirm the symbol follows.

- [ ] **Step 5: Verify fixes survive backgrounding**

Press home, wait two minutes, confirm from logcat that fixes still arrive. Then swipe from recents and confirm the same. This is the milestone's deliverable.

- [ ] **Step 6: Commit**

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

- [ ] **Step 1: Reproduce the bug on the current build**

Start a session, swipe from recents, relaunch from the launcher. Expected: blank white webview, no JS execution, and `adb shell dumpsys activity top | grep -c ViewRoot` reporting zero webview views. Confirm the Rust side is alive underneath by checking fixes still appear in logcat.

- [ ] **Step 2: Try the public-API path**

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

- [ ] **Step 3: Record the outcome**

Write the result into this plan, directly under task 7, as the decision that determines which branch task 7 takes. Include what was observed, not just the verdict. A negative result is the valuable one: it justifies carrying a patched dependency, and the justification needs to live where the next person will find it.

- [ ] **Step 4: Commit**

```bash
git add -u
git commit -m "docs: Record whether webview re-creation works through public APIs"
```

---

### Task 7: Webview re-creation

- [ ] **Step 1: Report activity transitions and offer the rebuild**

Build the shape the spike measured. The plugin's Kotlin registers `Application.ActivityLifecycleCallbacks` and reports each stage on a `tauri::ipc::Channel`, the plugin exposes `watch_activities`, and the app turns a transition into repeated offers of the rebuild to `AppHandle::run_on_main_thread`.

Trigger on `started`, never `created`. The `created` abort is the spike's own finding, and it is why the trigger is not the obvious one.

- [ ] **Step 2: Bound the retry**

An unbounded loop offers until it succeeds and repeats a persistent build failure every time. Cancel on `destroyed` — in the closure, not only in the loop — stop on a build error, and cut patience to ten offers. See the section above for why each is load-bearing.

- [ ] **Step 3: Build from the window configuration**

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

- [ ] **Step 4: Verify three destroy/relaunch cycles**

Swipe from recents and relaunch three times in one process. Each time the UI must render, the map must show the ownship, and the pid must not change. The frontend re-subscribes on load and the driver replays current topics, so the map should repopulate without a gap in the fix stream.

Move the mocked position between cycles. A map showing the *new* position proves the rebuilt webview is live rather than merely present, which window count alone does not.

- [ ] **Step 5: Keep the hazards in the record**

Whatever gets refactored, preserve the reasons: the trigger stays on `onActivityStarted` or later, the offer stays a retry rather than a single dispatch, and `destroyed` stays a cancellation the offer itself honours. All three are load-bearing and none is obvious from the code.

The six stage names are a wire contract between `UpdraftMobilePlugin.kt` and `tauri/src/activity.rs` with no type to enforce it. Both sides carry a comment naming the other, and the Rust matches all six so an unrecognised stage warns instead of being dropped. Without that, renaming `"destroyed"` on one side leaves the build permission stuck on — a process abort with nothing in the log.

The per-transition log line stays at `debug!`: six lines per foreground cycle, forever, on a device that logs to file is not worth `info!`.

[tauri#15671](https://github.com/tauri-apps/tauri/issues/15671) and [tauri#15678](https://github.com/tauri-apps/tauri/pull/15678) remain the upstream fix for the same bug. When 15678 merges, the app-level `RunEvent::Resumed` handler becomes the simpler implementation and this scaffolding can go — but there is no reason to fork the crate to get there early.

- [ ] **Step 6: Commit**

```bash
git add -u
git commit -m "tauri: Rebuild the webview when the activity is relaunched"
```

---

### Task 8: Milestone verification

A checklist, not new code.

- [ ] **Step 1: Full repository verification**

Run: `cargo test --workspace --exclude tauri-plugin-updraft --all-features && cargo test -p tauri-plugin-updraft --all-features && cargo clippy --workspace --exclude tauri-plugin-updraft --all-targets --all-features -- -D warnings && pnpm build && pnpm check && pnpm test && pnpm lint && pnpm test:e2e`
Expected: all pass.

The rebuild's per-transition lines are `debug!`, so run with `UPDRAFT_LOG=debug` when tracing a relaunch. At the default `info` level only `"Watching activity transitions"` and `"Rebuilt the webview window"` appear, and the absence of the rest is the intended quiet rather than a fault. The Kotlin side logs every transition to logcat regardless, under `Tauri/UpdraftMobilePlugin`.

- [ ] **Step 2: Emulator matrix**

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

- [ ] **Step 3: Record what the emulator cannot tell us**

Append the results to this plan, keeping failures. Note the known emulator limits: the ongoing notification is not user-dismissible there, and freezer timings are indicative only. Device verification stays open, as does whether the notification should be dismissible at all.

- [ ] **Step 4: Commit**

```bash
git add -u
git commit -m "docs: Record the milestone 2 emulator verification"
```

---

## Milestone complete

Updraft keeps navigating on Android after the pilot leaves the app: the foreground service holds the process, GNSS fixes reach the core as typed values with the geoid correction applied, and destroying or relaunching the activity loses neither the session nor the UI.

Milestone 3 adds the Bluetooth SPP transport and FLARM traffic, joining the same plugin, adding the `connectedDevice` type bit and the command to switch the mask at runtime.
