package aero.updraft.mobile

import android.annotation.SuppressLint
import android.bluetooth.BluetoothAdapter
import android.bluetooth.BluetoothManager
import android.content.Context
import android.os.Build
import app.tauri.plugin.JSObject
import org.json.JSONArray
import org.json.JSONObject

internal data class BondedBluetoothDevice(val address: String, val name: String?)

internal sealed class BondedBluetoothDevices {
    object Unsupported : BondedBluetoothDevices()
    object PermissionDenied : BondedBluetoothDevices()
    object Disabled : BondedBluetoothDevices()
    data class Available(val devices: List<BondedBluetoothDevice>) : BondedBluetoothDevices()
}

internal interface BondedBluetoothAdapter {
    val enabled: Boolean
    fun bondedDevices(): Set<BondedBluetoothDevice>
}

internal fun bondedBluetoothDevices(
    sdkInt: Int,
    nearbyDevicesGranted: Boolean,
    adapter: BondedBluetoothAdapter?
): BondedBluetoothDevices {
    if (sdkInt >= Build.VERSION_CODES.S && !nearbyDevicesGranted) {
        return BondedBluetoothDevices.PermissionDenied
    }
    adapter ?: return BondedBluetoothDevices.Unsupported

    return try {
        if (!adapter.enabled) {
            BondedBluetoothDevices.Disabled
        } else {
            BondedBluetoothDevices.Available(
                adapter.bondedDevices().sortedBy(BondedBluetoothDevice::address)
            )
        }
    } catch (_: SecurityException) {
        BondedBluetoothDevices.PermissionDenied
    }
}

internal fun queryBondedBluetoothDevices(
    context: Context,
    sdkInt: Int,
    nearbyDevicesGranted: Boolean
): BondedBluetoothDevices {
    val manager = context.getSystemService(BluetoothManager::class.java)
    val adapter = manager?.adapter?.let(::AndroidBondedBluetoothAdapter)
    return bondedBluetoothDevices(sdkInt, nearbyDevicesGranted, adapter)
}

internal fun BondedBluetoothDevices.toJsObject(): JSObject = when (this) {
    BondedBluetoothDevices.Unsupported -> JSObject().put("status", "unsupported")
    BondedBluetoothDevices.PermissionDenied -> JSObject().put("status", "permissionDenied")
    BondedBluetoothDevices.Disabled -> JSObject().put("status", "disabled")
    is BondedBluetoothDevices.Available -> {
        val bondedDevices = JSONArray()
        devices.forEach { bondedDevices.put(it.toJsObject()) }
        JSObject()
            .put("status", "available")
            .put("devices", bondedDevices)
    }
}

private fun BondedBluetoothDevice.toJsObject(): JSObject = JSObject()
    .put("address", address)
    .put("name", name ?: JSONObject.NULL)

@SuppressLint("MissingPermission")
private class AndroidBondedBluetoothAdapter(
    private val adapter: BluetoothAdapter
) : BondedBluetoothAdapter {
    override val enabled: Boolean
        get() = adapter.isEnabled

    override fun bondedDevices(): Set<BondedBluetoothDevice> = adapter.bondedDevices
        .mapTo(mutableSetOf()) { BondedBluetoothDevice(it.address, it.name) }
}
