package aero.updraft.mobile

import java.io.InputStream

internal interface SppSocket {
    val input: InputStream
    fun connect()
    fun close()
}

internal class SppReader(
    private val socket: SppSocket,
    private val onConnected: () -> Unit,
    private val onBytes: (ByteArray) -> Unit
) {
    fun run(): Exception? {
        var failure: Exception? = null
        try {
            socket.connect()
            onConnected()

            val buffer = ByteArray(4096)
            while (true) {
                val count = socket.input.read(buffer)
                if (count < 0) {
                    break
                }
                if (count > 0) {
                    onBytes(buffer.copyOf(count))
                }
            }
        } catch (e: Exception) {
            failure = e
        }

        try {
            socket.close()
        } catch (e: Exception) {
            if (failure == null) {
                failure = e
            }
        }
        return failure
    }

    fun stop(): Exception? = try {
        socket.close()
        null
    } catch (e: Exception) {
        e
    }
}
