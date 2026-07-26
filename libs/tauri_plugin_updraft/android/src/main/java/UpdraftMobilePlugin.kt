package aero.updraft.mobile

import android.app.Activity
import app.tauri.annotation.Command
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.Plugin

@TauriPlugin
class UpdraftMobilePlugin(private val activity: Activity) : Plugin(activity) {
    @Command
    fun startSession(invoke: Invoke) {
        invoke.resolve()
    }

    @Command
    fun stopSession(invoke: Invoke) {
        invoke.resolve()
    }
}
