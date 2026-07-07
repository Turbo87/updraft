# Draft comment for tauri-apps/tauri#11609 (ask to re-open)

---

I can still reproduce this on current versions (tauri 2.11.5, tao 0.35.3,
wry 0.55.1) and I believe I found the actual root cause, so I'd like to ask
for this to be re-opened. Minimal standalone reproduction below, verified
on Android 14 and 15 emulator images.

## Root cause

Three layers are involved when a foreground service keeps the process alive
and the user swipes the app away from recents, then relaunches it:

1. Without countermeasures the process does not even survive: destroying
   the last activity closes the webview window and Tauri's default
   exit-on-last-window-close makes tao's Android `EventLoop::run` call
   `std::process::exit(0)`, killing the FGS with it. An app can prevent
   that with `RunEvent::ExitRequested` + `api.prevent_exit()` (used below).
2. tao maps one Android Activity to one tao `Window`
   (`WindowId == ActivityId`), and `Window::new` binds to the "next
   available activity" via `ndk_glue::next_available_activity()`. wry can
   even re-create a webview when an activity with a known id comes back
   (`android_setup` replays stored `WEBVIEW_ATTRIBUTES`), but the id in the
   generated `WryActivity.onCreate` is
   `savedInstanceState ?: intent extras ?: hashCode()`. A launcher relaunch
   after task removal has neither saved state nor the extra, so the new
   activity gets a fresh `hashCode()`, nothing matches, and no webview is
   ever created for it. The relaunched activity stays blank and unclaimed.
3. The app cannot react on its own either: in `tauri-runtime-wry` the
   mobile `e @ Event::Resumed | e @ Event::Suspended` arm dispatches only
   per-window events. With zero windows the loop body never runs, so the
   resume is silently dropped. App-level `RunEvent::Resumed` only fires
   from `NewEvents(StartCause::Poll)`, which never happens here.

The webview shows plain white and JS never executes (zero console output),
which matches the reports above (whether you see `__TAURI_INVOKE_KEY__`
noise or nothing seems to depend on timing, the cause is the same).

## Minimal reproduction

```sh
npm create tauri-app@latest tauri-11609-repro -- --template vanilla-ts \
  --manager npm --identifier com.example.repro --yes
cd tauri-11609-repro && npm install
```

Keep the process alive across activity destruction, like any app with a
foreground service needs to. In `src-tauri/src/lib.rs`, replace the
`.run(...)` call of the template with:

```rust
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|_app, event| {
            if let tauri::RunEvent::ExitRequested { api, .. } = &event {
                api.prevent_exit();
            }
        });
```

Then `npm run tauri android init` and add a minimal foreground service to
the generated project (this stands in for any real FGS, e.g. location
tracking):

<details>
<summary><code>src-tauri/gen/android/app/src/main/java/com/example/repro/KeepAliveService.kt</code> (new file)</summary>

```kotlin
package com.example.repro

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.Service
import android.content.Intent
import android.os.IBinder

class KeepAliveService : Service() {
  override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
    val nm = getSystemService(NotificationManager::class.java)
    nm.createNotificationChannel(
      NotificationChannel("keepalive", "Keep alive", NotificationManager.IMPORTANCE_LOW)
    )
    val notification = Notification.Builder(this, "keepalive")
      .setContentTitle("running")
      .setSmallIcon(android.R.drawable.ic_menu_info_details)
      .build()
    startForeground(1, notification)
    return START_STICKY
  }

  override fun onBind(intent: Intent?): IBinder? = null
}
```

</details>

<details>
<summary><code>MainActivity.kt</code> (start the service) and <code>AndroidManifest.xml</code> (permissions + service)</summary>

```kotlin
package com.example.repro

import android.content.Intent
import android.os.Bundle

class MainActivity : TauriActivity() {
  override fun onCreate(savedInstanceState: Bundle?) {
    super.onCreate(savedInstanceState)
    startForegroundService(Intent(this, KeepAliveService::class.java))
  }
}
```

```xml
<!-- next to the INTERNET permission: -->
<uses-permission android:name="android.permission.FOREGROUND_SERVICE" />
<uses-permission android:name="android.permission.FOREGROUND_SERVICE_DATA_SYNC" />

<!-- inside <application>: -->
<service
    android:name=".KeepAliveService"
    android:exported="false"
    android:foregroundServiceType="dataSync" />
```

</details>

Run it and reproduce:

```sh
npm run tauri android build -- --debug --apk
adb install -r src-tauri/gen/android/app/build/outputs/apk/universal/debug/app-universal-debug.apk
adb shell am start -n com.example.repro/.MainActivity   # renders the template UI
# swipe the app away from recents (activity destroyed, FGS keeps the process):
adb shell input keyevent KEYCODE_APP_SWITCH && sleep 2 && adb shell input swipe 540 1080 540 150 250
adb shell pidof com.example.repro                        # still alive
adb shell am start -n com.example.repro/.MainActivity   # relaunch
```

Result: blank white screen, no webview content, no JS execution. A second
relaunch does not recover, only killing the process does. The Rust side
keeps running fine the whole time.

## Fix, validated end-to-end

Two parts. First, surface the resume to the app in `tauri-runtime-wry`
(in the mobile `Resumed`/`Suspended` arm):

```rust
if matches!(e, Event::Resumed) {
  callback(RunEvent::Resumed);
}
```

Second, re-create the window when resumed with no webviews. In the repro's
`run` closure:

```rust
tauri::RunEvent::Resumed => {
    use tauri::Manager;
    if app.webview_windows().is_empty() {
        tauri::WebviewWindowBuilder::new(app, "main", tauri::WebviewUrl::default())
            .build()
            .expect("failed to recreate webview window");
    }
}
```

The builder claims the relaunched activity through the existing
`next_available_activity()` machinery. With these two changes the exact
repro above relaunches into a fully working UI: the page renders, `invoke`
round-trips work (verified with the template's `greet` command), and in a
larger app I verified a foreground-service `Channel` stream staying
gap-free across three swipe-away/relaunch cycles in a single process.

## Proposal

Part two shouldn't be every app's problem. Tauri core could do it: on
Android, when `Resumed` arrives and no webviews exist, re-run the same
config-window loop that `setup()` already uses
(`WebviewWindowBuilder::from_config` for windows with `create: true`).
Zero webviews while resumed is never a healthy state on Android, so I'd
argue for default-on rather than a config flag, but happy to gate it if you
prefer. One thing worth a docs note either way: the recreated webview
starts with fresh frontend state, apps keeping background state in Rust
need to re-sync on load.

I'm happy to submit a PR for both parts.
