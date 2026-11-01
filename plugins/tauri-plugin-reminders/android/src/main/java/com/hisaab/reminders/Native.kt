package com.hisaab.reminders

/**
 * JNI bridge into the app's Rust core (`src-tauri/src/jni.rs` → hisaab-core).
 * Runs without the webview, e.g. when an alarm fires with the app closed.
 * All functions return JSON; failures are `{"error": "..."}`.
 */
object Native {
    init {
        System.loadLibrary("hisaab_lib")
    }

    /** Alarm went off. → `{"show": Reminder?, "next": Reminder?}` */
    external fun onFired(dbPath: String, subscriptionId: String, tz: String): String

    /** Every subscription's next reminder: `[Reminder]` */
    external fun reminders(dbPath: String, tz: String): String

    /** Snooze overlay quick picks for now in [tz]. → `[{"label","atMs"}]` */
    external fun snoozePicks(tz: String): String

    /** Snooze until [untilMs]. → `{"next": Reminder?}` */
    external fun snooze(dbPath: String, subscriptionId: String, untilMs: Long, tz: String): String

    /** Consistent snapshot of the database to [outPath]. → `{}` */
    external fun exportTo(dbPath: String, outPath: String): String

    /** One background sync round. → `{"reminders"?: [Reminder], "skipped"?, "problem"?}` */
    external fun syncOnce(dbPath: String, keyPath: String, tz: String): String
}
