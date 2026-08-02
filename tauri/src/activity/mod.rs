//! Keeping a window on the activity the platform gives the process.
//!
//! Android destroys the activity when the pilot swipes the app away and hands
//! the surviving process a fresh one on the next launch. Nothing in the runtime
//! claims that activity, so the app has to notice it and build a window itself.

use std::fmt::Display;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tauri::ipc::{Channel, InvokeResponseBody};
use tauri::{AppHandle, Manager, Runtime};
use tauri_plugin_updraft::UpdraftMobileExt;

/// The transition after which a window may be built for an activity.
///
/// Spelled as `report(...)` spells it in
/// `libs/tauri_plugin_updraft/android/src/main/java/UpdraftMobilePlugin.kt`,
/// which is the other half of this contract. Renaming it there and not here
/// costs a window after every relaunch, which the catch-all arm below warns
/// about on every transition.
///
/// `created` arrives from inside the activity's own `onCreate`, before the
/// runtime has registered the activity a webview attaches to. tao's window
/// reads the context map, which `onCreate` fills first, and succeeds; wry's
/// webview reads the activity proxy, which the same call fills a few JNI hops
/// later, and aborts the process on `no available activity`. `started` is the
/// first transition that cannot run before `onCreate` has returned.
const ATTACHED: &str = "started";

/// The transition after which no window may be built, from the same Kotlin
/// `report(...)` calls as [`ATTACHED`]. Renaming it there and not here leaves
/// the permission below stuck on, which is what aborts the process.
///
/// It closes the `created` race from the other side: the next activity's
/// `onCreate` runs on the Java UI thread while the event loop runs on its own,
/// so an offer still pending from the destroyed activity would otherwise be
/// free to build a window in the middle of it.
///
/// The withdrawal wins that race by ordering, not by margin. Traced against
/// tauri 2.11.5, which the workspace pins exactly; a bump has to re-walk it.
/// `Channel.sendObject` calls its handler inline (tauri's
/// `mobile/android/src/main/java/app/tauri/plugin/Channel.kt`); the handler is
/// `PluginManager.sendChannelData`, an `external fun`; its JNI
/// entry point calls `send_channel_data` (tauri's `src/lib.rs`), which reaches
/// `Channel::send` and from there this module's channel closure on the calling
/// thread (tauri's `src/plugin/mobile.rs`, `src/ipc/channel.rs`). Nothing in that
/// chain posts to a queue, so the flag is cleared inside `onActivityDestroyed`,
/// on the Java UI thread, before that thread can enter any later `onCreate`.
///
/// The ordering is what makes this safe rather than lucky, and the difference is
/// reachable: `AndroidManifest.xml` leaves `density`, `fontScale` and
/// `layoutDirection` out of `configChanges`, so a display- or font-size change
/// still destroys and recreates the activity back to back on that one thread,
/// with no margin at all. That one case survives on a second footing as well:
/// `isChangingConfigurations` is true for it, so tao emits no `Destroyed` event
/// and leaves its `CONTEXTS` entry alone, `webview_windows()` stays non-empty,
/// and [`take_offer`] returns on its first predicate. The ordering is what makes
/// the *general* case safe, and it is the only thing that does.
const DETACHED: &str = "destroyed";

/// How often a pending rebuild is offered to the event loop again.
///
/// tao's Android event loop loses the wake carrying a `run_on_main_thread`
/// closure when it shares a poll with the activity events a relaunch produces,
/// and with no window there is nothing else to wake it. Every offer carries a
/// fresh wake, and the loop drains its whole queue once one lands, so repeating
/// is what turns a dropped wake into a late one.
const OFFER_INTERVAL: Duration = Duration::from_millis(200);

