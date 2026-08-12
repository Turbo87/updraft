package aero.updraft.mobile

import android.Manifest
import android.app.Activity
import android.app.ActivityManager
import android.app.Application
import android.os.Build
import android.os.Bundle
import app.tauri.Logger
import app.tauri.PermissionState
import app.tauri.annotation.Command
import app.tauri.annotation.InvokeArg
import app.tauri.annotation.Permission
import app.tauri.annotation.PermissionCallback
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Channel
import app.tauri.plugin.Invoke
import app.tauri.plugin.Plugin
import kotlin.system.exitProcess

private const val LOCATION_ALIAS = "location"
private const val NEARBY_DEVICES_ALIAS = "nearbyDevices"
private const val NOTIFICATIONS_ALIAS = "notifications"

internal data class SourcePermissions(val location: Boolean, val spp: Boolean)

internal enum class StartupStage {
    Location,
    NearbyDevices,
    Notifications,
    Finalize
}

internal sealed class StartupAction {
    object RequestLocation : StartupAction()
    object RequestNearbyDevices : StartupAction()
    object RequestNotifications : StartupAction()
    data class StartService(val sources: SourcePermissions) : StartupAction()
    object Reject : StartupAction()
}

internal fun sourcePermissions(
    locationGranted: Boolean,
    nearbyDevicesGranted: Boolean,
    sdkInt: Int
): SourcePermissions = SourcePermissions(
    location = locationGranted,
    spp = sdkInt < Build.VERSION_CODES.S || nearbyDevicesGranted
)

internal fun startupAction(
    stage: StartupStage,
    sources: SourcePermissions,
    notificationsGranted: Boolean,
    sdkInt: Int
): StartupAction = when (stage) {
    StartupStage.Location -> {
        if (!sources.location) {
            StartupAction.RequestLocation
        } else {
            startupAction(StartupStage.NearbyDevices, sources, notificationsGranted, sdkInt)
        }
    }
    StartupStage.NearbyDevices -> {
        if (!sources.spp) {
            StartupAction.RequestNearbyDevices
        } else {
            startupAction(StartupStage.Notifications, sources, notificationsGranted, sdkInt)
        }
    }
    StartupStage.Notifications -> {
        if (sdkInt >= Build.VERSION_CODES.TIRAMISU && !notificationsGranted) {
            StartupAction.RequestNotifications
        } else {
            startupAction(StartupStage.Finalize, sources, notificationsGranted, sdkInt)
        }
    }
    StartupStage.Finalize -> {
        if (sources.location || sources.spp) {
            StartupAction.StartService(sources)
        } else {
            StartupAction.Reject
        }
    }
}

@InvokeArg
class StartSessionArgs {
    lateinit var fixes: Channel
}

@InvokeArg
class StartSppAttemptArgs {
    lateinit var address: String
    lateinit var serviceUuid: String
    lateinit var events: Channel
}

@InvokeArg
class CancelSppAttemptArgs {
    var connectionId: Long? = null
}

@InvokeArg
class WatchActivitiesArgs {
    lateinit var activities: Channel
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
        Permission(strings = [Manifest.permission.BLUETOOTH_CONNECT], alias = NEARBY_DEVICES_ALIAS),
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

    /** Returns the current bonded Bluetooth devices without starting discovery. */
    @Command
    fun bondedBluetoothDevices(invoke: Invoke) {
        val result = queryBondedBluetoothDevices(
            application,
            Build.VERSION.SDK_INT,
            getPermissionState(NEARBY_DEVICES_ALIAS) == PermissionState.GRANTED
        )
        invoke.resolve(result.toJsObject())
    }

    /**
     * Starts foreground support for every source the pilot permits.
     *
     * Resolves once the service is in the foreground and its requested
     * Location source has initialized. SPP attempts start separately.
     */
    @Command
    fun startSession(invoke: Invoke) {
        advanceStartup(invoke, StartupStage.Location)
    }

    @Command
    fun stopSession(invoke: Invoke) {
        SessionService.stop(application)
        invoke.resolve()
    }

    /**
     * Stops the session and ends the process.
     *
     * Resolves before the exit, so the caller learns that the command arrived
     * rather than losing its response to the process ending.
     */
    @Command
    fun quit(invoke: Invoke) {
        invoke.resolve()
        quitApp()
    }

    /**
     * Stops the session and ends the process that carries it.
     *
     * The platform performs the exit because Rust cannot wake its event loop after
     * Android removes the window. The service must stop first so Android records
     * the stop before the process exits and does not restart the sticky service.
     */
    private fun quitApp(): Nothing {
        Logger.info(TAG, "Stopping the session and ending the process")
        SessionService.stop(application)
        finishAppTasks()
        exitProcess(0)
    }

    /** Removes the app through `ActivityManager` because the plugin activity can be stale. */
    private fun finishAppTasks() {
        val manager = application.getSystemService(ActivityManager::class.java)
        manager.appTasks.forEach { it.finishAndRemoveTask() }
    }

    @Command
    fun startSppAttempt(invoke: Invoke) {
        if (
            Build.VERSION.SDK_INT >= Build.VERSION_CODES.S &&
            getPermissionState(NEARBY_DEVICES_ALIAS) != PermissionState.GRANTED
        ) {
            invoke.reject("Nearby Devices permission is not granted", "permissionDenied")
            return
        }

        val args = invoke.parseArgs(StartSppAttemptArgs::class.java)
        SessionService.startSppAttempt(
            application,
            SppRequest(args.address, args.serviceUuid, args.events) { failure ->
                if (failure == null) {
                    invoke.resolve()
                } else {
                    invoke.reject(failure.toString(), "sppStartFailed")
                }
            }
        )
    }

