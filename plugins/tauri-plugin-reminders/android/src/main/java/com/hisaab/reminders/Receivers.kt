package com.hisaab.reminders

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.util.Log

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

/** Alarms are wiped on reboot / app update; time changes need re-arming too. */
class RestoreReceiver : BroadcastReceiver() {
    override fun onReceive(ctx: Context, intent: Intent) {
        Log.i(TAG, "restore on ${intent.action}")
        Scheduler.restoreAll(ctx)
    }
}
