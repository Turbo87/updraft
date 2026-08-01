package aero.updraft.mobile

import app.tauri.plugin.Channel

internal interface SppAttempt {
    fun run()
    fun stop(): Exception?
}

internal data class SppRequest(
    val address: String,
    val serviceUuid: String,
    val events: Channel,
    val onStarted: (Exception?) -> Unit
) {
    val connectionId: Long
        get() = events.id
}

internal data class ActiveSppAttempt(
    val request: SppRequest,
    val attempt: SppAttempt
)

internal data class DrainedSppAttempts(
    val pending: List<SppRequest>,
    val active: List<ActiveSppAttempt>
)

private sealed interface SppAttemptEntry {
    data class Pending(val request: SppRequest) : SppAttemptEntry
    data class Active(
        val request: SppRequest,
        val attempt: SppAttempt
    ) : SppAttemptEntry
}

internal class SppAttemptRegistry {
    private val lock = Any()
    private val entries = mutableMapOf<Long, SppAttemptEntry>()

    fun reserve(request: SppRequest): Boolean = synchronized(lock) {
        if (entries.containsKey(request.connectionId)) {
            false
        } else {
            entries[request.connectionId] = SppAttemptEntry.Pending(request)
            true
        }
    }

    fun pending(connectionId: Long): SppRequest? = synchronized(lock) {
        (entries[connectionId] as? SppAttemptEntry.Pending)?.request
    }

    fun abandon(request: SppRequest) {
        synchronized(lock) {
            val pending = entries[request.connectionId] as? SppAttemptEntry.Pending
            if (pending?.request === request) {
                entries.remove(request.connectionId)
            }
        }
    }

    fun activate(
        connectionId: Long,
        factory: (SppRequest) -> SppAttempt
    ): Pair<SppRequest, SppAttempt>? = synchronized(lock) {
        val pending = entries[connectionId] as? SppAttemptEntry.Pending
            ?: return@synchronized null
        val attempt = factory(pending.request)
        entries[connectionId] = SppAttemptEntry.Active(pending.request, attempt)
        pending.request to attempt
    }

    fun clear(connectionId: Long, attempt: SppAttempt) {
        synchronized(lock) {
            val active = entries[connectionId] as? SppAttemptEntry.Active
            if (active?.attempt === attempt) {
                entries.remove(connectionId)
            }
        }
    }

    fun cancel(connectionId: Long): Exception? {
        val attempt = synchronized(lock) {
            (entries[connectionId] as? SppAttemptEntry.Active)?.attempt
        }
        return attempt?.stop()
    }

    fun drain(): DrainedSppAttempts = synchronized(lock) {
        val pending = entries.values.mapNotNull {
            (it as? SppAttemptEntry.Pending)?.request
        }
        val active = entries.values.mapNotNull {
            (it as? SppAttemptEntry.Active)?.let { entry ->
                ActiveSppAttempt(entry.request, entry.attempt)
            }
        }
        entries.clear()
        DrainedSppAttempts(pending, active)
    }
}