    @Command
    fun cancelSppAttempt(invoke: Invoke) {
        val connectionId = invoke.parseArgs(CancelSppAttemptArgs::class.java).connectionId
        if (connectionId == null) {
            invoke.reject("SPP connection ID is missing", "invalidArgs")
            return
        }
        val failure = SessionService.cancelSppAttempt(connectionId)
        if (failure == null) {
            invoke.resolve()
        } else {
            invoke.reject(failure.toString(), "sppCancelFailed")
        }
    }

    /**
     * Reports every activity lifecycle transition on the caller's channel,
     * naming the stage and the activity it belongs to.
     *
     * The callbacks are registered on the [Application], which outlives every
     * activity, so a relaunch after the pilot swiped the app away still
     * reaches the caller.
     *
     * The stage names are a wire contract with `tauri/src/activity.rs`, which
     * matches on all six. Renaming one here without renaming it there costs
     * either the window after a relaunch or the guard that keeps a rebuild from
     * aborting the process. The stage that no longer matches falls to that
     * file's catch-all, which warns on every transition.
     */
    @Command
    fun watchActivities(invoke: Invoke) {
        val activities = invoke.parseArgs(WatchActivitiesArgs::class.java).activities
        application.registerActivityLifecycleCallbacks(
            object : Application.ActivityLifecycleCallbacks {
                override fun onActivityCreated(activity: Activity, state: Bundle?) =
                    report("created", activity)

                override fun onActivityStarted(activity: Activity) = report("started", activity)

                override fun onActivityResumed(activity: Activity) = report("resumed", activity)

                override fun onActivityPaused(activity: Activity) = report("paused", activity)

                override fun onActivityStopped(activity: Activity) = report("stopped", activity)

                override fun onActivitySaveInstanceState(activity: Activity, state: Bundle) {}

                override fun onActivityDestroyed(activity: Activity) = report("destroyed", activity)

                /**
                 * Sends on the caller's thread, and must keep doing so.
                 *
                 * `sendObject` runs the whole way into Rust inline, so a
                 * `destroyed` is recorded before this callback returns and
                 * therefore before this thread can enter the next activity's
                 * `onCreate`. `tauri/src/activity.rs` builds a window from
                 * another thread and relies on exactly that ordering to know
                 * it still has an activity to build for. Posting this to a
                 * `Handler` or launching it in a coroutine compiles, passes
                 * every test, looks normal at runtime, and aborts the process
                 * the next time a rebuild races an activity being recreated.
                 */
                private fun report(stage: String, activity: Activity) {
                    Logger.info(TAG, "Activity ${activity.javaClass.name}#${activity.hashCode()} $stage")
                    activities.sendObject(stage)
                }
            }
        )
        invoke.resolve()
    }

    /**
     * Continues to the independent Nearby Devices request after the location
     * prompt, whether or not Android granted precise location.
     */
    @PermissionCallback
    fun locationPermissionResult(invoke: Invoke) {
        advanceStartup(invoke, StartupStage.NearbyDevices)
    }

    /**
     * Continues to the optional notification request after the Nearby Devices
     * prompt, whether or not Android granted Bluetooth access.
     */
    @PermissionCallback
    fun nearbyDevicesPermissionResult(invoke: Invoke) {
        advanceStartup(invoke, StartupStage.Notifications)
    }

    /**
     * Starts the session whatever the pilot answered.
     *
     * Refusing the notification only costs its visibility. A session with an
     * available source still runs, so treating that refusal like either source
     * permission would trade a working session for a visible one.
     */
    @PermissionCallback
    fun notificationPermissionResult(invoke: Invoke) {
        advanceStartup(invoke, StartupStage.Finalize)
    }

    private fun advanceStartup(invoke: Invoke, stage: StartupStage) {
        when (
            val action = startupAction(
                stage,
                currentSourcePermissions(),
                notificationsGranted(),
                Build.VERSION.SDK_INT
            )
        ) {
            StartupAction.RequestLocation ->
                requestPermissionForAlias(LOCATION_ALIAS, invoke, "locationPermissionResult")
            StartupAction.RequestNearbyDevices ->
                requestPermissionForAlias(
                    NEARBY_DEVICES_ALIAS,
                    invoke,
                    "nearbyDevicesPermissionResult"
                )
            StartupAction.RequestNotifications ->
                requestPermissionForAlias(
                    NOTIFICATIONS_ALIAS,
                    invoke,
                    "notificationPermissionResult"
                )
            is StartupAction.StartService -> startService(invoke, action.sources)
            StartupAction.Reject ->
                invoke.reject("location and Nearby Devices permissions are not granted", "permissionDenied")
        }
    }

    private fun startService(invoke: Invoke, sources: SourcePermissions) {
        val fixes = invoke.parseArgs(StartSessionArgs::class.java).fixes
        SessionService.start(application, fixes, sources.location, sources.spp) { failure ->
            if (failure == null) {
                invoke.resolve()
            } else {
                invoke.reject(failure.toString(), "serviceStartFailed")
            }
        }
    }

    private fun currentSourcePermissions(): SourcePermissions = sourcePermissions(
        locationGranted = getPermissionState(LOCATION_ALIAS) == PermissionState.GRANTED,
        nearbyDevicesGranted = getPermissionState(NEARBY_DEVICES_ALIAS) == PermissionState.GRANTED,
        sdkInt = Build.VERSION.SDK_INT
    )

    private fun notificationsGranted(): Boolean =
        getPermissionState(NOTIFICATIONS_ALIAS) == PermissionState.GRANTED

    companion object {
        private val TAG = Logger.tags("UpdraftMobilePlugin")
    }
}
