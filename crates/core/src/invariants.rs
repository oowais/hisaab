//! Rules every subscription's periods follow, checked before each save:
//! - periods never overlap (gaps = not subscribed);
//! - at most one open-ended period, and it's the latest;
//! - `end_date >= start_date`;
//! - valid terms (interval ≥ 1, amount ≥ 0, ISO 4217 currency code).
//!
//! And the status shown for a subscription, derived from its periods.

use chrono::NaiveDate;

use crate::currency::is_currency_code;
use crate::model::{Period, PeriodStatus, SubscriptionStatus, Terms};
use crate::{Error, Result};

pub fn validate_terms(t: &Terms) -> Result<()> {
    if t.every_n == 0 {
        return Err(Error::Invalid("interval must be at least 1".into()));
    }
    if t.amount_minor < 0 {
        return Err(Error::Invalid("amount can't be negative".into()));
    }
    if !is_currency_code(&t.currency) {
        return Err(Error::Invalid(format!("unknown currency {:?}", t.currency)));
    }
    Ok(())
}

/// `periods` of one subscription, in any order.
pub fn validate_periods(periods: &[Period]) -> Result<()> {
    let mut sorted: Vec<&Period> = periods.iter().collect();
    sorted.sort_by_key(|p| p.start_date);
    for p in &sorted {
        validate_terms(&p.terms)?;
        if p.end_date.is_some_and(|e| e < p.start_date) {
            return Err(Error::Invalid("period ends before it starts".into()));
        }
    }
    for w in sorted.windows(2) {
        match w[0].end_date {
            None => return Err(Error::Invalid("only the latest period can be open-ended".into())),
            Some(end) if end >= w[1].start_date => return Err(Error::Invalid("periods overlap".into())),
            Some(_) => {}
        }
    }
    Ok(())
}

/// Status on `today`: the status of the period covering it; else, when one
/// is scheduled later, that one's; else Ended (no period, or all ended).
pub fn status_on(periods: &[Period], today: NaiveDate) -> SubscriptionStatus {
    let pick = periods.iter().find(|p| p.covers(today)).or_else(|| {
        periods.iter().filter(|p| p.start_date > today).min_by_key(|p| p.start_date)
    });
    match pick.map(|p| p.status) {
        Some(PeriodStatus::Trial) => SubscriptionStatus::Trial,
        Some(PeriodStatus::Active) => SubscriptionStatus::Active,
        Some(PeriodStatus::Paused) => SubscriptionStatus::Paused,
        None => SubscriptionStatus::Ended,
    }
}