/// How many offers to make before giving up, two seconds' worth at
/// [`OFFER_INTERVAL`].
///
/// Measured relaunches rebuilt on the first or second offer and never needed
/// a third: at most one wake shares a poll with the relaunch's event burst,
/// and the burst is over in milliseconds. Ten offers is an order of magnitude
/// past that. Waiting longer buys nothing a pilot would notice, and it keeps
/// offers pending across a relaunch, where the only thing holding them back
/// from wry's abort is [`DETACHED`]. The measurements are in
/// `docs/superpowers/verification/2026-07-26-android-platform.md`.
///
/// Giving up is not a dead end. The offers already queued in tao's channel stay
/// there, and the loop drains all of them at its next wake from any source, so
/// a rebuild that ran out of patience still lands on its own as soon as the loop
/// wakes for anything else, provided the activity is still attached.
const OFFERS: usize = 10;

/// The window state a rebuild reads and the offer it makes, so the retry policy
/// can be exercised without an event loop.
trait WindowRebuild {
    type Error: Display;

    /// Whether a window exists, so there is nothing left to build.
    fn window_exists(&self) -> bool;

    /// Whether an activity a window may attach to still exists.
    fn activity_exists(&self) -> bool;

    /// Whether an offer that ran found the build itself failing.
    fn build_failed(&self) -> bool;

    /// Builds the windows the app is configured to have. Runs on the event
    /// loop, reached only through [`take_offer`].
    fn build(&self);

    /// Queues [`take_offer`] for the thread that owns the event loop.
    ///
    /// `Ok` only says the offer was queued. Whether it ran shows up in
    /// [`WindowRebuild::window_exists`] and [`WindowRebuild::build_failed`].
    fn offer(&self) -> Result<(), Self::Error>;
}

/// Decides, on the event loop, whether a queued offer may still build.
///
/// This is the guard that closes the abort, and the loop's own checks are no
/// substitute for it: an offer already queued in tao's channel runs whenever
/// the loop next wakes, however long afterwards and whatever the loop decided
/// in the meantime. An offer that outlived its activity would otherwise be free
/// to build a window inside the *next* activity's `onCreate`, where tao's
/// window succeeds and wry's webview aborts the process. See [`DETACHED`].
///
/// The predicates are the loop's, deliberately. An offer queued before a build
/// failed reaches the event loop after the loop has already given up on it, and
/// would otherwise repeat both the failing build and its error.
fn take_offer(target: &impl WindowRebuild) {
    if target.window_exists() || !target.activity_exists() || target.build_failed() {
        return;
    }

    target.build();
}

/// Why a rebuild stopped.
#[derive(Debug, PartialEq, Eq)]
enum Outcome {
    /// A window exists, whether this rebuild built it or found it.
    Window,
    /// The activity the rebuild was for went away.
    ActivityGone,
    /// An offer ran and the build failed, which no further offer changes.
    BuildFailed,
    /// The event loop can no longer be reached.
    LoopClosed,
    /// Every offer was made and no window appeared.
    GaveUp,
}

/// Offers the event loop a rebuilt window until it takes one.
///
/// Offering repeatedly rather than once is what makes a dropped wake recover:
/// an offer costs nothing when a window already exists, and the loop runs every
/// closure it has queued as soon as any wake reaches it.
async fn rebuild(target: impl WindowRebuild) -> Outcome {
    for _ in 0..OFFERS {
        if target.window_exists() {
            return Outcome::Window;
        }
        if !target.activity_exists() {
            return Outcome::ActivityGone;
        }
        if target.build_failed() {
            return Outcome::BuildFailed;
        }
        if let Err(error) = target.offer() {
            tracing::error!(%error, "Could not reach the event loop");
            return Outcome::LoopClosed;
        }

        tokio::time::sleep(OFFER_INTERVAL).await;
    }

    if target.window_exists() {
        Outcome::Window
    } else {
        Outcome::GaveUp
    }
}

/// The app's configured windows, and the activity they attach to.
struct ConfiguredWindows<R: Runtime> {
    app: AppHandle<R>,
    activity_attached: Arc<AtomicBool>,
    build_failed: Arc<AtomicBool>,
    signalled_at: std::time::Instant,
}

impl<R: Runtime> Clone for ConfiguredWindows<R> {
    fn clone(&self) -> Self {
        Self {
            app: self.app.clone(),
            activity_attached: self.activity_attached.clone(),
            build_failed: self.build_failed.clone(),
            signalled_at: self.signalled_at,
        }
    }
}

