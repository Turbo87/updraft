package aero.updraft.mobile

import app.tauri.plugin.Channel
import com.fasterxml.jackson.databind.ObjectMapper
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertSame
import org.junit.Assert.assertTrue
import org.junit.Test

class SppAttemptOwnerTest {
    @Test
    fun `first request reserves and second pending request is rejected`() {
        val owner = SppAttemptOwner()

        assertTrue(owner.reserve(request("first")))
        assertFalse(owner.reserve(request("second")))
    }

    @Test
    fun `activation atomically moves the pending request to the active attempt`() {
        val owner = SppAttemptOwner()
        val request = request("first")
        val attempt = FakeAttempt()
        owner.reserve(request)

        val activated = owner.activate { attempt }

        assertSame(request, activated?.first)
        assertSame(attempt, activated?.second)
        assertFalse(owner.reserve(request("second")))
    }

    @Test
    fun `request is rejected while an attempt is active`() {
        val owner = SppAttemptOwner()
        owner.reserve(request("first"))
        owner.activate { FakeAttempt() }

        assertFalse(owner.reserve(request("second")))
    }

    @Test
    fun `cancel stops the active attempt without clearing it`() {
        val owner = SppAttemptOwner()
        val attempt = FakeAttempt()
        owner.reserve(request("first"))
        owner.activate { attempt }

        assertNull(owner.cancel())

        assertTrue(attempt.stopped)
        assertFalse(owner.reserve(request("second")))
    }

    @Test
    fun `clear permits reservation only for the active attempt instance`() {
        val owner = SppAttemptOwner()
        val active = FakeAttempt()
        owner.reserve(request("first"))
        owner.activate { active }

        owner.clear(FakeAttempt())
        assertFalse(owner.reserve(request("second")))

        owner.clear(active)
        assertTrue(owner.reserve(request("second")))
    }

    @Test
    fun `abandon releases only the matching pending request`() {
        val owner = SppAttemptOwner()
        val pending = request("first")
        owner.reserve(pending)

        owner.abandon(request("other"))
        assertFalse(owner.reserve(request("second")))

        owner.abandon(pending)
        assertTrue(owner.reserve(request("second")))
    }

    private fun request(address: String): SppRequest =
        SppRequest(address, channel(), {})

    private fun channel(): Channel {
        val mapper = ObjectMapper()
        return Channel(1, {}, mapper)
    }

    private class FakeAttempt : SppAttempt {
        var stopped = false

        override fun run() {}

        override fun stop(): Exception? {
            stopped = true
            return null
        }
    }
}
