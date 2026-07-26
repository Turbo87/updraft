package aero.updraft.mobile

import android.Manifest
import android.app.Activity
import android.os.Build
import app.tauri.PermissionState
import app.tauri.annotation.Command
import app.tauri.annotation.InvokeArg
import app.tauri.annotation.Permission
import app.tauri.annotation.PermissionCallback
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Channel
import app.tauri.plugin.Invoke
import app.tauri.plugin.Plugin

private const val LOCATION_ALIAS = "location"
private const val NOTIFICATIONS_ALIAS = "notifications"

@InvokeArg
class StartSessionArgs {
    lateinit var fixes: Channel
}

@TauriPlugin(
    permissions = [
        // Both, because Android 12 ignores a request for the fine permission
        // on its own. The alias is granted only when the pilot answers the one
        // dialog with "precise": an approximate position cannot fly a glider.
        Permission(
            strings = [
                Manifest.permission.ACCESS_FINE_LOCATION,
                Manifest.permission.ACCESS_COARSE_LOCATION
            ],
            alias = LOCATION_ALIAS
        ),
        Permission(strings = [Manifest.permission.POST_NOTIFICATIONS], alias = NOTIFICATIONS_ALIAS)
    ]
)
class UpdraftMobilePlugin(activity: Activity) : Plugin(activity) {
    /**
     * The context a session is started and stopped through.
     *
     * Deliberately not the activity the plugin was built from. That activity is
     * destroyed when the pilot swipes the app away, while the plugin instance
     * survives: tauri's `PluginManager.onActivityCreate` returns early once its
     * activity slot is set, so it is never re-pointed at the replacement. A
     * session outlives any one activity, so it has to be reached through the
     * one context that does too.
     */
    private val application = activity.application

    /**
     * Starts a foreground session reporting the receiver's fixes on the
     * caller's channel, collecting the permissions it needs on the way.
     *
     * Resolves only once the service is in the foreground and subscribed to
     * the receiver, so a refusal on either count reaches the caller instead of
     * leaving behind a session that looks started but can never report a fix.
     */
    @Command
    fun startSession(invoke: Invoke) {
        if (getPermissionState(LOCATION_ALIAS) == PermissionState.GRANTED) {
            promptForNotifications(invoke)
        } else {
            requestPermissionForAlias(LOCATION_ALIAS, invoke, "locationPermissionResult")
        }
    }

    @Command
    fun stopSession(invoke: Invoke) {
        SessionService.stop(application)
        invoke.resolve()
    }

    /**
     * Location is the one permission a session cannot do without: refused, there
     * are no fixes and the session would navigate on nothing.
     */
    @PermissionCallback
    fun locationPermissionResult(invoke: Invoke) {
        val state = getPermissionState(LOCATION_ALIAS)
        if (state == PermissionState.GRANTED) {
            promptForNotifications(invoke)
        } else {
            invoke.reject("location permission $state", "permissionDenied")
        }
    }

    /**
     * Starts the session whatever the pilot answered.
     *
     * Refusing the notification only costs the ongoing notification, and a
     * session running unseen still navigates. Treating this like the location
     * refusal would trade a working session for a visible one.
     */
    @PermissionCallback
    fun notificationPermissionResult(invoke: Invoke) {
        startService(invoke)
    }

    /**
     * Asks for the permission the foreground service notification needs to
     * appear at all, which Android 13 turned into a runtime permission that
     * starts out denied.
     */
    private fun promptForNotifications(invoke: Invoke) {
        val prompt = Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU &&
            getPermissionState(NOTIFICATIONS_ALIAS) != PermissionState.GRANTED
        if (prompt) {
            requestPermissionForAlias(NOTIFICATIONS_ALIAS, invoke, "notificationPermissionResult")
        } else {
            startService(invoke)
        }
    }

    private fun startService(invoke: Invoke) {
        val fixes = invoke.parseArgs(StartSessionArgs::class.java).fixes
        SessionService.start(application, fixes) { failure ->
            if (failure == null) {
                invoke.resolve()
            } else {
                invoke.reject(failure.toString(), "serviceStartFailed")
            }
        }
    }
}
