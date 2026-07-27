# Android SDK 37 compile migration

## Context

`androidx.core:core:1.19.0` declares a minimum compile SDK of 37 and a
minimum Android Gradle plugin version of 9.1.0. Updraft currently compiles its
application and mobile plugin against SDK 36, so the Android CI build rejects
that dependency before compilation.

The compile SDK and target SDK have separate responsibilities. Raising the
compile SDK makes API 37 symbols and API-37-requiring dependencies available
to the build. Raising the target SDK opts the application into Android 17
runtime behavior changes. The dependency update requires only the former.

## Decision

Raise the compile SDK of the Updraft application and Updraft mobile plugin from
36 to 37. Keep `targetSdk = 36` and `minSdk = 24`.

The Tauri 2.11 Android library remains compiled against SDK 36. This is an
upstream module outside Updraft's generated application and plugin build
files. Android CI must therefore install both SDK Platform 36 and SDK Platform
37.0.

Keep Android SDK Build Tools 36.0.0. Android Gradle plugin 9.3.1 selects that
version by default and supports API 37. Updraft's Gradle 9.6.1 and Java 21
versions already satisfy the plugin's requirements.

## Repository changes

- `tauri/gen/android/app/build.gradle.kts` sets `compileSdk = 37`.
- `libs/tauri_plugin_updraft/android/build.gradle.kts` sets
  `compileSdk = 37`.
- `.github/workflows/ci.yml` installs `platforms;android-37.0` in addition to
  the existing `platforms;android-36` package.

No dependency version changes belong in this branch. The AndroidX Core update
remains a separate change after this migration lands.

## Target SDK boundary

This branch does not opt into Android 17 runtime behavior. In particular, it
does not:

- request `ACCESS_LOCAL_NETWORK` for future direct LAN instrument
  connections,
- change Bluetooth SPP disconnect handling,
- change native warning-audio or foreground-service behavior,
- change WebView, notification, or large-screen behavior, or
- claim that Updraft has completed Android 17 runtime compatibility testing.

Those concerns belong in a later `targetSdk = 37` migration. Keeping the
boundary explicit avoids adding permission prompts before Updraft exposes a
non-loopback TCP connection to the user.

## Validation

Before implementation, an isolated Gradle application compiled the Updraft
plugin against SDK 37 with `androidx.core:core:1.19.0` while leaving Tauri's
Android library on SDK 36. That check passed and established the package
combination used by this design.

The migration is complete when:

1. The application and plugin both resolve compile SDK 37.0.
2. Tauri's Android library still resolves compile SDK 36.
3. The repository's Android debug APK builds for `aarch64`.
4. Repository formatting and diff checks pass.

Existing Tauri warnings about the legacy Android Gradle plugin DSL and Kotlin
plugin remain upstream constraints. They are not caused by SDK 37 and do not
justify an AGP or Tauri migration in this branch.
