package aero.updraft.mobile

import android.Manifest
import android.annotation.SuppressLint
import android.bluetooth.BluetoothDevice
import android.bluetooth.BluetoothManager
import android.bluetooth.BluetoothSocket
import android.content.Context
import android.content.pm.PackageManager
import android.os.Build
import android.util.Base64
import androidx.core.content.ContextCompat
import app.tauri.plugin.Channel
import java.util.UUID

internal class SppSource(
    private val events: Channel,
    private val readerFactory: (
        onConnected: () -> Unit,
        onBytes: (ByteArray) -> Unit
    ) -> SppReader,
    private val encoder: (ByteArray) -> String
) : SppAttempt {
    constructor(context: Context, address: String, events: Channel) : this(
        events,
        { onConnected, onBytes ->
            SppReader(createSocket(context, address), onConnected, onBytes)
        },
        { bytes -> Base64.encodeToString(bytes, Base64.NO_WRAP) }
    )

    private val lock = Any()
    private var stopped = false
    private var reader: SppReader? = null
    private var terminal = false

    override fun run() {
        var failure: Exception? = null
        try {
            val assigned = readerFactory(
                { send(mapOf("type" to "connected")) },
                { bytes ->
                    send(
                        mapOf(
                            "type" to "bytes",
                            "data" to encoder(bytes)
                        )
                    )
                }
            )
            val stopOnAssignment = synchronized(lock) {
                reader = assigned
                stopped
            }
            failure = if (stopOnAssignment) assigned.stop() else assigned.run()
        } catch (e: Exception) {
            failure = e
        } finally {
            sendTerminal(failure)
        }
    }

    override fun stop(): Exception? {
        val assigned = synchronized(lock) {
            stopped = true
            reader
        }
        return assigned?.stop()
    }

    private fun send(event: Map<String, String>) {
        synchronized(lock) {
            if (!terminal) {
                events.sendObject(event)
            }
        }
    }

    private fun sendTerminal(failure: Exception?) {
        synchronized(lock) {
            if (!terminal) {
                terminal = true
                events.sendObject(
                    mapOf(
                        "type" to "disconnected",
                        "error" to failure?.toString()
                    )
                )
            }
        }
    }

    companion object {
        private val SPP_UUID = UUID.fromString("00001101-0000-1000-8000-00805F9B34FB")

        @SuppressLint("MissingPermission")
        private fun createSocket(context: Context, address: String): SppSocket {
            if (
                Build.VERSION.SDK_INT >= Build.VERSION_CODES.S &&
                ContextCompat.checkSelfPermission(
                    context,
                    Manifest.permission.BLUETOOTH_CONNECT
                ) != PackageManager.PERMISSION_GRANTED
            ) {
                throw SecurityException("Nearby Devices permission is not granted")
            }

            val manager = context.getSystemService(BluetoothManager::class.java)
                ?: throw IllegalStateException("Bluetooth is unavailable")
            val adapter = manager.adapter
                ?: throw IllegalStateException("Bluetooth is unavailable")
            check(adapter.isEnabled) { "Bluetooth is disabled" }

            val device = adapter.getRemoteDevice(address)
            check(device.bondState == BluetoothDevice.BOND_BONDED) {
                "Bluetooth device $address is not bonded"
            }
            return AndroidSppSocket(device.createRfcommSocketToServiceRecord(SPP_UUID))
        }
    }
}

private class AndroidSppSocket(private val socket: BluetoothSocket) : SppSocket {
    override val input
        get() = socket.inputStream

    override fun connect() {
        socket.connect()
    }

    override fun close() {
        socket.close()
    }
}
