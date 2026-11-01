//! The occurrence engine: periods + overrides → charges. Pure; charges are
//! never stored.
//!
//! Per non-paused period: dates step from `anchor_date` by `every_n unit`,
//! kept within `[start_date, end_date]`. Each date is `anchor + k·interval`
//! (see `dates::nth`), so month-end anchors clamp per month without drifting.
//! Then overrides apply (skip, move, other amount, settled amount).

use std::collections::HashMap;

use chrono::NaiveDate;

use crate::dates::{first_k_near, nth};
use crate::model::{Charge, ChargeOverride, ChargeStatus, OverrideKind, Period};

/// Scheduled dates of `period` within `[from, to]` (before overrides).
pub fn scheduled(period: &Period, from: NaiveDate, to: NaiveDate) -> Vec<NaiveDate> {
    let lo = from.max(period.start_date);
    let hi = period.end_date.map_or(to, |e| e.min(to));
    let mut out = Vec::new();
    if !period.charges() || lo > hi {
        return out;
    }
    let (anchor, n, unit) = (period.anchor_date, period.terms.every_n.max(1), period.terms.unit);
    let mut k = first_k_near(anchor, n, unit, lo);
    while let Some(d) = nth(anchor, n, unit, k) {
        if d > hi {
            break;
        }
        if d >= lo {
            out.push(d);
        }
        k += 1;
    }
    out
}

/// Whether the rule of `period` puts a charge on `day`.
pub fn is_scheduled(period: &Period, day: NaiveDate) -> bool {
    !scheduled(period, day, day).is_empty()
}

/// Charges of `periods` due in `[from, to]`, by date. `today` decides
/// paid (due on or before today) vs upcoming.
pub fn occurrences(
    periods: &[Period],
    overrides: &[ChargeOverride],
    from: NaiveDate,
    to: NaiveDate,
    today: NaiveDate,
) -> Vec<Charge> {
    let by_date: HashMap<(&str, NaiveDate), &ChargeOverride> =
        overrides.iter().map(|o| ((o.period_id.as_str(), o.due_date), o)).collect();
    let mut out = Vec::new();
    for p in periods {
        for d in scheduled(p, from, to) {
            match by_date.get(&(p.id.as_str(), d)) {
                None => out.push(charge(p, d, d, today, None)),
                Some(o) => {
                    if let Some(c) = apply(p, d, o, today) {
                        if (from..=to).contains(&c.due_date) {
                            out.push(c);
                        }
                    }
                }
            }
        }
        // Moved into the range from a date outside it.
        for o in overrides.iter().filter(|o| o.period_id == p.id && o.kind != OverrideKind::Skipped) {
            let inside = |d: NaiveDate| (from..=to).contains(&d);
            if !inside(o.due_date) && o.new_due_date.is_some_and(inside) && is_scheduled(p, o.due_date) {
                out.extend(apply(p, o.due_date, o, today));
            }
        }
    }
    out.sort_by(|a, b| {
        (a.due_date, &a.subscription_id, &a.period_id, a.scheduled_date).cmp(&(
            b.due_date,
            &b.subscription_id,
            &b.period_id,
            b.scheduled_date,
        ))
    });
    out
}

/// The next charge due on or after `today` (searches 10 years ahead).
pub fn next_charge(periods: &[Period], overrides: &[ChargeOverride], today: NaiveDate) -> Option<Charge> {
    let to = crate::dates::add_months_clamped(today, 120)?;
    occurrences(periods, overrides, today, to, today).into_iter().next()
}

fn charge(p: &Period, due: NaiveDate, scheduled: NaiveDate, today: NaiveDate, o: Option<&ChargeOverride>) -> Charge {
    Charge {
        subscription_id: p.subscription_id.clone(),
        period_id: p.id.clone(),
        due_date: due,
        scheduled_date: scheduled,
        amount_minor: p.terms.amount_minor,
        currency: p.terms.currency.clone(),
        settled_amount_minor: o.and_then(|o| o.settled_amount_minor),
        settled_currency: o.and_then(|o| o.settled_currency.clone()),
        estimated: p.terms.amount_kind == crate::model::AmountKind::Estimated,
        status: if due > today { ChargeStatus::Upcoming } else { ChargeStatus::Paid },
        is_override: o.is_some(),
    }
}

/// A moved charge keeps its new date when its amount is set too (any kind
/// but skipped may carry `new_due_date`).
fn apply(p: &Period, scheduled: NaiveDate, o: &ChargeOverride, today: NaiveDate) -> Option<Charge> {
    let due = o.new_due_date.unwrap_or(scheduled);
    match o.kind {
        OverrideKind::Skipped => None,
        OverrideKind::Moved => o.new_due_date.map(|d| charge(p, d, scheduled, today, Some(o))),
        OverrideKind::AmountChanged => {
            let mut c = charge(p, due, scheduled, today, Some(o));
            if let Some(a) = o.actual_amount_minor {
                c.amount_minor = a;
            }
            if let Some(cur) = &o.actual_currency {
                c.currency = cur.clone();
            }
            c.estimated = false;
            Some(c)
        }
        OverrideKind::Settled => Some(charge(p, due, scheduled, today, Some(o))),
    }
}
