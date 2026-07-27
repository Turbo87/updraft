package aero.updraft.mobile

import android.app.Service
import android.content.pm.ServiceInfo
import android.os.Build
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class SessionServiceTest {
    @Test
    fun `uses location foreground type for a location source`() {
        assertEquals(
            ServiceInfo.FOREGROUND_SERVICE_TYPE_LOCATION,
            foregroundServiceTypes(location = true, spp = false)
        )
    }

    @Test
    fun `uses connected device foreground type for an SPP source`() {
        assertEquals(
            ServiceInfo.FOREGROUND_SERVICE_TYPE_CONNECTED_DEVICE,
            foregroundServiceTypes(location = false, spp = true)
        )
    }

    @Test
    fun `combines foreground types for location and SPP sources`() {
        assertEquals(
            ServiceInfo.FOREGROUND_SERVICE_TYPE_LOCATION or
                ServiceInfo.FOREGROUND_SERVICE_TYPE_CONNECTED_DEVICE,
            foregroundServiceTypes(location = true, spp = true)
        )
    }

    @Test
    fun `uses no foreground type without a source`() {
        assertEquals(0, foregroundServiceTypes(location = false, spp = false))
    }

    @Test
    fun `continues through every permission prompt after source denials`() {
        val deniedSources = SourcePermissions(location = false, spp = false)

        assertEquals(
            StartupAction.RequestLocation,
            startupAction(StartupStage.Location, deniedSources, notificationsGranted = false, Build.VERSION_CODES.TIRAMISU)
        )
        assertEquals(
            StartupAction.RequestNearbyDevices,
            startupAction(
                StartupStage.NearbyDevices,
                deniedSources,
                notificationsGranted = false,
                Build.VERSION_CODES.TIRAMISU
            )
        )
        assertEquals(
            StartupAction.RequestNotifications,
            startupAction(
                StartupStage.Notifications,
                deniedSources,
                notificationsGranted = false,
                Build.VERSION_CODES.TIRAMISU
            )
        )
        assertEquals(
            StartupAction.Reject,
            startupAction(
                StartupStage.Notifications,
                deniedSources,
                notificationsGranted = true,
                Build.VERSION_CODES.TIRAMISU
            )
        )
    }

    @Test
    fun `starts the permitted source combinations after optional notifications`() {
        assertEquals(
            StartupAction.StartService(SourcePermissions(location = true, spp = false)),
            startupAction(
                StartupStage.Notifications,
                SourcePermissions(location = true, spp = false),
                notificationsGranted = true,
                Build.VERSION_CODES.TIRAMISU
            )
        )
        assertEquals(
            StartupAction.StartService(SourcePermissions(location = false, spp = true)),
            startupAction(
                StartupStage.Notifications,
                SourcePermissions(location = false, spp = true),
                notificationsGranted = false,
                Build.VERSION_CODES.S
            )
        )
        assertEquals(
            StartupAction.StartService(SourcePermissions(location = true, spp = true)),
            startupAction(
                StartupStage.Notifications,
                SourcePermissions(location = true, spp = true),
                notificationsGranted = true,
                Build.VERSION_CODES.TIRAMISU
            )
        )
    }

    @Test
    fun `treats Nearby Devices as granted before Android S`() {
        val sources = sourcePermissions(
            locationGranted = false,
            nearbyDevicesGranted = false,
            sdkInt = Build.VERSION_CODES.S - 1
        )

        assertEquals(SourcePermissions(location = false, spp = true), sources)
        assertEquals(
            StartupAction.StartService(sources),
            startupAction(
                StartupStage.Notifications,
                sources,
                notificationsGranted = true,
                Build.VERSION_CODES.S - 1
            )
        )
    }

    @Test
    fun `finishes after notification denial without requesting it again`() {
        val locationOnly = SourcePermissions(location = true, spp = false)

        assertEquals(
            StartupAction.StartService(locationOnly),
            startupAction(
                StartupStage.Finalize,
                locationOnly,
                notificationsGranted = false,
                Build.VERSION_CODES.TIRAMISU
            )
        )
    }

    @Test
    fun `retains activated types across starts and resets on destruction`() {
        val types = ForegroundServiceTypeState()

        assertEquals(ServiceInfo.FOREGROUND_SERVICE_TYPE_LOCATION, types.activate(location = true, spp = false))
        assertEquals(
            ServiceInfo.FOREGROUND_SERVICE_TYPE_LOCATION or
                ServiceInfo.FOREGROUND_SERVICE_TYPE_CONNECTED_DEVICE,
            types.activate(location = false, spp = true)
        )
        assertEquals(
            ServiceInfo.FOREGROUND_SERVICE_TYPE_LOCATION or
                ServiceInfo.FOREGROUND_SERVICE_TYPE_CONNECTED_DEVICE,
            types.activate(location = false, spp = false)
        )

        types.reset()

        assertEquals(0, types.current)
        assertFalse(types.isForeground)
    }

    @Test
    fun `requested SPP type still stops a fresh failed service start`() {
        val state = ForegroundServiceTypeState()

        state.activate(location = false, spp = true)

        assertFalse(state.isForeground)
        val failure = state.failedSppStart()
        assertEquals(FailedSppServiceStart.Stop, failure)
        assertEquals(Service.START_NOT_STICKY, failure.startMode)
    }

    @Test
    fun `existing foreground session survives a failed SPP type upgrade`() {
        val state = ForegroundServiceTypeState()
        state.markForeground()

        state.activate(location = false, spp = true)

        assertTrue(state.isForeground)
        val failure = state.failedSppStart()
        assertEquals(FailedSppServiceStart.Keep, failure)
        assertEquals(Service.START_STICKY, failure.startMode)
    }
}
