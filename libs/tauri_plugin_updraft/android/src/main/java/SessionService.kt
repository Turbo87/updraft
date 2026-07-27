package aero.updraft.mobile

import android.annotation.SuppressLint
import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.app.Service
import android.content.Context
import android.content.Intent
import android.content.pm.ServiceInfo
import android.os.Build
import android.os.IBinder
import android.os.PowerManager
import androidx.core.app.NotificationCompat
import androidx.core.content.ContextCompat
import app.tauri.Logger
import app.tauri.plugin.Channel

internal fun foregroundServiceTypes(location: Boolean, spp: Boolean): Int =
    (if (location) ServiceInfo.FOREGROUND_SERVICE_TYPE_LOCATION else 0) or
        (if (spp) ServiceInfo.FOREGROUND_SERVICE_TYPE_CONNECTED_DEVICE else 0)

internal enum class FailedSppServiceStart(val startMode: Int) {
    Keep(Service.START_STICKY),
    Stop(Service.START_NOT_STICKY)
}

internal class ForegroundServiceTypeState {
    var current = 0
        private set
    var isForeground = false
        private set

    fun activate(location: Boolean, spp: Boolean): Int {
        current = current or foregroundServiceTypes(location, spp)
        return current
    }

    fun markForeground() {
        isForeground = true
    }

    fun failedSppStart(): FailedSppServiceStart =
        if (isForeground) FailedSppServiceStart.Keep else FailedSppServiceStart.Stop

    fun reset() {
        current = 0
        isForeground = false
    }
}

/**
 * Keeps the flight computer running while the pilot is not looking at the app.
 *
 * Android freezes a process once its last activity goes away, which stops
 * navigation. Running in the foreground with a partial wake lock keeps the
 * process scheduled and the CPU awake for the duration of a session.
 */
class SessionService : Service() {
    private var wakeLock: PowerManager.WakeLock? = null
    private var gps: GpsSource? = null
    private val foregroundServiceTypeState = ForegroundServiceTypeState()

    override fun onBind(intent: Intent?): IBinder? = null

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        // A null intent means the system restarted us after the process died.
        // Nothing carried over from the session that was running, so staying up
        // would only hold a notification for a session that no longer exists.
        if (intent == null) {
            Logger.info(TAG, "Restarted without a session to resume, stopping")
            stopSelf()
            return START_NOT_STICKY
        }

