package aero.updraft.mobile

import android.Manifest
import android.content.Context
import android.content.pm.PackageManager
import android.location.Location
import android.location.LocationListener
import android.location.LocationManager
import android.os.Bundle
import android.os.Looper
import androidx.core.content.ContextCompat
import app.tauri.Logger
import app.tauri.plugin.Channel
import app.tauri.plugin.JSObject
import org.json.JSONObject

/**
 * Reports the fixes of the device's own GNSS receiver on a session channel.
 *
 * Reads [LocationManager.GPS_PROVIDER] rather than either fused provider.
 * Fusion is tuned for walking and driving: it smooths, which lags through the
 * sustained turns and vertical rates that thermalling consists of, and it can
 * blend in network-derived positions that are kilometres out with nothing to
 * mark them as such. A cell-derived position would corrupt track, ground speed
 * and every glide calculation without ever looking like a lost fix.
 */
class GpsSource(private val context: Context, private val fixes: Channel) : LocationListener {
    private var locationManager: LocationManager? = null

    /**
     * Subscribes to the receiver, returning the reason it could not be reached
     * rather than throwing.
     */
    fun start(): Exception? {
        // The plugin collects the permission before a session starts, but the
        // system can restart a service after the pilot has revoked it.
        val granted = ContextCompat.checkSelfPermission(
            context,
            Manifest.permission.ACCESS_FINE_LOCATION
        ) == PackageManager.PERMISSION_GRANTED
        if (!granted) {
            return SecurityException("${Manifest.permission.ACCESS_FINE_LOCATION} is not granted")
        }

        val manager = context.getSystemService(Context.LOCATION_SERVICE) as? LocationManager
            ?: return IllegalStateException("the device has no location service")

        return try {
            manager.requestLocationUpdates(
                LocationManager.GPS_PROVIDER,
                MIN_INTERVAL_MILLIS,
                MIN_DISTANCE_METERS,
                this,
                Looper.getMainLooper()
            )
            locationManager = manager
            null
        } catch (e: SecurityException) {
            e
        } catch (e: IllegalArgumentException) {
            e
        }
    }

    fun stop() {
        locationManager?.removeUpdates(this)
        locationManager = null
    }

    override fun onLocationChanged(location: Location) {
        val fix = location.toFix()
        Logger.debug(TAG, "Fix $fix")
        fixes.send(fix)
    }

    override fun onProviderEnabled(provider: String) {
        Logger.info(TAG, "$provider enabled")
    }

    override fun onProviderDisabled(provider: String) {
        Logger.warn(TAG, "$provider disabled, no further fixes will arrive")
    }

    @Deprecated("Implemented for API levels below 30, where it has no default body")
    override fun onStatusChanged(provider: String?, status: Int, extras: Bundle?) {
    }

    private companion object {
        val TAG = Logger.tags("GpsSource")

        const val MIN_INTERVAL_MILLIS = 1000L
        const val MIN_DISTANCE_METERS = 0.0f
    }
}

/**
 * Renders a fix as the JSON the session channel carries.
 *
 * Fielded to match `Fix` in `libs/tauri_plugin_updraft/src/models.rs`, which
 * is the other half of this contract. That struct denies unknown fields, so
 * renaming one here fails the whole fix with a logged error rather than
 * leaving the instrument it feeds frozen at its last reading.
 *
 * `altitude` is height above the WGS84 ellipsoid, which the field name keeps
 * explicit: the correction to mean sea level is a domain conversion and
 * belongs to the core, not here.
 */
private fun Location.toFix(): JSObject = JSObject()
    .put("latitudeDegrees", latitude)
    .put("longitudeDegrees", longitude)
    .put("altitudeEllipsoidMeters", orNull(hasAltitude(), altitude))
    .put("trackDegrees", orNull(hasBearing(), bearing.toDouble()))
    .put("groundSpeedMetersPerSecond", orNull(hasSpeed(), speed.toDouble()))

/**
 * `Location` reports 0.0 rather than nothing for a value it does not have, so
 * an unchecked read makes a stationary glider look like it is tracking due
 * north at sea level. An explicit null instead leaves the last real reading
 * standing.
 */
private fun orNull(has: Boolean, value: Double): Any = if (has) value else JSONObject.NULL
