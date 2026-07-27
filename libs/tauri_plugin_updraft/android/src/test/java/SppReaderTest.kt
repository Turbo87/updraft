package aero.updraft.mobile

import java.io.IOException
import java.io.InputStream
import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotSame
import org.junit.Assert.assertNull
import org.junit.Assert.assertSame
import org.junit.Assert.assertTrue
import org.junit.Test

class SppReaderTest {
    @Test
    fun `connect emits connected, copies bulk reads, reaches EOF, and closes`() {
        val input = ChunkedInputStream(byteArrayOf(1, 2), byteArrayOf(3))
        val socket = FakeSocket(input)
        val events = mutableListOf<String>()
        val chunks = mutableListOf<ByteArray>()
        val reader = SppReader(
            socket,
            onConnected = { events += "connected" },
            onBytes = {
                events += "bytes"
                chunks += it
            }
        )

        val failure = reader.run()

        assertNull(failure)
        assertEquals(listOf("connected", "bytes", "bytes"), events)
        assertArrayEquals(byteArrayOf(1, 2), chunks[0])
        assertArrayEquals(byteArrayOf(3), chunks[1])
        assertEquals(listOf(4096, 4096, 4096), input.readLengths)
        assertTrue(socket.closed)
    }

    @Test
    fun `connect failure is returned and closes`() {
        val failure = IOException("connect failed")
        val socket = FakeSocket(ChunkedInputStream(), connectFailure = failure)
        val reader = SppReader(socket, {}, {})

        assertSame(failure, reader.run())
        assertTrue(socket.closed)
    }

    @Test
    fun `read failure is returned and closes`() {
        val failure = IOException("read failed")
        val socket = FakeSocket(FailingInputStream(failure))
        val reader = SppReader(socket, {}, {})

        assertSame(failure, reader.run())
        assertTrue(socket.closed)
    }

    @Test
    fun `stop closes the socket`() {
        val socket = FakeSocket(ChunkedInputStream())
        val reader = SppReader(socket, {}, {})

        assertNull(reader.stop())
        assertTrue(socket.closed)
    }

    @Test
    fun `separate reads produce separate copied arrays`() {
        val chunks = mutableListOf<ByteArray>()
        val reader = SppReader(
            FakeSocket(ChunkedInputStream(byteArrayOf(7), byteArrayOf(8))),
            {},
            chunks::add
        )

        assertNull(reader.run())
        assertEquals(2, chunks.size)
        assertNotSame(chunks[0], chunks[1])
        assertArrayEquals(byteArrayOf(7), chunks[0])
        assertArrayEquals(byteArrayOf(8), chunks[1])
    }

    @Test
    fun `close failure is returned after EOF`() {
        val failure = IOException("close failed")
        val reader = SppReader(
            FakeSocket(ChunkedInputStream(), closeFailure = failure),
            {},
            {}
        )

        assertSame(failure, reader.run())
    }

    @Test
    fun `connect failure takes precedence over close failure`() {
        val connectFailure = IOException("connect failed")
        val reader = SppReader(
            FakeSocket(
                ChunkedInputStream(),
                connectFailure = connectFailure,
                closeFailure = IOException("close failed")
            ),
            {},
            {}
        )

        assertSame(connectFailure, reader.run())
    }

    @Test
    fun `read failure takes precedence over close failure`() {
        val readFailure = IOException("read failed")
        val reader = SppReader(
            FakeSocket(
                FailingInputStream(readFailure),
                closeFailure = IOException("close failed")
            ),
            {},
            {}
        )

        assertSame(readFailure, reader.run())
    }

    @Test
    fun `stop propagates close failure`() {
        val failure = IOException("close failed")
        val reader = SppReader(
            FakeSocket(ChunkedInputStream(), closeFailure = failure),
            {},
            {}
        )

        assertSame(failure, reader.stop())
    }

    private class FakeSocket(
        override val input: InputStream,
        private val connectFailure: Exception? = null,
        private val closeFailure: Exception? = null
    ) : SppSocket {
        var closed = false

        override fun connect() {
            connectFailure?.let { throw it }
        }

        override fun close() {
            closed = true
            closeFailure?.let { throw it }
        }
    }

    private class ChunkedInputStream(vararg chunks: ByteArray) : InputStream() {
        private val chunks = ArrayDeque(chunks.toList())
        val readLengths = mutableListOf<Int>()

        override fun read(): Int = error("SppReader must use bulk reads")

        override fun read(target: ByteArray, offset: Int, length: Int): Int {
            readLengths += length
            val chunk = chunks.removeFirstOrNull() ?: return -1
            chunk.copyInto(target, offset)
            return chunk.size
        }
    }

    private class FailingInputStream(private val failure: IOException) : InputStream() {
        override fun read(): Int = error("SppReader must use bulk reads")

        override fun read(target: ByteArray, offset: Int, length: Int): Int = throw failure
    }
}