impl<R: Runtime> WindowRebuild for ConfiguredWindows<R> {
    type Error = tauri::Error;

    fn window_exists(&self) -> bool {
        !self.app.webview_windows().is_empty()
    }

    fn activity_exists(&self) -> bool {
        self.activity_attached.load(Ordering::SeqCst)
    }

    fn build_failed(&self) -> bool {
        self.build_failed.load(Ordering::SeqCst)
    }

    /// From the configuration rather than a bare builder, so a window setting
    /// added later is not silently dropped. Android draws none of them today:
    /// tao's `Window::new` carries an explicit
    /// `FIXME this ignores requested window attributes`.
    fn build(&self) {
        // With more than one configured window, one build failing after another
        // succeeded ends the rebuild as a success and never retries the failed
        // one, because a window then exists. One window is configured today.
        for config in &self.app.config().app.windows {
            if !config.create {
                continue;
            }

            match tauri::WebviewWindowBuilder::from_config(&self.app, config)
                .and_then(|builder| builder.build())
            {
                Ok(_) => tracing::info!(
                    window = %config.label,
                    waited_ms = self.signalled_at.elapsed().as_millis() as u64,
                    "Rebuilt the webview window"
                ),
                Err(error) => {
                    self.build_failed.store(true, Ordering::SeqCst);
                    tracing::error!(window = %config.label, %error, "Could not rebuild the webview window");
                }
            }
        }
    }

    fn offer(&self) -> Result<(), tauri::Error> {
        let target = self.clone();
        self.app.run_on_main_thread(move || take_offer(&target))
    }
}

/// Watches the platform's activity transitions and keeps a window on whichever
/// activity it hands the process.
///
/// Only Android reports transitions. Elsewhere the plugin accepts the channel
/// and never reports on it, so nothing below the channel ever runs.
pub fn watch<R: Runtime>(app: AppHandle<R>) {
    let activity_attached = Arc::new(AtomicBool::new(false));
    let rebuild_for = app.clone();

    let transitions = Channel::new(move |body: InvokeResponseBody| {
        let stage = match body.deserialize::<String>() {
            Ok(stage) => stage,
            Err(error) => {
                tracing::error!(%error, "Discarded an unreadable activity transition");
                return Ok(());
            }
        };
        tracing::debug!(%stage, "Activity transition");

        match stage.as_str() {
            ATTACHED => {
                activity_attached.store(true, Ordering::SeqCst);
                let target = ConfiguredWindows {
                    app: rebuild_for.clone(),
                    activity_attached: activity_attached.clone(),
                    build_failed: Arc::new(AtomicBool::new(false)),
                    signalled_at: std::time::Instant::now(),
                };
                tauri::async_runtime::spawn(async move {
                    if rebuild(target).await == Outcome::GaveUp {
                        tracing::error!("Gave up rebuilding the webview window");
                    }
                });
            }
            DETACHED => activity_attached.store(false, Ordering::SeqCst),
            // The rest of what `UpdraftMobilePlugin.kt` reports. Naming them
            // rather than defaulting is what makes a rename of the two above
            // loud instead of silent.
            "created" | "resumed" | "paused" | "stopped" => {}
            _ => tracing::warn!(%stage, "Unrecognised activity transition"),
        }
        Ok(())
    });

    tauri::async_runtime::spawn_blocking(move || {
        match app.updraft_mobile().watch_activities(transitions) {
            // At the default filter on Android, because this is the only line
            // that says the rebuild is armed, and unarmed means a blank screen
            // after every relaunch. Elsewhere the plugin reports nothing ever,
            // so at `info!` the line would claim something that never happens.
            #[cfg(target_os = "android")]
            Ok(()) => tracing::info!("Watching activity transitions"),
            #[cfg(not(target_os = "android"))]
            Ok(()) => tracing::debug!("Watching activity transitions"),
            Err(error) => tracing::error!(%error, "Failed to watch activity transitions"),
        }
    });
}

#[cfg(test)]
mod tests;
