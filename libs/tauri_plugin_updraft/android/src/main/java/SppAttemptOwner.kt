package aero.updraft.mobile

import app.tauri.plugin.Channel

internal interface SppAttempt {
    fun run()
    fun stop(): Exception?
}

internal data class SppRequest(
    val address: String,
    val events: Channel,
    val onStarted: (Exception?) -> Unit
)

internal class SppAttemptOwner {
    private val lock = Any()
    private var pending: SppRequest? = null
    private var active: SppAttempt? = null

    fun reserve(request: SppRequest): Boolean = synchronized(lock) {
        if (pending != null || active != null) {
            false
        } else {
            pending = request
            true
        }
    }

    fun abandon(request: SppRequest) {
        synchronized(lock) {
            if (pending === request) {
                pending = null
            }
        }
    }

    fun activate(factory: (SppRequest) -> SppAttempt): Pair<SppRequest, SppAttempt>? =
        synchronized(lock) {
            val request = pending ?: return@synchronized null
            val attempt = factory(request)
            pending = null
            active = attempt
            request to attempt
        }

    fun clear(attempt: SppAttempt) {
        synchronized(lock) {
            if (active === attempt) {
                active = null
            }
        }
    }

    fun cancel(): Exception? {
        val attempt = synchronized(lock) { active }
        return attempt?.stop()
    }
}
