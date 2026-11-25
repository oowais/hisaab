package com.hisaab.reminders

import android.app.AlarmManager
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.content.Context
import android.content.Intent
import android.net.Uri
import android.os.Build
import android.util.Log
import androidx.core.app.NotificationCompat
import androidx.core.app.NotificationManagerCompat
import org.json.JSONArray
import org.json.JSONObject
import java.util.TimeZone

internal const val TAG = "HisaabReminders"
/** Channel settings are immutable once created; bump the id to change them. */
internal const val CHANNEL_ID = "reminders-v1"
private val VIBRATION = longArrayOf(0, 300, 150, 300)
internal const val EXTRA_ID = "hisaab.id"
internal const val EXTRA_TITLE = "hisaab.title"
internal const val EXTRA_ROUTE = "hisaab.route"
internal const val EXTRA_REMINDER = "hisaab.reminder"
internal const val EXTRA_UNDO = "hisaab.undo"
internal const val ACTION_FIRE = "com.hisaab.reminders.FIRE"
internal const val ACTION_PAY = "com.hisaab.reminders.PAY"
internal const val ACTION_UNDO_PAY = "com.hisaab.reminders.UNDO_PAY"
/** How long "Paid today" stays up with its Undo. */
private const val PAID_UNDO_MS = 10_000L
private const val PREFS = "hisaab.reminders"
private const val PREF_DB_PATH = "dbPath"
private const val PREF_ARMED = "armed"

internal object Prefs {
    private fun prefs(ctx: Context) = ctx.getSharedPreferences(PREFS, Context.MODE_PRIVATE)

    fun dbPath(ctx: Context): String? = prefs(ctx).getString(PREF_DB_PATH, null)

    fun setDbPath(ctx: Context, path: String) = prefs(ctx).edit().putString(PREF_DB_PATH, path).apply()

    /** Subscription ids with an alarm armed, so a full replace can cancel the rest. */
    fun armed(ctx: Context): Set<String> = prefs(ctx).getStringSet(PREF_ARMED, emptySet()) ?: emptySet()

    fun setArmed(ctx: Context, ids: Set<String>) = prefs(ctx).edit().putStringSet(PREF_ARMED, ids).apply()
}

internal fun tz(): String = TimeZone.getDefault().id

/** One alarm and one notification per subscription; both keyed by its id. */
internal fun notificationId(id: String) = id.hashCode()

private fun uri(id: String, kind: String) = Uri.parse("hisaab://$kind/$id")

private const val PI_FLAGS = PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT

internal object Scheduler {
    /** setExactAndAllowWhileIdle; inexact fallback without the permission. */
    fun schedule(ctx: Context, id: String, atMs: Long) {
        val am = ctx.getSystemService(AlarmManager::class.java)
        val pi = firePendingIntent(ctx, id)
        val exactAllowed = Build.VERSION.SDK_INT < Build.VERSION_CODES.S || am.canScheduleExactAlarms()
        if (exactAllowed) {
            am.setExactAndAllowWhileIdle(AlarmManager.RTC_WAKEUP, atMs, pi)
        } else {
            Log.w(TAG, "exact alarms not permitted; falling back to inexact for $id")
            am.setAndAllowWhileIdle(AlarmManager.RTC_WAKEUP, atMs, pi)
        }
    }

    fun cancel(ctx: Context, id: String) {
        ctx.getSystemService(AlarmManager::class.java).cancel(firePendingIntent(ctx, id))
    }

    /** Arm exactly these reminders (core `Reminder` JSON), cancelling all others. */
    fun replaceAll(ctx: Context, reminders: JSONArray) {
        val ids = mutableSetOf<String>()
        for (i in 0 until reminders.length()) {
            val r = reminders.getJSONObject(i)
            val id = r.getString("subscriptionId")
            schedule(ctx, id, r.getLong("atMs"))
            ids += id
        }
        for (gone in Prefs.armed(ctx) - ids) cancel(ctx, gone)
        Prefs.setArmed(ctx, ids)
        Log.i(TAG, "armed ${ids.size} reminders")
    }

    /** Re-arm what core says is next (boot, update, time change). */
    fun restoreAll(ctx: Context) {
        val dbPath = Prefs.dbPath(ctx) ?: return
        val raw = Native.reminders(dbPath, tz())
        if (raw.startsWith("{")) {
            parseOutcome(raw) // logs the error
            return
        }
        replaceAll(ctx, JSONArray(raw))
    }

    /** One subscription's next alarm (core `Reminder` JSON or null). */
    fun armNext(ctx: Context, id: String, next: JSONObject?) {
        val armed = Prefs.armed(ctx).toMutableSet()
        if (next != null) {
            schedule(ctx, id, next.getLong("atMs"))
            armed += id
        } else {
            cancel(ctx, id)
            armed -= id
        }
        Prefs.setArmed(ctx, armed)
    }

    private fun firePendingIntent(ctx: Context, id: String): PendingIntent {
        val intent = Intent(ctx, AlarmReceiver::class.java)
            .setAction(ACTION_FIRE)
            .setData(uri(id, "fire"))
            .putExtra(EXTRA_ID, id)
        return PendingIntent.getBroadcast(ctx, 0, intent, PI_FLAGS)
    }
}

