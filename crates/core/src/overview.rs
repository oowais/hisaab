//! What the screens show, computed in one place: per subscription its
//! status, current terms and next charge; the upcoming charges; the cost
//! per month / year / day in the main currency; a subscription's history.
//! Amounts "in main" are None when a rate is missing (shown in the original
//! currency only and left out of totals).

use chrono::NaiveDate;
use serde::Serialize;

use crate::dates::add_months_clamped;
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

impl Store {
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
