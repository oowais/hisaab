//! Reminders, computed like charges: never stored, derived from periods.
//! Each subscription has at most one alarm, its next reminder:
//! - `N` days before each charge (the subscription's own `remind_days_before`,
//!   else the default) at the reminder time: "Netflix · €4.99 in 3 days";
//! - 2 days before a trial ends: "Claude trial ends in 2 days".
//!
//! `reminder_state` (this device only) remembers what already fired, so
//! nothing shows twice, and a snoozed reminder. A subscription seen for the
//! first time starts "fired through now": installing the app or adding a
//! payment never fires reminders from the past.
//!
//! Platforms (Android alarms, the desktop scheduler) only arm what
//! `reminders` returns and call `on_fired` when an alarm goes off.

use chrono::{DateTime, Datelike, Duration, LocalResult, NaiveDate, NaiveTime, TimeZone, Timelike, Utc};
use chrono_tz::Tz;
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};

use crate::currency::minor_digits;
use crate::model::*;
use crate::occurrence::occurrences;
use crate::store::Store;
use crate::Result;

/// Days before a trial ends to say so.
pub const TRIAL_NOTICE_DAYS: i64 = 2;
/// An alarm a little early still counts as due (alarms can fire early).
const SLACK_MS: i64 = 60_000;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Reminder {
    /// Also the alarm's id: one per subscription.
    pub subscription_id: String,
    pub title: String,
    pub text: String,
    /// UTC ms.
    pub at_ms: i64,
    /// Where tapping it opens the app.
    pub route: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Fired {
    /// The notification to show now, if any.
    pub show: Option<Reminder>,
    /// The subscription's next alarm.
    pub next: Option<Reminder>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SnoozePick {
    pub label: String,
    pub at_ms: i64,
}

/// An IANA zone; unknown ones fall back to UTC.
pub fn parse_tz(tz: &str) -> Tz {
    tz.parse().unwrap_or(Tz::UTC)
}

/// `day` at `time` in `tz` as UTC ms (a time skipped by DST moves on an hour).
pub fn local_ms(day: NaiveDate, time: NaiveTime, tz: Tz) -> i64 {
    let mut local = day.and_time(time);
    for _ in 0..3 {
        match tz.from_local_datetime(&local) {
            LocalResult::Single(t) | LocalResult::Ambiguous(t, _) => return t.timestamp_millis(),
            LocalResult::None => local += Duration::hours(1),
        }
    }
    Utc.from_utc_datetime(&local).timestamp_millis()
}

pub(crate) fn local_day(ms: i64, tz: Tz) -> NaiveDate {
    Utc.timestamp_millis_opt(ms).single().map_or_else(|| Utc::now().date_naive(), |t| t.with_timezone(&tz).date_naive())
}

/// "€4.99", "$10.00", "¥1500", "CHF 12.00" (notification text; no locale here).
pub fn money_text(minor: i64, currency: &str) -> String {
    let digits = minor_digits(currency);
    let major = minor as f64 / 10f64.powi(digits as i32);
    let n = format!("{major:.prec$}", prec = digits as usize);
    match currency {
        "EUR" => format!("€{n}"),
        "USD" => format!("${n}"),
        "GBP" => format!("£{n}"),
        "JPY" => format!("¥{n}"),
        "INR" => format!("₹{n}"),
        other => format!("{other} {n}"),
    }
}

/// "today", "tomorrow", "in 3 days", seen from `from`.
fn when_text(from: NaiveDate, to: NaiveDate) -> String {
    match (to - from).num_days() {
        0 => "today".into(),
        1 => "tomorrow".into(),
        n => format!("in {n} days"),
    }
}

fn day_text(d: NaiveDate) -> String {
    d.format("%a %-d %b").to_string()
}

struct State {
    fired_through: i64,
    snooze: Option<(i64, String, String)>,
}

impl Store {
    fn reminder_state(&self, id: &str, now_ms: i64) -> Result<State> {
        let row = self
            .conn
            .query_row(
                "SELECT fired_through, snooze_at, snooze_title, snooze_text FROM reminder_state WHERE subscription_id = ?1",
                [id],
                |r| Ok((r.get::<_, i64>(0)?, r.get::<_, Option<i64>>(1)?, r.get::<_, Option<String>>(2)?, r.get::<_, Option<String>>(3)?)),
            )
            .optional()?;
        Ok(match row {
            Some((fired_through, at, title, text)) => State {
                fired_through,
                snooze: at.map(|at| (at, title.unwrap_or_default(), text.unwrap_or_default())),
            },
            None => {
                // First sight: nothing before now fires.
                self.conn.execute(
                    "INSERT INTO reminder_state (subscription_id, fired_through) VALUES (?1, ?2)",
                    params![id, now_ms],
                )?;
                State { fired_through: now_ms, snooze: None }
            }
        })
    }

    /// Every reminder of `s` from `today` on (charges due today or later,
    /// trials ending today or later), by time.
    fn candidates(&self, s: &SubscriptionDetail, today: NaiveDate, tz: Tz) -> Result<Vec<Reminder>> {
        if s.subscription.archived_at.is_some() {
            return Ok(Vec::new());
        }
        let settings = self.settings()?;
        let time = NaiveTime::parse_from_str(&settings.reminder_time, "%H:%M").unwrap_or_default();
        let days = i64::from(s.subscription.remind_days_before.unwrap_or(settings.default_remind_days));
        let name = &s.subscription.name;
        let route = format!("/sub/{}", s.subscription.id);
        let to = today + Duration::days(400);
        let mut out: Vec<Reminder> = occurrences(&s.periods, &s.overrides, today, to, today)
            .into_iter()
            .map(|c| {
                let day = c.due_date - Duration::days(days);
                Reminder {
                    subscription_id: s.subscription.id.clone(),
                    title: format!(
                        "{name} · {}{} {}",
                        if c.estimated { "~" } else { "" },
                        money_text(c.amount_minor, &c.currency),
                        when_text(day, c.due_date)
                    ),
                    text: day_text(c.due_date),
                    at_ms: local_ms(day, time, tz),
                    route: route.clone(),
                }
            })
            .collect();
        for p in s.periods.iter().filter(|p| p.status == PeriodStatus::Trial) {
            let Some(end) = p.end_date.filter(|e| *e >= today) else { continue };
            let day = end - Duration::days(TRIAL_NOTICE_DAYS);
            out.push(Reminder {
                subscription_id: s.subscription.id.clone(),
                title: format!("{name} trial ends {}", when_text(day, end)),
                text: format!("Last day {}", day_text(end)),
                at_ms: local_ms(day, time, tz),
                route: route.clone(),
            });
        }
        // Stable: a trial notice (pushed last) goes before a charge reminder at
        // the same time, and wins: it's the one that needs acting on.
        out.sort_by_key(|r| (r.at_ms, !r.title.contains(" trial ends ")));
        Ok(out)
    }

    fn next_for(&self, s: &SubscriptionDetail, now_ms: i64, tz: Tz) -> Result<Option<Reminder>> {
        let state = self.reminder_state(&s.subscription.id, now_ms)?;
        if let Some((at, title, text)) = state.snooze.filter(|(at, ..)| *at > state.fired_through) {
            return Ok(Some(Reminder {
                subscription_id: s.subscription.id.clone(),
                title,
                text,
                at_ms: at,
                route: format!("/sub/{}", s.subscription.id),
            }));
        }
        let today = local_day(now_ms, tz);
        Ok(self.candidates(s, today, tz)?.into_iter().find(|r| r.at_ms > state.fired_through))
    }

    /// The next reminder of every subscription: what to arm (app start,
    /// after any change, boot).
    pub fn reminders(&self, now_ms: i64, tz: &str) -> Result<Vec<Reminder>> {
        let tz = parse_tz(tz);
        let mut out = Vec::new();
        if !self.settings()?.reminders_on {
            return Ok(out);
        }
        for s in self.subscriptions()? {
            out.extend(self.next_for(&s, now_ms, tz)?);
        }
        Ok(out)
    }

    /// The subscription's alarm went off: what to show (the latest reminder
    /// that is due and not yet shown) and the next alarm.
    pub fn on_fired(&mut self, id: &str, now_ms: i64, tz: &str) -> Result<Fired> {
        let tz = parse_tz(tz);
        let Ok(s) = self.subscription(id) else {
            return Ok(Fired { show: None, next: None });
        };
        if !self.settings()?.reminders_on {
            return Ok(Fired { show: None, next: None });
        }
        let state = self.reminder_state(id, now_ms)?;
        let due = now_ms + SLACK_MS;
        let show = match state.snooze.as_ref().filter(|(at, ..)| *at > state.fired_through && *at <= due) {
            Some((at, title, text)) => Some(Reminder {
                subscription_id: id.into(),
                title: title.clone(),
                text: text.clone(),
                at_ms: *at,
                route: format!("/sub/{id}"),
            }),
            None => self
                .candidates(&s, local_day(now_ms, tz), tz)?
                .into_iter()
                .filter(|r| r.at_ms > state.fired_through && r.at_ms <= due)
                // The latest due one; on a tie the first (the trial notice).
                .rev()
                .max_by_key(|r| r.at_ms),
        };
        if let Some(r) = &show {
            self.conn.execute(
                "UPDATE reminder_state SET fired_through = ?2, snooze_at = NULL, snooze_title = NULL, snooze_text = NULL
                 WHERE subscription_id = ?1",
                params![id, r.at_ms.max(now_ms)],
            )?;
        }
        Ok(Fired { show, next: self.next_for(&s, now_ms, tz)? })
    }

    /// Show the subscription's reminder again at `until_ms`. Returns the
    /// next alarm (that one).
    pub fn snooze_reminder(&mut self, id: &str, until_ms: i64, now_ms: i64, tz: &str) -> Result<Option<Reminder>> {
        let tz = parse_tz(tz);
        let s = self.subscription(id)?;
        let state = self.reminder_state(id, now_ms)?;
        // What's being snoozed: the snoozed one, else the latest shown, else the next.
        let all = self.candidates(&s, local_day(now_ms, tz) - Duration::days(1), tz)?;
        let (title, text) = match state.snooze {
            Some((_, title, text)) => (title, text),
            None => all
                .iter()
                .rfind(|r| r.at_ms <= now_ms + SLACK_MS)
                .or_else(|| all.first())
                .map(|r| (r.title.clone(), r.text.clone()))
                .unwrap_or_else(|| (s.subscription.name.clone(), String::new())),
        };
        self.conn.execute(
            "UPDATE reminder_state SET snooze_at = ?2, snooze_title = ?3, snooze_text = ?4 WHERE subscription_id = ?1",
            params![id, until_ms, title, text],
        )?;
        self.next_for(&s, now_ms, tz)
    }
}

/// Snooze quick picks for `now_ms` in `tz` (Befikr's, for charges: later
/// today, tomorrow morning, in 2 days, next week).
pub fn snooze_picks(now_ms: i64, tz: &str) -> Vec<SnoozePick> {
    let tz = parse_tz(tz);
    let now: DateTime<Tz> = Utc.timestamp_millis_opt(now_ms).single().unwrap_or_else(Utc::now).with_timezone(&tz);
    let today = now.date_naive();
    let at = |days: i64, h: u32| local_ms(today + Duration::days(days), NaiveTime::from_hms_opt(h, 0, 0).unwrap_or_default(), tz);
    let round = |ms: i64| (ms + 299_999).div_euclid(300_000) * 300_000;
    let pick = |label: &str, at_ms| SnoozePick { label: label.into(), at_ms };
    let mut list = vec![pick("In 1 hour", round(now_ms + 3_600_000))];
    if now.hour() < 18 {
        list.push(pick("This evening", at(0, 18)));
    }
    list.push(pick("Tomorrow morning", at(1, 9)));
    list.push(pick("In 2 days", at(2, 9)));
    let to_monday = 7 - i64::from(now.weekday().num_days_from_monday());
    list.push(pick("Next week", at(to_monday, 9)));
    list
}
