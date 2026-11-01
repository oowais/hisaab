package com.hisaab.reminders

import android.Manifest
import android.app.Activity
import android.app.AlarmManager
import android.content.Intent
import android.net.Uri
import android.os.Build
import android.os.PowerManager
import android.provider.Settings
import android.webkit.WebView
import androidx.activity.result.ActivityResult
import androidx.core.app.ActivityCompat
import androidx.core.app.NotificationManagerCompat
import app.tauri.annotation.ActivityCallback
import app.tauri.annotation.Command
import app.tauri.annotation.InvokeArg
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin
import org.json.JSONArray

@InvokeArg
class InstallUpdateArgs {
    lateinit var url: String
}

@InvokeArg
class ArmArgs {
    lateinit var dbPath: String
    /** Core `Reminder` list as JSON: exactly these get an alarm. */
    lateinit var reminders: String
}

@TauriPlugin
class RemindersPlugin(private val activity: Activity) : Plugin(activity) {
    private var launchRoute: String? = null
    /** Daily backup and background sync get set up on the first arm. */
    private var configured = false

    override fun load(webView: WebView) {
        // Page background until the splash paints (no white flash on cold start).
        webView.setBackgroundColor(activity.getColor(R.color.hisaab_window))
        Notifier.ensureChannel(activity)
        launchRoute = activity.intent?.getStringExtra(EXTRA_ROUTE)
    }

    /** A notification tapped while the app is open: go to its route. */
    override fun onNewIntent(intent: Intent) {
        val route = intent.getStringExtra(EXTRA_ROUTE) ?: return
        launchRoute = route
        trigger("launchRoute", JSObject().apply { put("route", route) })
    }

    /** Route the app was opened with (a notification), once. */
    @Command
    fun takeLaunchRoute(invoke: Invoke) {
        val route = launchRoute
        launchRoute = null
        invoke.resolve(JSObject().apply { put("route", route) })
    }

    /**
     * Arm exactly these reminders; remembers the database for the receivers
     * and backups. The first time: schedule the daily backup and background
     * sync, and run a backup that's due.
     */
    @Command
    fun arm(invoke: Invoke) {
        val args = invoke.parseArgs(ArmArgs::class.java)
        Prefs.setDbPath(activity, args.dbPath)
        Scheduler.replaceAll(activity, JSONArray(args.reminders))
        if (!configured) {
            configured = true
            Backup.scheduleDaily(activity)
            SyncWorker.schedule(activity)
            Thread { Backup.runIfDue(activity.applicationContext) }.start()
        }
        invoke.resolve()
    }

    @Command
    fun status(invoke: Invoke) {
        val am = activity.getSystemService(AlarmManager::class.java)
        val pm = activity.getSystemService(PowerManager::class.java)
        invoke.resolve(JSObject().apply {
            put("notifications", NotificationManagerCompat.from(activity).areNotificationsEnabled())
            put("exactAlarms", Build.VERSION.SDK_INT < Build.VERSION_CODES.S || am.canScheduleExactAlarms())
            put("batteryUnrestricted", pm.isIgnoringBatteryOptimizations(activity.packageName))
        })
    }

    @Command
    fun requestNotifications(invoke: Invoke) {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
            ActivityCompat.requestPermissions(activity, arrayOf(Manifest.permission.POST_NOTIFICATIONS), 4201)
        }
        invoke.resolve()
    }

    @Command
    fun openExactAlarmSettings(invoke: Invoke) {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) {
            activity.startActivity(
                Intent(Settings.ACTION_REQUEST_SCHEDULE_EXACT_ALARM, Uri.parse("package:${activity.packageName}"))
            )
        }
        invoke.resolve()
    }

    @Command
    fun openBatterySettings(invoke: Invoke) {
        activity.startActivity(
            Intent(Settings.ACTION_REQUEST_IGNORE_BATTERY_OPTIMIZATIONS, Uri.parse("package:${activity.packageName}"))
        )
        invoke.resolve()
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

    // ---- Backups (Befikr's) ----

    private fun status() = JSObject(Backup.status(activity).toString())

    @Command
    fun backupStatus(invoke: Invoke) = invoke.resolve(status())

    @Command
    fun pickBackupFolder(invoke: Invoke) {
        val intent = Intent(Intent.ACTION_OPEN_DOCUMENT_TREE)
            .addFlags(Intent.FLAG_GRANT_PERSISTABLE_URI_PERMISSION)
            .addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION or Intent.FLAG_GRANT_WRITE_URI_PERMISSION)
        startActivityForResult(invoke, intent, "onBackupFolder")
    }

    @ActivityCallback
    fun onBackupFolder(invoke: Invoke, result: ActivityResult) {
        val uri = result.data?.data
        if (result.resultCode != Activity.RESULT_OK || uri == null) return invoke.resolve(status())
        activity.contentResolver.takePersistableUriPermission(
            uri, Intent.FLAG_GRANT_READ_URI_PERMISSION or Intent.FLAG_GRANT_WRITE_URI_PERMISSION
        )
        Backup.tree(activity)?.takeIf { it != uri }?.let { old ->
            runCatching { activity.contentResolver.releasePersistableUriPermission(old, Intent.FLAG_GRANT_READ_URI_PERMISSION or Intent.FLAG_GRANT_WRITE_URI_PERMISSION) }
        }
        Backup.setTree(activity, uri)
        // First backup right away, so the folder is proven writable.
        background(invoke) {
            runCatching { Backup.runNow(activity) }
            status()
        }
    }

    @Command
    fun clearBackupFolder(invoke: Invoke) {
        Backup.tree(activity)?.let { old ->
            runCatching { activity.contentResolver.releasePersistableUriPermission(old, Intent.FLAG_GRANT_READ_URI_PERMISSION or Intent.FLAG_GRANT_WRITE_URI_PERMISSION) }
        }
        Backup.setTree(activity, null)
        invoke.resolve(status())
    }

    @Command
    fun backupNow(invoke: Invoke) = background(invoke) {
        Backup.runNow(activity)
        status()
    }

    /** Save one copy wherever the user chooses. */
    @Command
    fun exportBackup(invoke: Invoke) {
        val intent = Intent(Intent.ACTION_CREATE_DOCUMENT)
            .addCategory(Intent.CATEGORY_OPENABLE)
            .setType("application/octet-stream")
            .putExtra(Intent.EXTRA_TITLE, Backup.fileName())
        startActivityForResult(invoke, intent, "onExportFile")
    }

    @ActivityCallback
    fun onExportFile(invoke: Invoke, result: ActivityResult) {
        val uri = result.data?.data
        if (result.resultCode != Activity.RESULT_OK || uri == null) return invoke.resolve(JSObject().apply { put("saved", false) })
        background(invoke) {
            Backup.exportTo(activity, uri)
            JSObject().apply { put("saved", true) }
        }
    }

    /** Pick a backup file; it is copied into the cache and its path returned for Rust to restore. */
    @Command
    fun pickImportFile(invoke: Invoke) {
        val intent = Intent(Intent.ACTION_OPEN_DOCUMENT).addCategory(Intent.CATEGORY_OPENABLE).setType("*/*")
        startActivityForResult(invoke, intent, "onImportFile")
    }

    @ActivityCallback
    fun onImportFile(invoke: Invoke, result: ActivityResult) {
        val uri = result.data?.data
        if (result.resultCode != Activity.RESULT_OK || uri == null) return invoke.resolve(JSObject())
        background(invoke) {
            val file = Backup.importToCache(activity, uri)
            JSObject().apply { put("path", file.absolutePath) }
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
