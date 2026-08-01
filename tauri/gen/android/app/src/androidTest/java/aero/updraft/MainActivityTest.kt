package aero.updraft

import android.Manifest
import android.view.WindowManager
import androidx.test.core.app.ActivityScenario
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.rule.GrantPermissionRule
import org.junit.Assert.assertEquals
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class MainActivityTest {
  @get:Rule
  val startupPermissions: GrantPermissionRule =
    GrantPermissionRule.grant(
      Manifest.permission.ACCESS_FINE_LOCATION,
      Manifest.permission.ACCESS_COARSE_LOCATION,
      Manifest.permission.BLUETOOTH_CONNECT,
      Manifest.permission.POST_NOTIFICATIONS,
    )

  @Test
  fun visibleActivityKeepsScreenAwakeAfterRecreation() {
    ActivityScenario.launch(MainActivity::class.java).use { scenario ->
      scenario.onActivity(::assertKeepsScreenAwake)
      scenario.recreate()
      scenario.onActivity(::assertKeepsScreenAwake)
    }
  }

  private fun assertKeepsScreenAwake(activity: MainActivity) {
    val keepScreenOn = WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON
    assertEquals(keepScreenOn, activity.window.attributes.flags and keepScreenOn)
  }
}
