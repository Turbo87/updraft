package aero.updraft.mobile

import app.tauri.plugin.Channel
import com.fasterxml.jackson.databind.ObjectMapper
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertSame
import org.junit.Assert.assertTrue
import org.junit.Test

class SppAttemptRegistryTest {
    @Test
    fun `different connection IDs can be active together`() {
        val registry = SppAttemptRegistry()
        val firstRequest = request(11, "00:11:22:33:44:55")
        val secondRequest = request(12, "00:11:22:33:44:55")
        val firstAttempt = FakeAttempt()
        val secondAttempt = FakeAttempt()

        assertTrue(registry.reserve(firstRequest))
        assertTrue(registry.reserve(secondRequest))
        assertSame(firstAttempt, registry.activate(11) { firstAttempt }?.second)
        assertSame(secondAttempt, registry.activate(12) { secondAttempt }?.second)

        assertFalse(registry.reserve(request(11, "00:11:22:33:44:77")))
        assertFalse(registry.reserve(request(12, "00:11:22:33:44:88")))
    }

    @Test
    fun `duplicate connection ID does not change another entry`() {
        val registry = SppAttemptRegistry()
        val first = request(21, "00:11:22:33:44:55")
        val second = request(22, "00:11:22:33:44:66")

        assertTrue(registry.reserve(first))
        assertTrue(registry.reserve(second))
        assertFalse(registry.reserve(request(21, "00:11:22:33:44:77")))
        assertSame(first, registry.pending(21))
        assertSame(second, registry.pending(22))
    }

    @Test
    fun `cancel stops only the selected active attempt`() {
        val registry = SppAttemptRegistry()
        val first = FakeAttempt()
        val second = FakeAttempt()
        registry.reserve(request(31, "00:11:22:33:44:55"))
        registry.reserve(request(32, "00:11:22:33:44:66"))
        registry.activate(31) { first }
        registry.activate(32) { second }

        assertNull(registry.cancel(31))

        assertTrue(first.stopped)
        assertFalse(second.stopped)
        assertNull(registry.cancel(99))
    }

    @Test
    fun `clear removes only the matching attempt instance`() {
        val registry = SppAttemptRegistry()
        val active = FakeAttempt()
        registry.reserve(request(41, "00:11:22:33:44:55"))
        registry.activate(41) { active }

        registry.clear(41, FakeAttempt())
        assertFalse(registry.reserve(request(41, "00:11:22:33:44:66")))

        registry.clear(41, active)
        assertTrue(registry.reserve(request(41, "00:11:22:33:44:66")))
    }

    @Test
    fun `abandon removes only the matching pending request`() {
        val registry = SppAttemptRegistry()
        val first = request(51, "00:11:22:33:44:55")
        val second = request(52, "00:11:22:33:44:66")
        registry.reserve(first)
        registry.reserve(second)

        registry.abandon(request(51, "00:11:22:33:44:77"))
        assertSame(first, registry.pending(51))
        assertSame(second, registry.pending(52))

        registry.abandon(first)
        assertNull(registry.pending(51))
        assertSame(second, registry.pending(52))
    }

    @Test
    fun `drain returns all pending and active entries`() {
        val registry = SppAttemptRegistry()
        val pending = request(61, "00:11:22:33:44:55")
        val activeRequest = request(62, "00:11:22:33:44:66")
        val activeAttempt = FakeAttempt()
        registry.reserve(pending)
        registry.reserve(activeRequest)
        registry.activate(62) { activeAttempt }

        val drained = registry.drain()

        assertEquals(listOf(pending), drained.pending)
        assertEquals(listOf(activeRequest), drained.active.map { it.request })
        assertSame(activeAttempt, drained.active.single().attempt)
        assertTrue(registry.reserve(request(61, "00:11:22:33:44:77")))
        assertTrue(registry.reserve(request(62, "00:11:22:33:44:88")))
    }

    private fun request(connectionId: Long, address: String): SppRequest =
        SppRequest(
            address,
            "00001101-0000-1000-8000-00805F9B34FB",
            channel(connectionId),
            {}
        )

    private fun channel(id: Long): Channel = Channel(id, {}, ObjectMapper())

    private class FakeAttempt : SppAttempt {
        var stopped = false

        override fun run() {}

        override fun stop(): Exception? {
            stopped = true
            return null
        }
    }
}
