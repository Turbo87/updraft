package aero.updraft.mobile

import android.os.Build
import org.junit.Assert.assertEquals
import org.junit.Test

class BondedBluetoothDevicesTest {
    @Test
    fun `reports permission denial on Android S and newer`() {
        assertEquals(
            BondedBluetoothDevices.PermissionDenied,
            bondedBluetoothDevices(
                sdkInt = Build.VERSION_CODES.S,
                nearbyDevicesGranted = false,
                adapter = FakeBondedBluetoothAdapter()
            )
        )
    }

    @Test
    fun `reports unsupported when the adapter is missing`() {
        assertEquals(
            BondedBluetoothDevices.Unsupported,
            bondedBluetoothDevices(
                sdkInt = Build.VERSION_CODES.S - 1,
                nearbyDevicesGranted = false,
                adapter = null
            )
        )
    }

    @Test
    fun `reports disabled before reading bonded devices`() {
        val adapter = FakeBondedBluetoothAdapter(enabled = false)

        assertEquals(
            BondedBluetoothDevices.Disabled,
            bondedBluetoothDevices(
                sdkInt = Build.VERSION_CODES.S,
                nearbyDevicesGranted = true,
                adapter = adapter
            )
        )
        assertEquals(0, adapter.reads)
    }

    @Test
    fun `reports an empty bonded set`() {
        assertEquals(
            BondedBluetoothDevices.Available(emptyList()),
            bondedBluetoothDevices(
                sdkInt = Build.VERSION_CODES.S,
                nearbyDevicesGranted = true,
                adapter = FakeBondedBluetoothAdapter()
            )
        )
    }

    @Test
    fun `returns addresses and optional names in address order`() {
        val devices = setOf(
            BondedBluetoothDevice("AA:BB:CC:DD:EE:FF", null),
            BondedBluetoothDevice("00:11:22:33:44:55", "Flight recorder")
        )

        assertEquals(
            BondedBluetoothDevices.Available(
                listOf(
                    BondedBluetoothDevice("00:11:22:33:44:55", "Flight recorder"),
                    BondedBluetoothDevice("AA:BB:CC:DD:EE:FF", null)
                )
            ),
            bondedBluetoothDevices(
                sdkInt = Build.VERSION_CODES.S - 1,
                nearbyDevicesGranted = false,
                adapter = FakeBondedBluetoothAdapter(devices = devices)
            )
        )
    }

    @Test
    fun `reports permission denial when platform access loses permission`() {
        assertEquals(
            BondedBluetoothDevices.PermissionDenied,
            bondedBluetoothDevices(
                sdkInt = Build.VERSION_CODES.S,
                nearbyDevicesGranted = true,
                adapter = FakeBondedBluetoothAdapter(failure = SecurityException())
            )
        )
    }
}

private class FakeBondedBluetoothAdapter(
    override val enabled: Boolean = true,
    private val devices: Set<BondedBluetoothDevice> = emptySet(),
    private val failure: SecurityException? = null
) : BondedBluetoothAdapter {
    var reads = 0

    override fun bondedDevices(): Set<BondedBluetoothDevice> {
        reads += 1
        failure?.let { throw it }
        return devices
    }
}
