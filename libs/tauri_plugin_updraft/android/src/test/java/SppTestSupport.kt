package aero.updraft.mobile

import java.io.InputStream

internal class FakeSppSocket(
    override val input: InputStream,
    private val connectFailure: Exception? = null,
    private val closeFailure: Exception? = null
) : SppSocket {
    var connected = false
    var closed = false

    override fun connect() {
        connectFailure?.let { throw it }
        connected = true
    }

    override fun close() {
        closed = true
        closeFailure?.let { throw it }
    }
}

internal class ChunkedInputStream(vararg chunks: ByteArray) : InputStream() {
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
