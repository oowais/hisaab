//! Stats: what was spent month by month (from the charges the periods give,
//! so past prices stay right), what the running payments cost a year split
//! by category, payment method and currency, and what pausing and
//! cancelling saved. All in the main currency; charges at their day's rate.

use chrono::{Datelike, NaiveDate};
use serde::Serialize;

use crate::dates::{add_months_clamped, prev_day};
use crate::occurrence::{occurrences, scheduled};
use crate::store::Store;
use crate::{Charge, ChargeStatus, Period, Result, SubscriptionStatus};

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MonthSpend {
    /// "YYYY-MM".
    pub month: String,
    pub total: i64,
    /// Per category id (None = no category), largest first.
    pub by_category: Vec<Slice>,
    /// The month is still to come (or still running): charges not all paid.
    pub forecast: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Slice {
    /// Category or payment method id, or currency code; None = not set.
    pub key: Option<String>,
    pub amount: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Stats {
    pub main_currency: String,
    /// From the first month with a charge (at most `months` back) to the
    /// end of next month.
    pub months: Vec<MonthSpend>,
    /// The running payments' cost per year, and its splits (largest first).
    pub yearly: i64,
    pub by_category: Vec<Slice>,
    pub by_method: Vec<Slice>,
    pub by_currency: Vec<Slice>,
    /// What paused stretches and cancelled payments would have cost until
    /// today at their last price.
    pub saved: i64,
    /// Currencies left out for lack of a rate.
    pub excluded_currencies: Vec<String>,
    /// First day a rate is needed (fetch the history's rates from it).
    pub from: Option<NaiveDate>,
    /// The monthly budget (Settings), in main: a line on the chart.
    pub monthly_budget: Option<i64>,
}

fn add(slices: &mut Vec<Slice>, key: Option<String>, amount: i64) {
    match slices.iter_mut().find(|s| s.key == key) {
        Some(s) => s.amount += amount,
        None => slices.push(Slice { key, amount }),
    }
}

fn sorted(mut v: Vec<Slice>) -> Vec<Slice> {
    v.sort_by(|a, b| b.amount.cmp(&a.amount).then_with(|| a.key.cmp(&b.key)));
    v
}

fn month_key(d: NaiveDate) -> String {
    format!("{:04}-{:02}", d.year(), d.month())
}

/// Charges a stretch without charges would have had at `p`'s terms: from
/// the day after `p` ends to `until`.
fn would_have(p: &Period, until: NaiveDate, today: NaiveDate) -> Vec<Charge> {
    let Some(end) = p.end_date else { return Vec::new() };
    let Some(from) = end.succ_opt() else { return Vec::new() };
    if from > until {
        return Vec::new();
    }
    let ghost = Period { start_date: from, end_date: Some(until), ..p.clone() };
    scheduled(&ghost, from, until)
        .into_iter()
        .map(|d| Charge {
            subscription_id: p.subscription_id.clone(),
            period_id: p.id.clone(),
            due_date: d,
            scheduled_date: d,
            amount_minor: p.terms.amount_minor,
            currency: p.terms.currency.clone(),
            settled_amount_minor: None,
            settled_currency: None,
            estimated: false,
            status: if d > today { ChargeStatus::Upcoming } else { ChargeStatus::Paid },
            is_override: false,
        })
        .collect()
}

impl Store {
    /// Stats for `today`, months going back at most `months` (0 = all).
    pub fn stats(&self, today: NaiveDate, months: u32) -> Result<Stats> {
        let main = self.main_currency()?;
        let subs = self.subscriptions()?;
        let this_month = today.with_day(1).unwrap_or(today);
        let end = add_months_clamped(this_month, 2).map(prev_day).unwrap_or(today);
        let first = subs.iter().flat_map(|s| s.periods.first()).map(|p| p.start_date).min();
        let earliest = if months == 0 {
            first
        } else {
            add_months_clamped(this_month, -(i64::from(months) - 1)).map(|m| first.map_or(m, |f| f.max(m)))
        };
        let mut excluded: Vec<String> = Vec::new();
        let mut month_list: Vec<MonthSpend> = Vec::new();
        if let Some(start) = earliest.map(|d| d.with_day(1).unwrap_or(d)) {
            let mut m = start;
            while m <= end {
                month_list.push(MonthSpend {
                    month: month_key(m),
                    total: 0,
                    by_category: Vec::new(),
                    forecast: m >= this_month,
                });
                m = match add_months_clamped(m, 1) {
                    Some(n) => n,
                    None => break,
                };
            }
            // History includes archived payments: it happened.
            for s in &subs {
                for c in occurrences(&s.periods, &s.overrides, start, end, today) {
                    let Some(v) = self.charge_in(&c, &main)? else {
                        excluded.push(c.currency.clone());
                        continue;
                    };
                    if let Some(ms) = month_list.iter_mut().find(|ms| ms.month == month_key(c.due_date)) {
                        ms.total += v;
                        add(&mut ms.by_category, s.subscription.category_id.clone(), v);
                    }
                }
            }
            for ms in &mut month_list {
                ms.by_category = sorted(std::mem::take(&mut ms.by_category));
            }
        }

        // Running payments, per year.
        let (mut yearly, mut by_category, mut by_method, mut by_currency) = (0, Vec::new(), Vec::new(), Vec::new());
        let mut saved = 0;
        for s in subs.iter().filter(|s| s.subscription.archived_at.is_none()) {
            let v = self.view(s, today, &main)?;
            let running = matches!(v.status, SubscriptionStatus::Active | SubscriptionStatus::Trial);
            if let Some(p) = v.current.as_ref().filter(|p| running && p.charges()) {
                match v.yearly_in_main {
                    Some(y) => {
                        yearly += y;
                        add(&mut by_category, s.subscription.category_id.clone(), y);
                        add(&mut by_method, s.subscription.payment_method_id.clone(), y);
                        add(&mut by_currency, Some(p.terms.currency.clone()), y);
                    }
                    None => excluded.push(p.terms.currency.clone()),
                }
            }
        }
        // Saved: every stretch after a charging period without charges (a
        // pause, a gap, the time since cancelling), up to today.
        for s in &subs {
            let charging: Vec<&Period> = s.periods.iter().filter(|p| p.charges()).collect();
            for (i, p) in charging.iter().enumerate() {
                let next_start = charging.get(i + 1).map(|n| prev_day(n.start_date));
                let until = next_start.map_or(today, |d| d.min(today));
                for c in would_have(p, until, today) {
                    match self.charge_in(&c, &main)? {
                        Some(v) => saved += v,
                        None => excluded.push(c.currency.clone()),
                    }
                }
            }
        }
        excluded.sort();
        excluded.dedup();
        let from = earliest.filter(|d| *d <= today);
        Ok(Stats {
            main_currency: main,
            months: month_list,
            yearly,
            by_category: sorted(by_category),
            by_method: sorted(by_method),
            by_currency: sorted(by_currency),
            saved,
            excluded_currencies: excluded,
            from,
            monthly_budget: self.settings()?.monthly_budget,
        })
    }
}
