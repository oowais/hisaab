package com.hisaab.reminders

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.util.Log
import org.json.JSONObject

/** Alarm went off: core decides what to show and arms the next one. */
class AlarmReceiver : BroadcastReceiver() {
    override fun onReceive(ctx: Context, intent: Intent) {
        val id = intent.getStringExtra(EXTRA_ID) ?: return
        val dbPath = Prefs.dbPath(ctx) ?: return
        val out = parseOutcome(Native.onFired(dbPath, id, tz())) ?: return
        out.optJSONObject("show")?.let { Notifier.show(ctx, it) }
        Scheduler.armNext(ctx, id, out.optJSONObject("next"))
    }
}

/**
 * Notification "Pay now" (that charge paid today, then "Paid" with Undo) and
 * its Undo (the reminder back). Core writes; the next alarm, the widget and
 * an open app follow.
 */
class PayReceiver : BroadcastReceiver() {
    override fun onReceive(ctx: Context, intent: Intent) {
        val id = intent.getStringExtra(EXTRA_ID) ?: return
        val reminder = intent.getStringExtra(EXTRA_REMINDER)?.let(::JSONObject) ?: return
        val dbPath = Prefs.dbPath(ctx) ?: return
        val pending = goAsync()
        Thread {
            try {
                if (intent.action == ACTION_UNDO_PAY) undo(ctx, dbPath, id, reminder, intent.getStringExtra(EXTRA_UNDO))
                else pay(ctx, dbPath, id, reminder)
                PaymentsWidget.refresh(ctx)
                RemindersPlugin.dataChanged()
            } catch (e: Exception) {
                Log.e(TAG, "pay ${intent.action} failed", e)
            } finally {
                pending.finish()
            }
        }.start()
    }

    private fun pay(ctx: Context, dbPath: String, id: String, reminder: JSONObject) {
        val charge = reminder.optJSONObject("charge") ?: return
        val out = parseOutcome(Native.payNow(dbPath, id, charge.toString(), tz()))
        if (out == null) {
            // Gone or changed since (e.g. edited in the app): nothing to pay here.
            Notifier.cancel(ctx, id)
            return
        }
        Scheduler.armNext(ctx, id, out.optJSONObject("next"))
        Notifier.showPaid(ctx, reminder, out.getString("label"), out.optJSONObject("undo"))
    }

    private fun undo(ctx: Context, dbPath: String, id: String, reminder: JSONObject, undo: String?) {
        val out = parseOutcome(Native.undoPay(dbPath, id, undo ?: return, tz())) ?: return
        Scheduler.armNext(ctx, id, out.optJSONObject("next"))
        Notifier.show(ctx, reminder, quiet = true)
    }
}

/** Alarms are wiped on reboot / app update; time changes need re-arming too. */
class RestoreReceiver : BroadcastReceiver() {
    override fun onReceive(ctx: Context, intent: Intent) {
        Log.i(TAG, "restore on ${intent.action}")
        Scheduler.restoreAll(ctx)
    }
}