/** Opens the app on [route]; the webview reads [EXTRA_ROUTE] via the plugin. */
internal fun launchPendingIntent(ctx: Context, id: String, route: String): PendingIntent {
    val intent = ctx.packageManager.getLaunchIntentForPackage(ctx.packageName)!!
        .setData(uri(id, "open"))
        .addFlags(Intent.FLAG_ACTIVITY_SINGLE_TOP or Intent.FLAG_ACTIVITY_CLEAR_TOP)
        .putExtra(EXTRA_ROUTE, route)
    return PendingIntent.getActivity(ctx, 0, intent, PI_FLAGS)
}

internal object Notifier {
    fun ensureChannel(ctx: Context) {
        val nm = ctx.getSystemService(NotificationManager::class.java)
        if (nm.getNotificationChannel(CHANNEL_ID) == null) {
            nm.createNotificationChannel(
                NotificationChannel(CHANNEL_ID, "Upcoming charges", NotificationManager.IMPORTANCE_HIGH).apply {
                    enableVibration(true)
                    vibrationPattern = VIBRATION
                }
            )
        }
    }

    /**
     * A reminder (core `Reminder` JSON): tap opens the subscription; Pay now
     * (a charge's) marks it paid today; Snooze shows the overlay. [quiet]:
     * put back after an undo, without sound.
     */
    fun show(ctx: Context, r: JSONObject, quiet: Boolean = false) {
        ensureChannel(ctx)
        val id = r.getString("subscriptionId")
        val title = r.getString("title")
        val route = r.optString("route", "/")
        val n = NotificationCompat.Builder(ctx, CHANNEL_ID)
            .setSmallIcon(smallIcon(ctx))
            .setContentTitle(title)
            .setContentText(r.optString("text"))
            .setCategory(NotificationCompat.CATEGORY_REMINDER)
            .setPriority(NotificationCompat.PRIORITY_HIGH)
            .setVibrate(VIBRATION)
            .setSilent(quiet)
            .setAutoCancel(true)
            // Unique group per subscription: try to stop Android auto-bundling.
            .setGroup("sub-$id")
            .setContentIntent(launchPendingIntent(ctx, id, route))
        if (r.optJSONObject("charge") != null) {
            n.addAction(0, "Pay now", payPendingIntent(ctx, ACTION_PAY, r, null))
        }
        n.addAction(
            0,
            "Snooze",
            PendingIntent.getActivity(ctx, 0, SnoozeActivity.intent(ctx, id, title), PI_FLAGS)
        )
        n.addAction(0, "Open", launchPendingIntent(ctx, id, route))
        post(ctx, id, n)
    }

    /** After Pay now, in the reminder's place for a few seconds: [label] and Undo (when something changed). */
    fun showPaid(ctx: Context, r: JSONObject, label: String, undo: JSONObject?) {
        val id = r.getString("subscriptionId")
        val n = NotificationCompat.Builder(ctx, CHANNEL_ID)
            .setSmallIcon(smallIcon(ctx))
            .setContentTitle(label)
            .setCategory(NotificationCompat.CATEGORY_STATUS)
            .setSilent(true)
            .setAutoCancel(true)
            .setTimeoutAfter(PAID_UNDO_MS)
            .setGroup("sub-$id")
            .setContentIntent(launchPendingIntent(ctx, id, r.optString("route", "/")))
        if (undo != null) n.addAction(0, "Undo", payPendingIntent(ctx, ACTION_UNDO_PAY, r, undo))
        post(ctx, id, n)
    }

    private fun post(ctx: Context, id: String, n: NotificationCompat.Builder) {
        try {
            NotificationManagerCompat.from(ctx).notify(notificationId(id), n.build())
        } catch (e: SecurityException) {
            Log.e(TAG, "notification permission missing", e)
        }
    }

    private fun payPendingIntent(ctx: Context, action: String, r: JSONObject, undo: JSONObject?): PendingIntent {
        val id = r.getString("subscriptionId")
        val intent = Intent(ctx, PayReceiver::class.java)
            .setAction(action)
            .setData(uri(id, if (action == ACTION_PAY) "pay" else "undo-pay"))
            .putExtra(EXTRA_ID, id)
            .putExtra(EXTRA_REMINDER, r.toString())
        undo?.let { intent.putExtra(EXTRA_UNDO, it.toString()) }
        return PendingIntent.getBroadcast(ctx, 0, intent, PI_FLAGS)
    }

    fun cancel(ctx: Context, id: String) = NotificationManagerCompat.from(ctx).cancel(notificationId(id))

    /** App-provided `ic_stat_hisaab` (src-tauri/android/res), else a system bell. */
    fun smallIcon(ctx: Context): Int {
        val id = ctx.resources.getIdentifier("ic_stat_hisaab", "drawable", ctx.packageName)
        return if (id != 0) id else android.R.drawable.ic_popup_reminder
    }
}

internal fun parseOutcome(raw: String): JSONObject? {
    val o = JSONObject(raw)
    if (o.has("error")) {
        Log.e(TAG, "rust error: ${o.getString("error")}")
        return null
    }
    return o
}
