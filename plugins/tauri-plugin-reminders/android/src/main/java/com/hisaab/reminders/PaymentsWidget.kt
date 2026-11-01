package com.hisaab.reminders

import android.appwidget.AppWidgetManager
import android.appwidget.AppWidgetProvider
import android.content.ComponentName
import android.content.Context
import android.os.Bundle
import android.util.Log
import android.view.View
import android.widget.RemoteViews
import org.json.JSONObject
import java.text.NumberFormat
import java.time.LocalDate
import java.time.format.DateTimeFormatter
import java.time.temporal.ChronoUnit
import java.util.Currency

/**
 * Home-screen widget (Befikr's, for payments): what the running payments
 * cost a month, then the next charges (name, when, amount); tap one for
 * its screen, + for Quick add. Rows are fixed views, as many as fit.
 * Refreshed after every change ([refresh], from `arm`) and every 30 minutes
 * ("in 3 days" moves with the clock).
 */
class PaymentsWidget : AppWidgetProvider() {
    override fun onUpdate(ctx: Context, mgr: AppWidgetManager, ids: IntArray) {
        val data = load(ctx)
        ids.forEach { update(ctx, mgr, it, data) }
    }

    override fun onAppWidgetOptionsChanged(ctx: Context, mgr: AppWidgetManager, id: Int, options: Bundle) =
        update(ctx, mgr, id, load(ctx))

    companion object {
        private val ROWS = arrayOf(
            intArrayOf(R.id.row0, R.id.name0, R.id.when0, R.id.amount0),
            intArrayOf(R.id.row1, R.id.name1, R.id.when1, R.id.amount1),
            intArrayOf(R.id.row2, R.id.name2, R.id.when2, R.id.amount2),
            intArrayOf(R.id.row3, R.id.name3, R.id.when3, R.id.amount3),
            intArrayOf(R.id.row4, R.id.name4, R.id.when4, R.id.amount4),
        )
        /** Header and padding, then 40dp per row (see widget_payments.xml). */
        private const val HEADER_DP = 64
        private const val ROW_DP = 40

        /** Redraw every placed widget (after a change anywhere). */
        fun refresh(ctx: Context) {
            try {
                val mgr = AppWidgetManager.getInstance(ctx) ?: return
                val ids = mgr.getAppWidgetIds(ComponentName(ctx, PaymentsWidget::class.java))
                if (ids.isEmpty()) return
                val data = load(ctx)
                ids.forEach { update(ctx, mgr, it, data) }
            } catch (e: Exception) {
                Log.w(TAG, "widget refresh failed", e)
            }
        }

        /** From Rust; null when the database isn't known yet. */
        private fun load(ctx: Context): JSONObject? {
            val dbPath = Prefs.dbPath(ctx) ?: return null
            return parseOutcome(Native.widget(dbPath, tz(), ROWS.size))?.optJSONObject("widget")
        }

        private fun money(minor: Long, code: String): String {
            val f = NumberFormat.getCurrencyInstance()
            val digits = runCatching { Currency.getInstance(code) }.getOrNull()?.let {
                f.currency = it
                it.defaultFractionDigits.coerceAtLeast(0)
            } ?: 2
            f.minimumFractionDigits = digits
            f.maximumFractionDigits = digits
            return runCatching { f.format(minor / Math.pow(10.0, digits.toDouble())) }.getOrDefault("$minor $code")
        }

        private fun whenText(today: LocalDate, due: LocalDate): String = when (val n = ChronoUnit.DAYS.between(today, due)) {
            0L -> "Today"
            1L -> "Tomorrow"
            in 2L..6L -> "In $n days"
            else -> due.format(DateTimeFormatter.ofPattern("EEE d MMM"))
        }

        private fun update(ctx: Context, mgr: AppWidgetManager, widgetId: Int, data: JSONObject?) {
            val views = RemoteViews(ctx.packageName, R.layout.widget_payments)
            views.setOnClickPendingIntent(R.id.widget_header, launchPendingIntent(ctx, "widget", "/"))
            views.setOnClickPendingIntent(R.id.widget_add, launchPendingIntent(ctx, "widget-add", "/?add=1"))

            val charges = data?.optJSONArray("charges")
            val n = charges?.length() ?: 0
            val main = data?.optString("mainCurrency") ?: "EUR"
            views.setTextViewText(
                R.id.widget_header,
                if (data == null) "Hisaab" else "${money(data.optLong("monthlyTotal"), main)} / month",
            )
            val height = mgr.getAppWidgetOptions(widgetId).getInt(AppWidgetManager.OPTION_APPWIDGET_MAX_HEIGHT, 180)
            val fit = ((height - HEADER_DP) / ROW_DP).coerceIn(1, ROWS.size)
            views.setViewVisibility(R.id.widget_empty, if (data != null && n == 0) View.VISIBLE else View.GONE)
            val today = data?.optString("today")?.let { runCatching { LocalDate.parse(it) }.getOrNull() } ?: LocalDate.now()
            for ((i, ids) in ROWS.withIndex()) {
                val c = if (i < fit) charges?.optJSONObject(i) else null
                if (c == null) {
                    views.setViewVisibility(ids[0], View.GONE)
                    continue
                }
                val id = c.getString("subscriptionId")
                views.setViewVisibility(ids[0], View.VISIBLE)
                views.setTextViewText(ids[1], c.optString("name"))
                views.setTextViewText(ids[2], whenText(today, LocalDate.parse(c.getString("dueDate"))))
                views.setTextViewText(ids[3], money(c.optLong("amountMinor"), c.optString("currency")))
                views.setOnClickPendingIntent(ids[0], launchPendingIntent(ctx, id, "/sub/$id"))
            }
            mgr.updateAppWidget(widgetId, views)
        }
    }
}
