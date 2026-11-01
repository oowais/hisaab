//! What the screens show, computed in one place: per subscription its
//! status, current terms and next charge; the upcoming charges; the cost
//! per month / year / day in the main currency; a subscription's history.
//! Amounts "in main" are None when a rate is missing (shown in the original
//! currency only and left out of totals).

use chrono::NaiveDate;
use serde::Serialize;

use crate::dates::{add_months_clamped, prev_day};
use crate::invariants::status_on;
use crate::model::*;
use crate::occurrence::{next_charge, occurrences};
use crate::store::Store;
use crate::Result;

/// Charges per year of a schedule.
pub fn per_year(every_n: u32, unit: Unit) -> f64 {
    let n = f64::from(every_n.max(1));
    match unit {
        Unit::Day => 365.2425 / n,
        Unit::Week => 365.2425 / 7.0 / n,
        Unit::Month => 12.0 / n,
        Unit::Year => 1.0 / n,
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SubscriptionView {
    #[serde(flatten)]
    pub subscription: Subscription,
    pub status: SubscriptionStatus,
    /// The period covering today, else the next scheduled one, else the last.
    pub current: Option<Period>,
    pub next_charge: Option<Charge>,
    pub next_in_main: Option<i64>,
    /// Cost per year of the current terms, in main (None: no rate, or not
    /// paying now: paused, ended, free trial).
    pub yearly_in_main: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpcomingCharge {
    #[serde(flatten)]
    pub charge: Charge,
    pub in_main: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Overview {
    pub main_currency: String,
    /// Not archived, by name.
    pub subscriptions: Vec<SubscriptionView>,
    /// Today through the end of next month.
    pub upcoming: Vec<UpcomingCharge>,
    pub yearly_total: i64,
    /// Currencies left out of the totals for lack of a rate.
    pub excluded_currencies: Vec<String>,
    pub rates_date: Option<NaiveDate>,
    pub rates_stale: bool,
    /// The monthly budget (Settings), in main.
    pub monthly_budget: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryCharge {
    #[serde(flatten)]
    pub charge: Charge,
    pub in_main: Option<i64>,
    /// The override that made it differ, to edit or undo.
    pub override_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Detail {
    #[serde(flatten)]
    pub view: SubscriptionView,
    pub periods: Vec<Period>,
    /// Paid charges, newest first.
    pub history: Vec<HistoryCharge>,
    /// Coming charges to the end of next month, soonest first (to edit one).
    pub coming: Vec<HistoryCharge>,
    pub total_spent: i64,
    /// History currencies with no rate (left out of `total_spent`).
    pub excluded_currencies: Vec<String>,
    /// First day a rate is needed for the history (None: no history).
    pub history_from: Option<NaiveDate>,
}

/// The override that made `c` differ, if any.
fn override_of(overrides: &[ChargeOverride], c: &Charge) -> Option<String> {
    overrides.iter().find(|o| o.period_id == c.period_id && o.due_date == c.scheduled_date).map(|o| o.id.clone())
}

/// Last day of the month after `today`'s.
fn end_of_next_month(today: NaiveDate) -> NaiveDate {
    let first = today.with_day0(0).unwrap_or(today);
    add_months_clamped(first, 2).and_then(|d| d.pred_opt()).unwrap_or(today)
}

use chrono::Datelike;

impl Store {
    pub fn main_currency(&self) -> Result<String> {
        Ok(self
            .setting("main_currency")?
            .and_then(|v| v.as_str().map(String::from))
            .filter(|c| crate::currency::is_currency_code(c))
            .unwrap_or_else(|| "EUR".into()))
    }

    pub(crate) fn view(&self, s: &SubscriptionDetail, today: NaiveDate, main: &str) -> Result<SubscriptionView> {
        let status = status_on(&s.periods, today);
        let current = s
            .periods
            .iter()
            .find(|p| p.covers(today))
            .or_else(|| s.periods.iter().filter(|p| p.start_date > today).min_by_key(|p| p.start_date))
            .or(s.periods.last())
            .cloned();
        let next = next_charge(&s.periods, &s.overrides, today);
        let next_in_main = match &next {
            Some(c) => self.charge_in(c, main)?,
            None => None,
        };
        let paying = matches!(status, SubscriptionStatus::Active | SubscriptionStatus::Trial);
        let yearly_in_main = match current.as_ref().filter(|p| paying && p.charges()) {
            Some(p) => {
                let yearly = (p.terms.amount_minor as f64 * per_year(p.terms.every_n, p.terms.unit)).round() as i64;
                self.amount_in(yearly, &p.terms.currency, main)?
            }
            None => None,
        };
        Ok(SubscriptionView {
            subscription: s.subscription.clone(),
            status,
            current,
            next_charge: next,
            next_in_main,
            yearly_in_main,
        })
    }

    pub fn overview(&self, today: NaiveDate) -> Result<Overview> {
        let main = self.main_currency()?;
        let subs: Vec<_> =
            self.subscriptions()?.into_iter().filter(|s| s.subscription.archived_at.is_none()).collect();
        let mut views = Vec::with_capacity(subs.len());
        let mut excluded: Vec<String> = Vec::new();
        let mut yearly_total = 0;
        for s in &subs {
            let v = self.view(s, today, &main)?;
            match (v.yearly_in_main, &v.current) {
                (Some(y), _) => yearly_total += y,
                (None, Some(p)) if p.charges() && matches!(v.status, SubscriptionStatus::Active | SubscriptionStatus::Trial) => {
                    excluded.push(p.terms.currency.clone())
                }
                _ => {}
            }
            views.push(v);
        }
        let mut upcoming = Vec::new();
        for c in self.charges(today, end_of_next_month(today), today)? {
            let in_main = self.charge_in(&c, &main)?;
            if in_main.is_none() {
                excluded.push(c.currency.clone());
            }
            upcoming.push(UpcomingCharge { charge: c, in_main });
        }
        excluded.sort();
        excluded.dedup();
        Ok(Overview {
            main_currency: main,
            subscriptions: views,
            upcoming,
            yearly_total,
            excluded_currencies: excluded,
            rates_date: self.latest_rates_date()?,
            rates_stale: self.rates_stale(today)?,
            monthly_budget: self.settings()?.monthly_budget,
        })
    }

    pub fn detail(&self, id: &str, today: NaiveDate) -> Result<Detail> {
        let main = self.main_currency()?;
        let s = self.subscription(id)?;
        let view = self.view(&s, today, &main)?;
        let history_from = s.periods.first().map(|p| p.start_date).filter(|d| *d <= today);
        let mut history = Vec::new();
        let mut total_spent = 0;
        let mut excluded: Vec<String> = Vec::new();
        if let Some(from) = history_from {
            for c in occurrences(&s.periods, &s.overrides, from, today, today).into_iter().rev() {
                let in_main = self.charge_in(&c, &main)?;
                match in_main {
                    Some(v) => total_spent += v,
                    None => excluded.push(c.currency.clone()),
                }
                let override_id = override_of(&s.overrides, &c);
                history.push(HistoryCharge { charge: c, in_main, override_id });
            }
        }
        excluded.sort();
        excluded.dedup();
        let mut coming = Vec::new();
        if let Some(from) = today.succ_opt() {
            for c in occurrences(&s.periods, &s.overrides, from, end_of_next_month(today), today) {
                let in_main = self.charge_in(&c, &main)?;
                let override_id = override_of(&s.overrides, &c);
                coming.push(HistoryCharge { charge: c, in_main, override_id });
            }
        }
        Ok(Detail { view, periods: s.periods, history, coming, total_spent, excluded_currencies: excluded, history_from })
    }
}

/// The Android home-screen widget: the monthly cost and the next charges.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Widget {
    pub main_currency: String,
    /// What the running payments cost a month, in main.
    pub monthly_total: i64,
    /// This device's day (for "today", "in 3 days").
    pub today: NaiveDate,
    pub charges: Vec<WidgetCharge>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WidgetCharge {
    pub subscription_id: String,
    pub name: String,
    pub due_date: NaiveDate,
    pub amount_minor: i64,
    pub currency: String,
    pub estimated: bool,
}

/// One calendar month's charges (the Calendar screen).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MonthCharges {
    /// By date.
    pub charges: Vec<UpcomingCharge>,
    /// In main; charges with no rate are left out.
    pub total: i64,
    pub excluded_currencies: Vec<String>,
}

impl Store {
    /// Charges of the month `day` is in (archived payments left out).
    pub fn month_charges(&self, day: NaiveDate, today: NaiveDate) -> Result<MonthCharges> {
        let main = self.main_currency()?;
        let first = day.with_day(1).unwrap_or(day);
        let last = add_months_clamped(first, 1).map(prev_day).unwrap_or(first);
        let (mut charges, mut total, mut excluded) = (Vec::new(), 0, Vec::new());
        for c in self.charges(first, last, today)? {
            let in_main = self.charge_in(&c, &main)?;
            match in_main {
                Some(v) => total += v,
                None => excluded.push(c.currency.clone()),
            }
            charges.push(UpcomingCharge { charge: c, in_main });
        }
        excluded.sort();
        excluded.dedup();
        Ok(MonthCharges { charges, total, excluded_currencies: excluded })
    }

    /// The widget's data at `now_ms` in `tz`: the next `limit` charges from today.
    pub fn widget(&self, now_ms: i64, tz: &str, limit: usize) -> Result<Widget> {
        let today = crate::reminders::local_day(now_ms, crate::reminders::parse_tz(tz));
        let o = self.overview(today)?;
        let names: std::collections::HashMap<&str, &str> =
            o.subscriptions.iter().map(|v| (v.subscription.id.as_str(), v.subscription.name.as_str())).collect();
        let charges = o
            .upcoming
            .iter()
            .take(limit)
            .map(|u| WidgetCharge {
                subscription_id: u.charge.subscription_id.clone(),
                name: names.get(u.charge.subscription_id.as_str()).map(|n| n.to_string()).unwrap_or_default(),
                due_date: u.charge.due_date,
                amount_minor: u.charge.amount_minor,
                currency: u.charge.currency.clone(),
                estimated: u.charge.estimated,
            })
            .collect();
        Ok(Widget { main_currency: o.main_currency, monthly_total: (o.yearly_total + 6) / 12, today, charges })
    }

    /// What `amount_minor` of `currency` every `every_n` `unit`s costs a
    /// month in the main currency, at the latest rates (Quick add's budget
    /// note). None without a rate.
    pub fn monthly_in_main(&self, amount_minor: i64, currency: &str, every_n: u32, unit: Unit) -> Result<Option<i64>> {
        let yearly = (amount_minor as f64 * per_year(every_n, unit)).round() as i64;
        Ok(self.amount_in(yearly, currency, &self.main_currency()?)?.map(|y| (y + 6) / 12))
    }

    /// Archived subscriptions, by name (the Archive screen).
    pub fn archived(&self, today: NaiveDate) -> Result<Vec<SubscriptionView>> {
        let main = self.main_currency()?;
        self.subscriptions()?
            .into_iter()
            .filter(|s| s.subscription.archived_at.is_some())
            .map(|s| self.view(&s, today, &main))
            .collect()
    }
}

/// One lane of the Timeline: a subscription and all its periods.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineLane {
    #[serde(flatten)]
    pub view: SubscriptionView,
    /// By start date.
    pub periods: Vec<Period>,
}

impl Store {
    /// Every subscription that isn't archived, with its periods (ended ones too:
    /// the screen decides whether to show them).
    pub fn timeline(&self, today: NaiveDate) -> Result<Vec<TimelineLane>> {
        let main = self.main_currency()?;
        self.subscriptions()?
            .into_iter()
            .filter(|s| s.subscription.archived_at.is_none())
            .map(|s| Ok(TimelineLane { view: self.view(&s, today, &main)?, periods: s.periods }))
            .collect()
    }
}
