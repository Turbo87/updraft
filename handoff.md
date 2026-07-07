# Handoff: Android spikes S1/S2 and the tauri#11609 upstream fix

Session context from 2026-07-07. Read together with
`docs/research/spikes.md` (results log is filled in and committed).

## Where things stand

- **S1 + S2 executed on emulator** (API 34 + 35), results committed:
  `c60e19a` (spike results) and `cdd7eda` (Q2 root cause + validated fix)
  on branch `claude/android-foreground-services-8xu1c7`.
- **Q2 root cause is fully understood and a fix is validated end-to-end.**
  Tobi chose **option 2**: upstream the fix into Tauri itself so Android
  apps get webview re-creation for free.
- Next concrete step: post the issue comment (draft below in
  `tauri-11609-comment.md`) to get tauri#11609 re-opened and maintainer
  buy-in on default-on vs config flag. Then implement on a fork of
  tauri `dev`.

## The root cause chain (Q2, blank webview on relaunch)

Scenario: FGS keeps the process alive, user swipes the app from recents
(activity destroyed), then relaunches from the launcher.

1. **Process death (separate issue, solved):** stock Tauri exits the process
   when the last window closes. tao's Android `EventLoop::run` ends with
   `std::process::exit(exit_code)` (tao 0.35.3,
   `src/platform_impl/android/mod.rs:215`). Fixed at app level with
   `RunEvent::ExitRequested` + `api.prevent_exit()`.
2. **Orphaned relaunch activity:** tao maps one Android Activity per tao
   `Window` (`WindowId == ActivityId`). `tao::Window::new` claims the "next
   available activity" (`ndk_glue::next_available_activity()`). wry can
   replay stored webview attributes when an activity with a known id is
   recreated (wry 0.55.1 `src/android/mod.rs`, `android_setup`, line ~154),
   but the id comes from `savedInstanceState ?: intent extras ?: hashCode()`
   (generated `WryActivity.kt`). A launcher relaunch has neither saved state
   nor the extra, gets a fresh `hashCode()`, nothing matches, no webview is
   built.
3. **The dropped signal:** `tauri-runtime-wry` (2.11.4, `src/lib.rs` ~4402,
   same on `dev` at ~4307) dispatches mobile `Event::Resumed`/`Suspended`
   only per existing window. With zero windows the loop body never runs and
   the app never learns about the relaunch. App-level `RunEvent::Resumed`
   only fires from `NewEvents(StartCause::Poll)`, which is unrelated.

## The validated fix

Two pieces, both proven on the API 34 emulator (3 destroy/relaunch cycles
in one process, zero gaps in a 1 Hz FGS Channel stream, both IPC directions
verified in the recreated webview):

**Piece 1, tauri-runtime-wry** (in the mobile Resumed/Suspended branch):

```rust
#[cfg(mobile)]
e @ Event::Resumed | e @ Event::Suspended => {
  if matches!(e, Event::Resumed) {
    callback(RunEvent::Resumed);          // <- added, 3 lines
  }
  let event = match e { ... };            // existing per-window dispatch
```

**Piece 2, app level (spike) / tauri core (option 2 target):**

```rust
tauri::RunEvent::Resumed => {
    use tauri::Manager;
    if app.webview_windows().is_empty() {
        tauri::WebviewWindowBuilder::new(app, "main", tauri::WebviewUrl::default())
            .build()
            .unwrap();
    }
}
```

For option 2, piece 2 moves into `crates/tauri/src/app.rs`
`on_event_loop_event`, `RuntimeRunEvent::Resumed` arm, gated
`#[cfg(target_os = "android")]`: if zero webviews, re-run the same loop
`setup()` uses at `app.rs:2525`:
`for w in config.app.windows.iter().filter(|w| w.create) {
WebviewWindowBuilder::from_config(handle, w)?.build()?; }`.
Using `from_config` also fixes the known caveat that my bare builder ignored
window config (recreated webview drew under the status bar). Frontend state
starts fresh, re-sync from core state is the app's job (matches our
architecture).

## Upstream facts

- **tauri#11609 is CLOSED** (2025-07-28, FabianLars: "let's close it for
  now then and re-open if needed"). No fix ever landed. The `dev` branch
  still has the identical event-dropping code (verified 2026-07-07).
- Old reports mention `__TAURI_INVOKE_KEY__` errors, our repro shows JS
  never executes at all. Same underlying cause, different surface symptom.
- PR target: tauri monorepo `dev` branch, crates `tauri-runtime-wry` and
  `tauri`, with `.changes/` covector entries.

## Plan (agreed with Tobi, in order)

1. Post issue comment, get re-open + direction (default-on vs config flag).
   Draft ready in `tauri-11609-comment.md`, including a standalone minimal
   reproduction (vanilla template + ~30-line keep-alive FGS + prevent_exit,
   no plugin machinery). Every step of that repro was executed and verified
   on the API 34 emulator on 2026-07-07: bug (blank webview on relaunch,
   template UI fine on fresh launch) and fix (relaunch renders the full UI,
   `greet` invoke round-trip works in the recreated webview). Posting is
   Tobi's action.
2. Implement both pieces on a fork branched from `dev`.
3. Validate: apply the same diffs to vendored release crates via
   `[patch.crates-io]` in the spike app (avoids dev-workspace version
   dance), rerun repro + regression (first launch, plain home/resume,
   3x destroy/relaunch). Compile-check the fork against dev workspace.
4. Open PR with root cause, repro, validation evidence.
5. Independently: ship updraft interim with the 3-line runtime-wry patch +
   app-level recreate handler, drop when upstream releases.

## Reproduction (standalone, survives reboots)

The complete spike app (every file verbatim, scaffold commands, the
tauri-runtime-wry patch, build steps, and all repro procedures for the
11609 bug/fix plus the S1/S2 matrices) is documented in
**`docs/research/spike-fgs-repro.md`**. Nothing outside the repo is needed
to rebuild it.

Convenience state that may still exist on this machine (all recreatable
from the repro doc): AVDs `spike-api34` / `spike-api35` (pixel_7,
google_apis arm64, images API 34 `UE1A.230829.050` / API 35
`AE3A.240806.043`, note `spike-api35` has `always_finish_activities=1`
set), and the built spike app in the Claude Code session scratchpad under
`/private/tmp/claude-501/`. Build env:
`ANDROID_HOME=~/Library/Android/sdk`,
`NDK_HOME=$ANDROID_HOME/ndk/28.1.13356709`,
`JAVA_HOME="/Applications/Android Studio.app/Contents/jbr/Contents/Home"`.
