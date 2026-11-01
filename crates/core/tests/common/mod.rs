#![allow(dead_code)]
use chrono::NaiveDate;
use hisaab_core::*;

pub fn d(s: &str) -> NaiveDate {
    NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap()
}

pub fn terms(amount_minor: i64, currency: &str, every_n: u32, unit: Unit) -> Terms {
    Terms { amount_minor, currency: currency.into(), every_n, unit, plan_label: None, amount_kind: AmountKind::Fixed }
}

pub fn monthly(amount_minor: i64) -> Terms {
    terms(amount_minor, "EUR", 1, Unit::Month)
}

pub fn period(id: &str, status: PeriodStatus, t: Terms, anchor: &str, start: &str, end: Option<&str>) -> Period {
    Period {
        id: id.into(),
        subscription_id: "s".into(),
        status,
        terms: t,
        anchor_date: d(anchor),
        start_date: d(start),
        end_date: end.map(d),
    }
}

pub fn active(id: &str, t: Terms, anchor: &str, start: &str, end: Option<&str>) -> Period {
    period(id, PeriodStatus::Active, t, anchor, start, end)
}

pub fn dates(cs: &[Charge]) -> Vec<String> {
    cs.iter().map(|c| c.due_date.to_string()).collect()
}