        if (intent.action == ACTION_SPP_ATTEMPT) {
            return startSppAttempt()
        }
        if (intent.action != ACTION_START) {
            return START_STICKY
        }
        return startSession(intent)
    }

    private fun startSession(intent: Intent): Int {
        val location = intent.getBooleanExtra(EXTRA_LOCATION, false)
        val spp = intent.getBooleanExtra(EXTRA_SPP, false)
        val foregroundServiceTypes = foregroundServiceTypeState.activate(location, spp)

        val failure = doStartForeground(foregroundServiceTypes) ?: startFixes(location)
        if (failure != null) {
            reportStart(failure)
            stopSelf()
            return START_NOT_STICKY
        }

        acquireWakeLock()
        reportStart(null)
        return START_STICKY
    }

    private fun startSppAttempt(): Int {
        val request = sppRequest ?: return finishFailedSppStart()
        sppRequest = null
        val failure = doStartForeground(
            foregroundServiceTypeState.activate(location = false, spp = true)
        )
        if (failure != null) {
            sppAttemptOwner.abandon(request)
            request.onStarted(failure)
            return finishFailedSppStart()
        }

        val (_, attempt) = sppAttemptOwner.activate {
            SppSource(this, it.address, it.events)
        } ?: run {
            request.onStarted(IllegalStateException("SPP attempt reservation was lost"))
            return finishFailedSppStart()
        }
        Thread(
            {
                try {
                    attempt.run()
                } finally {
                    sppAttemptOwner.clear(attempt)
                }
            },
            "updraft-spp"
        ).start()
        acquireWakeLock()
        request.onStarted(null)
        return START_STICKY
    }

    private fun finishFailedSppStart(): Int {
        val failure = foregroundServiceTypeState.failedSppStart()
        if (failure == FailedSppServiceStart.Stop) {
            stopSelf()
        }
        return failure.startMode
    }

    override fun onDestroy() {
        sppRequest?.let { request ->
            sppRequest = null
            sppAttemptOwner.abandon(request)
            request.onStarted(IllegalStateException("session service was destroyed"))
        }
        sppAttemptOwner.cancel()?.let {
            Logger.error(TAG, "Could not close the SPP socket", it)
        }
        gps?.stop()
        gps = null
        fixes = null
        foregroundServiceTypeState.reset()
        releaseWakeLock()
        super.onDestroy()
    }

    /**
     * Points the receiver at the channel the plugin left behind, returning the
     * reason it could not, so a session that can never report a fix fails the
     * call that asked for it rather than looking like a receiver with no
     * signal.
     */
    private fun startFixes(location: Boolean): Exception? {
        if (!location) {
            return null
        }

        gps?.stop()
        gps = null

        val channel = fixes ?: return IllegalStateException("no channel to report fixes on")
        val source = GpsSource(this, channel)
        val failure = source.start()
        if (failure == null) {
            gps = source
        }
        return failure
    }

    /**
     * Promotes the service to the foreground, returning the reason it could not
     * be promoted rather than throwing.
     *
     * A refused `startForeground` leaves the service alive as a plain started
     * service instead of raising the usual ANR, so the failure has to travel
     * back to the caller to be distinguishable from a working session.
     */
    private fun doStartForeground(types: Int): Exception? {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            val manager = getSystemService(Context.NOTIFICATION_SERVICE) as NotificationManager
            manager.createNotificationChannel(
                NotificationChannel(
                    NOTIFICATION_CHANNEL_ID,
                    NOTIFICATION_CHANNEL_NAME,
                    NotificationManager.IMPORTANCE_LOW
                )
            )
        }

        val notification = buildNotification()
        return try {
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
                startForeground(
                    NOTIFICATION_ID,
                    notification,
                    types
                )
            } else {
                startForeground(NOTIFICATION_ID, notification)
            }
            foregroundServiceTypeState.markForeground()
            null
        } catch (e: SecurityException) {
            e
        } catch (e: IllegalStateException) {
            e
        }
    }

    private fun buildNotification(): Notification {
        val launch = packageManager.getLaunchIntentForPackage(packageName)
        val contentIntent = launch?.let {
            PendingIntent.getActivity(
                this,
                0,
                it,
                PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT
            )
        }

        return NotificationCompat.Builder(this, NOTIFICATION_CHANNEL_ID)
            .setContentTitle(NOTIFICATION_TITLE)
            .setContentText(NOTIFICATION_TEXT)
            .setSmallIcon(android.R.drawable.ic_menu_compass)
            .setPriority(NotificationCompat.PRIORITY_LOW)
            .setOngoing(true)
            .setContentIntent(contentIntent)
            .build()
    }

    // A timeout would end up shorter than some flights, and a flight computer
    // that stops navigating part way through one is worse than a battery hit.
    // The bound is the session itself: the lock goes when the service does, and
    // the ongoing notification is how the pilot sees a session still running.
    @SuppressLint("WakelockTimeout")
    private fun acquireWakeLock() {
        if (wakeLock != null) {
            return
        }
        val power = getSystemService(Context.POWER_SERVICE) as PowerManager
        wakeLock = power.newWakeLock(PowerManager.PARTIAL_WAKE_LOCK, WAKE_LOCK_TAG).apply {
            setReferenceCounted(false)
            acquire()
        }
    }

    private fun releaseWakeLock() {
        wakeLock?.takeIf { it.isHeld }?.release()
        wakeLock = null
    }

    private fun reportStart(failure: Exception?) {
        val listener = startListener
        startListener = null
        // Without a listener the failure has nowhere else to go: the service
        // was started by something other than the plugin.
        if (failure != null && listener == null) {
            Logger.error(TAG, "Could not start in the foreground", failure)
        }
        listener?.invoke(failure)
    }

    companion object {
        private val TAG = Logger.tags("SessionService")

        private const val ACTION_START = "aero.updraft.mobile.SESSION_START"
        private const val ACTION_SPP_ATTEMPT = "aero.updraft.mobile.SPP_ATTEMPT"
        private const val EXTRA_LOCATION = "location"
        private const val EXTRA_SPP = "spp"
        private const val NOTIFICATION_ID = 1
        private const val NOTIFICATION_CHANNEL_ID = "session"
        private const val NOTIFICATION_CHANNEL_NAME = "Flight session"
        private const val NOTIFICATION_TITLE = "Updraft"
        private const val NOTIFICATION_TEXT = "Navigating in the background"
        private const val WAKE_LOCK_TAG = "updraft:session"

        /**
         * Reports the outcome of the pending start back to the plugin.
         *
         * `startForegroundService` returns before the service has had a chance
         * to promote itself, so success or failure can only be answered once
         * `onStartCommand` has run.
         */
        @Volatile
        private var startListener: ((Exception?) -> Unit)? = null

        /**
         * The channel the service reports its fixes on.
         *
         * A `Channel` cannot travel in an `Intent`, so the plugin leaves it
         * here for the service to pick up as it starts.
         */
        @Volatile
        private var fixes: Channel? = null

        private val sppAttemptOwner = SppAttemptOwner()

        @Volatile
        private var sppRequest: SppRequest? = null

        /**
         * Starts a session that reports every fix on [fixes], calling
         * [onStarted] with null once the service is in the foreground and its
         * permitted Location source has started, or with the reason it could
         * not get there. The SPP flag reserves foreground support while
         * attempts start separately through [startSppAttempt].
         */
        fun start(
            context: Context,
            fixes: Channel,
            location: Boolean,
            spp: Boolean,
            onStarted: (Exception?) -> Unit
        ) {
            startListener = onStarted
            this.fixes = fixes

            val intent = Intent(context, SessionService::class.java)
                .setAction(ACTION_START)
                .putExtra(EXTRA_LOCATION, location)
                .putExtra(EXTRA_SPP, spp)
            try {
                ContextCompat.startForegroundService(context, intent)
            } catch (e: IllegalStateException) {
                startListener = null
                onStarted(e)
            }
        }

        fun stop(context: Context) {
            context.stopService(Intent(context, SessionService::class.java))
        }

        internal fun startSppAttempt(context: Context, request: SppRequest) {
            if (!sppAttemptOwner.reserve(request)) {
                request.onStarted(IllegalStateException("an SPP attempt is already active"))
                return
            }
            sppRequest = request

            try {
                ContextCompat.startForegroundService(
                    context,
                    Intent(context, SessionService::class.java).setAction(ACTION_SPP_ATTEMPT)
                )
            } catch (e: IllegalStateException) {
                if (sppRequest === request) {
                    sppRequest = null
                }
                sppAttemptOwner.abandon(request)
                request.onStarted(e)
            } catch (e: SecurityException) {
                if (sppRequest === request) {
                    sppRequest = null
                }
                sppAttemptOwner.abandon(request)
                request.onStarted(e)
            }
        }

        internal fun cancelSppAttempt(): Exception? = sppAttemptOwner.cancel()
    }
}
