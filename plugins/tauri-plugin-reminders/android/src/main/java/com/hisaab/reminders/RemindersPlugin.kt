package com.hisaab.reminders

import android.app.Activity
import android.webkit.WebView
import app.tauri.annotation.Command
import app.tauri.annotation.InvokeArg
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin

@InvokeArg
class InstallUpdateArgs {
    lateinit var url: String
}

@TauriPlugin
class RemindersPlugin(private val activity: Activity) : Plugin(activity) {
    override fun load(webView: WebView) {
        // Page background until the splash paints (no white flash on cold start).
        webView.setBackgroundColor(activity.getColor(R.color.hisaab_window))
    }

    /** Back on the first page: to the background like other apps (the process stays, so it reopens warm). */
    @Command
    fun moveToBack(invoke: Invoke) {
        activity.moveTaskToBack(true)
        invoke.resolve()
    }

    @Command
    fun installUpdate(invoke: Invoke) {
        val url = invoke.parseArgs(InstallUpdateArgs::class.java).url
        if (!Updater.canInstall(activity)) {
            Updater.openInstallSettings(activity)
            invoke.resolve(JSObject().apply { put("needsPermission", true) })
            return
        }
        background(invoke) {
            Updater.downloadAndInstall(activity.applicationContext, url) { done, total ->
                trigger("updateProgress", JSObject().apply { put("done", done); put("total", total) })
            }
            JSObject().apply { put("needsPermission", false) }
        }
    }

    private fun background(invoke: Invoke, work: () -> JSObject) {
        Thread {
            try {
                invoke.resolve(work())
            } catch (e: Exception) {
                invoke.reject(e.message ?: e.toString())
            }
        }.start()
    }
}
