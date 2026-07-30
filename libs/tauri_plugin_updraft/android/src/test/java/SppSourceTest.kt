package aero.updraft.mobile

import app.tauri.plugin.Channel
import com.fasterxml.jackson.databind.JsonNode
import com.fasterxml.jackson.databind.ObjectMapper
import java.io.ByteArrayInputStream
import java.io.IOException
import java.io.InputStream
import java.util.Base64
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class SppSourceTest {
    @Test
    fun `EOF emits connected and bytes in order followed by one terminal event`() {
        val events = EventCollector()
        val source = source(events) { _, connected, bytes ->
            SppReader(
                FakeSocket(ChunkedInputStream(byteArrayOf(1, 2), byteArrayOf(3))),
                connected,
                bytes
            )
        }

        source.run()

        assertEquals(listOf("connected", "bytes", "bytes", "disconnected"), events.types())
        assertEquals(
            listOf(
                Base64.getEncoder().encodeToString(byteArrayOf(1, 2)),
                Base64.getEncoder().encodeToString(byteArrayOf(3))
            ),
            events.values("data")
        )
        assertTrue(events.last()["error"].isNull)
    }

    @Test
    fun `connect failure emits one terminal event carrying the error`() {
        val events = EventCollector()
        val source = source(events) { _, connected, bytes ->
            SppReader(
                FakeSocket(
                    ByteArrayInputStream(byteArrayOf()),
                    connectFailure = IOException("connect failed")
                ),
                connected,
                bytes
            )
        }

        source.run()

        assertEquals(listOf("disconnected"), events.types())
        assertTrue(events.last()["error"].asText().contains("connect failed"))
    }

    @Test
    fun `read failure emits one terminal event carrying the error`() {
        val events = EventCollector()
        val source = source(events) { _, connected, bytes ->
            SppReader(FakeSocket(FailingInputStream()), connected, bytes)
        }

        source.run()

        assertEquals(listOf("connected", "disconnected"), events.types())
        assertTrue(events.last()["error"].asText().contains("read failed"))
    }

    @Test
    fun `stop before reader assignment closes the reader when assigned`() {
        val events = EventCollector()
        lateinit var socket: FakeSocket
        lateinit var source: SppSource
        source = source(events) { _, connected, bytes ->
            source.stop()
            socket = FakeSocket(ByteArrayInputStream(byteArrayOf()))
            SppReader(socket, connected, bytes)
        }

        source.run()

        assertTrue(socket.closed)
        assertFalse(socket.connected)
        assertEquals(listOf("disconnected"), events.types())
    }

    @Test
    fun `cancellation emits no event after terminal disconnected`() {
        val events = EventCollector()
        lateinit var source: SppSource
        lateinit var connected: () -> Unit
        lateinit var bytes: (ByteArray) -> Unit
        source = source(events) { _, reportConnected, reportBytes ->
            connected = reportConnected
            bytes = reportBytes
            SppReader(
                FakeSocket(
                    object : InputStream() {
                        override fun read(): Int {
                            source.stop()
                            throw IOException("socket closed")
                        }
                    }
                ),
                reportConnected,
                reportBytes
            )
        }

        source.run()
        val terminalEvents = events.size
        connected()
        bytes(byteArrayOf(9))

        assertEquals(listOf("connected", "disconnected"), events.types())
        assertEquals(terminalEvents, events.size)
    }

    @Test
    fun `custom service UUID reaches the reader factory`() {
        val events = EventCollector()
        val customUuid = "e56617bf-f548-4f7c-9cef-4a26eec19b04"
        var receivedUuid: String? = null
        val source = source(events, customUuid) { serviceUuid, connected, bytes ->
            receivedUuid = serviceUuid
            SppReader(
                FakeSocket(ByteArrayInputStream(byteArrayOf())),
                connected,
                bytes
            )
        }

        source.run()

        assertEquals(customUuid, receivedUuid)
        assertEquals(listOf("connected", "disconnected"), events.types())
    }

    @Test
    fun `reader creation failure emits one terminal event`() {
        val events = EventCollector()
        val source = source(events) { _, _, _ ->
            throw IllegalArgumentException("invalid service UUID")
        }

        source.run()

        assertEquals(listOf("disconnected"), events.types())
        assertTrue(events.last()["error"].asText().contains("invalid service UUID"))
    }

    private fun source(
        events: EventCollector,
        serviceUuid: String = "00001101-0000-1000-8000-00805F9B34FB",
        readerFactory: (
            serviceUuid: String,
            onConnected: () -> Unit,
            onBytes: (ByteArray) -> Unit
        ) -> SppReader
    ): SppSource = SppSource(
        serviceUuid,
        events.channel,
        readerFactory,
        encoder = Base64.getEncoder()::encodeToString
    )

    private class EventCollector {
        private val mapper = ObjectMapper()
        private val events = mutableListOf<JsonNode>()
        val channel = Channel(1, { events.add(mapper.readTree(it)) }, mapper)
        val size: Int
            get() = events.size

        fun types(): List<String> = values("type")

        fun values(field: String): List<String> =
            events.mapNotNull { event -> event[field]?.takeUnless(JsonNode::isNull)?.asText() }

        fun last(): JsonNode = events.last()
    }

    private class FakeSocket(
        override val input: InputStream,
        private val connectFailure: Exception? = null
    ) : SppSocket {
        var connected = false
        var closed = false

        override fun connect() {
            connectFailure?.let { throw it }
            connected = true
        }

        override fun close() {
            closed = true
        }
    }

    private class ChunkedInputStream(vararg chunks: ByteArray) : InputStream() {
        private val chunks = ArrayDeque(chunks.toList())

        override fun read(): Int = error("SppReader must use bulk reads")

        override fun read(target: ByteArray, offset: Int, length: Int): Int {
            val chunk = chunks.removeFirstOrNull() ?: return -1
            chunk.copyInto(target, offset)
            return chunk.size
        }
    }

    private class FailingInputStream : InputStream() {
        override fun read(): Int = throw IOException("read failed")
    }
}
