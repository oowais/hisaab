//! Calendar-date arithmetic for charges. Dates are local calendar days
//! (`NaiveDate`, stored as `YYYY-MM-DD`): a charge is due on a day, not at
//! an instant.

use chrono::{Datelike, Days, NaiveDate};

use crate::model::Unit;

/// Days in `year`-`month` (1–12).
pub fn days_in_month(year: i32, month: u32) -> u32 {
    let (y, m) = if month == 12 { (year + 1, 1) } else { (year, month + 1) };
    NaiveDate::from_ymd_opt(y, m, 1).and_then(|d| d.pred_opt()).map_or(31, |d| d.day())
}

/// `anchor` moved by `months`, keeping the anchor's day of month and clamping
/// it to the month's length: Jan 31 + 1 → Feb 28/29, + 2 → Mar 31.
pub fn add_months_clamped(anchor: NaiveDate, months: i64) -> Option<NaiveDate> {
    let total = i64::from(anchor.year()) * 12 + i64::from(anchor.month0()) + months;
    let year = i32::try_from(total.div_euclid(12)).ok()?;
    let month = total.rem_euclid(12) as u32 + 1;
    let day = anchor.day().min(days_in_month(year, month));
    NaiveDate::from_ymd_opt(year, month, day)
}

/// The `k`-th charge date of a schedule (k = 0 is the anchor). Always computed
/// from the anchor, never from the previous date, so clamping doesn't drift.
pub fn nth(anchor: NaiveDate, every_n: u32, unit: Unit, k: u64) -> Option<NaiveDate> {
    let steps = k.checked_mul(u64::from(every_n))?;
    match unit {
        Unit::Day => anchor.checked_add_days(Days::new(steps)),
        Unit::Week => anchor.checked_add_days(Days::new(steps.checked_mul(7)?)),
        Unit::Month => add_months_clamped(anchor, i64::try_from(steps).ok()?),
        Unit::Year => add_months_clamped(anchor, i64::try_from(steps.checked_mul(12)?).ok()?),
    }
}

/// Smallest k whose date may be on or after `day` (a safe lower bound: the
/// caller still checks each date).
pub fn first_k_near(anchor: NaiveDate, every_n: u32, unit: Unit, day: NaiveDate) -> u64 {
    if day <= anchor {
        return 0;
    }
    let n = u64::from(every_n.max(1));
    let k = match unit {
        Unit::Day => (day - anchor).num_days() as u64 / n,
        Unit::Week => (day - anchor).num_days() as u64 / (7 * n),
        Unit::Month | Unit::Year => {
            let months = (i64::from(day.year()) - i64::from(anchor.year())) * 12
                + i64::from(day.month0())
                - i64::from(anchor.month0());
            let per = if unit == Unit::Year { 12 * n } else { n };
            months.max(0) as u64 / per
        }
    };
    k.saturating_sub(1)
}

pub fn prev_day(d: NaiveDate) -> NaiveDate {
    d.pred_opt().unwrap_or(d)
}

pub fn next_day(d: NaiveDate) -> NaiveDate {
    d.succ_opt().unwrap_or(d)
}
