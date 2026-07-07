# Draft new issue for tauri-apps/tauri

Fields follow the bug report template. Suggested title:

> [bug] Android: blank webview when relaunching the app after task removal
> while a foreground service keeps the process alive

---

### Describe the bug

When a foreground service keeps the app process alive and the user swipes
the app away from recents and then relaunches it from the launcher, the
relaunched activity stays a plain white screen. No webview is created, JS
never executes (zero console output). A second relaunch does not recover,
only killing the process does. The Rust side keeps running fine the whole
time.

Reproduced with current versions (tauri 2.11.5, tao 0.35.3, wry 0.55.1) on
Android 14 and 15 emulator images. Minimal standalone reproduction below.

Possibly related: #11609 reported this scenario (foreground service keeps
the process alive, app unusable on next launch). The activity memory leak
from that issue was fixed and the issue was closed after the original
reproduction disappeared, but the relaunch case itself is still broken.
The `__TAURI_INVOKE_KEY__` errors quoted there came from wry's
process-wide statics in pre-leak-fix versions. On current versions no
webview is created at all, so there is nothing left to log. The surface
symptom changed along the way, but the scenario is the same: a process
that outlives its activity does not get a working webview back.

#### Root cause

Three layers are involved when a foreground service keeps the process alive
and the user swipes the app away from recents, then relaunches it:

1. Without countermeasures the process does not even survive: destroying
   the last activity closes the webview window and Tauri's default
   exit-on-last-window-close makes tao's Android `EventLoop::run` call
   `std::process::exit(0)`, killing the FGS with it. An app can prevent
   that with `RunEvent::ExitRequested` + `api.prevent_exit()` (used in the
   reproduction below).
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

### Reproduction

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

(The swipe coordinates fit the 1080x2400 pixel_7 emulator image, on other
devices dismiss the recents card manually.)

Result: blank white screen, no webview content, no JS execution. A second
relaunch does not recover, only killing the process does. The Rust side
keeps running fine the whole time.

### Expected behavior

Relaunching the app while the FGS-pinned process is still alive shows a
working webview again, like a fresh cold start does.

### Full `tauri info` output

```text
[✔] Environment
    - OS: Mac OS 15.7.4 arm64 (X64)
    ✔ Xcode Command Line Tools: installed
    ✔ Xcode: 26.3
    ✔ rustc: 1.96.0 (ac68faa20 2026-05-25)
    ✔ cargo: 1.96.0 (30a34c682 2026-05-25)
    ✔ rustup: 1.29.0 (28d1352db 2026-03-05)
    ✔ Rust toolchain: stable-aarch64-apple-darwin (default)
    - node: 24.11.1
    - pnpm: 11.2.2
    - yarn: 1.22.22
    - npm: 11.6.2
    - bun: 1.2.17

[-] Packages
    - tauri 🦀: 2.11.5
    - tauri-build 🦀: 2.6.3
    - wry 🦀: 0.55.1
    - tao 🦀: 0.35.3
    - tauri-cli 🦀: 2.11.4
    - @tauri-apps/api  ⱼₛ: 2.11.1
    - @tauri-apps/cli  ⱼₛ: 2.11.4

[-] Plugins
    - tauri-plugin-opener 🦀: 2.5.4
    - @tauri-apps/plugin-opener  ⱼₛ: 2.5.4

[-] App
    - build-type: bundle
    - CSP: unset
    - frontendDist: ../dist
    - devUrl: http://localhost:1420/
    - bundler: Vite
```

### Stack trace

Nothing is logged when the blank activity comes up. That is part of the
problem, the resume is silently dropped (see root cause above).

### Additional context

#### Fix, validated end-to-end

Two parts. First, surface the resume to the app in `tauri-runtime-wry`
(in the mobile `Resumed`/`Suspended` arm):

```rust
if matches!(e, Event::Resumed) {
  callback(RunEvent::Resumed);
}
```

(To try this against the published release, copy `tauri-runtime-wry` 2.11.4
out of the cargo registry, add the three lines at the top of the arm, and
point a `[patch.crates-io]` entry at the copy. The `dev` branch has the
identical code in this arm.)

Second, re-create the window when resumed with no webviews. The repro's
`run` closure from above becomes:

```rust
        .run(|app, event| match event {
            tauri::RunEvent::ExitRequested { api, .. } => {
                api.prevent_exit();
            }
            tauri::RunEvent::Resumed => {
                use tauri::Manager;
                if app.webview_windows().is_empty() {
                    tauri::WebviewWindowBuilder::new(app, "main", tauri::WebviewUrl::default())
                        .build()
                        .expect("failed to recreate webview window");
                }
            }
            _ => {}
        });
```

The builder claims the relaunched activity through the existing
`next_available_activity()` machinery. With these two changes the exact
repro above relaunches into a fully working UI: the page renders, `invoke`
round-trips work (verified with the template's `greet` command), and in a
larger app I verified a foreground-service `Channel` stream staying
gap-free across three swipe-away/relaunch cycles in a single process.

#### Proposal

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
