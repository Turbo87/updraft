# Keep the Android screen awake

## Purpose

Updraft must keep the Android display awake while its activity is visible.
Android must not dim the display or lock the device because of the system idle
timer during this time.

The screen can turn off after Updraft is no longer visible. The existing
foreground service must continue to run according to its current lifecycle.

## Decision

`MainActivity.onCreate()` adds
`WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON` to its window. Android applies
this flag only while the window is visible.

Each new `MainActivity` receives the flag. This includes an activity that
Android creates after configuration changes or after the user removes and
reopens Updraft while the foreground service keeps the process alive.

The implementation does not use a Tauri plugin. It does not store an activity
reference outside `MainActivity`. It does not acquire another wake lock.

## Automated test

An Android instrumentation test launches `MainActivity` with
`ActivityScenario`. The test checks that the activity window contains
`FLAG_KEEP_SCREEN_ON`. It then recreates the activity and checks the flag on
the new window.

The test must fail before the production change. It must pass after
`MainActivity.onCreate()` adds the flag.

The Android application configures the standard Android instrumentation test
runner. The new test stays in the application module because that module owns
`MainActivity` and its window behavior.

## Validation

Run the instrumentation test on a connected Android device. Build the Android
debug APK. On a device, keep Updraft idle longer than the configured screen
timeout and confirm that the display stays awake.

Remove Updraft from Recents while its foreground service runs. Reopen Updraft
and repeat the idle check. Confirm that the foreground service behavior does
not change.

## Scope

This change does not add a setting, a runtime toggle, an iOS implementation,
or an emulator-based CI job. It does not change the foreground service or its
partial wake lock.
