package com.hisaab.reminders

import android.content.Context
import android.util.Log
import androidx.work.Constraints
import androidx.work.ExistingPeriodicWorkPolicy
import androidx.work.NetworkType
import androidx.work.PeriodicWorkRequestBuilder
import androidx.work.WorkManager
import androidx.work.Worker
import androidx.work.WorkerParameters
import org.json.JSONObject
import java.io.File
import java.util.concurrent.TimeUnit

/**
 * Background sync while the app is closed (Befikr's): about every 15 minutes
 * (Android's minimum; Doze may stretch it), with a network, one sync round
 * with every paired device (Rust, `Native.syncOnce`), then re-arm the
 * reminders when something merged. So a payment added on another device
 * reminds here without opening Hisaab. Rust skips the run when sync is off,
 * nothing is paired, or the app is open and syncing itself.
 */
class SyncWorker(ctx: Context, params: WorkerParameters) : Worker(ctx, params) {
    override fun doWork(): Result {
        val ctx = applicationContext
        val db = Prefs.dbPath(ctx) ?: return Result.success()
        // The device key lives next to the database (see src-tauri/src/sync.rs).
        val key = File(File(db).parentFile, "sync.key").absolutePath
        val out = runCatching { JSONObject(Native.syncOnce(db, key, tz())) }.getOrElse {
            Log.w(TAG, "background sync failed", it)
            return Result.success()
        }
        if (out.has("error")) {
            Log.w(TAG, "background sync: ${out.getString("error")}")
            return Result.success()
        }
        out.optJSONArray("reminders")?.let {
            Scheduler.replaceAll(ctx, it)
            PaymentsWidget.refresh(ctx)
        }
        Log.i(TAG, "background sync: " + (if (out.has("reminders")) "merged" else "nothing new") +
            (out.optString("skipped").takeIf { it.isNotEmpty() }?.let { ", skipped ($it)" } ?: "") +
            (out.optString("problem").takeIf { it.isNotEmpty() }?.let { ", $it" } ?: ""))
        return Result.success()
    }

    companion object {
        private const val NAME = "hisaab-sync"

        /** Idempotent: keeps an already scheduled run. Survives reboots (WorkManager). */
        fun schedule(ctx: Context) {
            val online = Constraints.Builder().setRequiredNetworkType(NetworkType.CONNECTED).build()
            val request = PeriodicWorkRequestBuilder<SyncWorker>(15, TimeUnit.MINUTES).setConstraints(online).build()
            WorkManager.getInstance(ctx).enqueueUniquePeriodicWork(NAME, ExistingPeriodicWorkPolicy.KEEP, request)
        }
    }
}
