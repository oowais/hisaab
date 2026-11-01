mod common;
use common::*;
use hisaab_core::occurrence::{next_charge, occurrences};
use hisaab_core::*;

fn run(ps: &[Period], from: &str, to: &str) -> Vec<String> {
    dates(&occurrences(ps, &[], d(from), d(to), d("2000-01-01")))
}

#[test]
fn month_end_clamps_without_drift() {
    let p = active("p", monthly(999), "2025-01-31", "2025-01-31", None);
    assert_eq!(
        run(&[p], "2025-01-01", "2025-06-30"),
        ["2025-01-31", "2025-02-28", "2025-03-31", "2025-04-30", "2025-05-31", "2025-06-30"]
    );
}

#[test]
fn leap_years() {
    let p = active("p", monthly(100), "2024-01-31", "2024-01-31", None);
    assert_eq!(run(&[p], "2024-02-01", "2024-03-31"), ["2024-02-29", "2024-03-31"]);
    let y = active("y", terms(100, "EUR", 1, Unit::Year), "2024-02-29", "2024-02-29", None);
    assert_eq!(run(&[y], "2024-01-01", "2028-12-31"), ["2024-02-29", "2025-02-28", "2026-02-28", "2027-02-28", "2028-02-29"]);
}

#[test]
fn weekly_daily_and_every_3_months() {
    let w = active("w", terms(100, "EUR", 1, Unit::Week), "2026-01-05", "2026-01-05", None);
    assert_eq!(run(&[w], "2026-01-01", "2026-01-31"), ["2026-01-05", "2026-01-12", "2026-01-19", "2026-01-26"]);
    let w2 = active("w2", terms(100, "EUR", 2, Unit::Week), "2026-01-05", "2026-01-05", None);
    assert_eq!(run(&[w2], "2026-01-10", "2026-02-10"), ["2026-01-19", "2026-02-02"]);
    let dd = active("dd", terms(100, "EUR", 10, Unit::Day), "2026-01-01", "2026-01-01", None);
    assert_eq!(run(&[dd], "2026-01-01", "2026-01-31"), ["2026-01-01", "2026-01-11", "2026-01-21", "2026-01-31"]);
    let q = active("q", terms(3000, "EUR", 3, Unit::Month), "2025-11-30", "2025-11-30", None);
    assert_eq!(run(&[q], "2025-11-01", "2026-12-31"), ["2025-11-30", "2026-02-28", "2026-05-30", "2026-08-30", "2026-11-30"]);
}

#[test]
fn far_from_anchor_starts_at_the_right_date() {
    let p = active("p", monthly(100), "2001-03-31", "2001-03-31", None);
    assert_eq!(run(&[p], "2026-02-01", "2026-04-30"), ["2026-02-28", "2026-03-31", "2026-04-30"]);
}

#[test]
fn respects_start_end_and_anchor_after_start() {
    let p = active("p", monthly(100), "2026-01-15", "2026-02-01", Some("2026-04-14"));
    assert_eq!(run(&[p], "2026-01-01", "2026-12-31"), ["2026-02-15", "2026-03-15"]);
    let trial_then_paid = active("t", monthly(100), "2026-01-20", "2026-01-01", None);
    assert_eq!(run(&[trial_then_paid], "2026-01-01", "2026-02-28"), ["2026-01-20", "2026-02-20"]);
}

#[test]
fn pause_in_the_middle_skips_its_charges() {
    let ps = [
        active("a", monthly(100), "2026-01-10", "2026-01-10", Some("2026-03-04")),
        period("p", PeriodStatus::Paused, monthly(100), "2026-01-10", "2026-03-05", Some("2026-05-31")),
        active("b", monthly(100), "2026-06-01", "2026-06-01", None),
    ];
    assert_eq!(run(&ps, "2026-01-01", "2026-07-31"), ["2026-01-10", "2026-02-10", "2026-06-01", "2026-07-01"]);
}

#[test]
fn ranges_crossing_period_boundaries_use_each_periods_terms() {
    let ps = [
        active("old", monthly(1399), "2026-01-05", "2026-01-05", Some("2026-03-04")),
        active("new", monthly(499), "2026-01-05", "2026-03-05", None),
    ];
    let cs = occurrences(&ps, &[], d("2026-02-01"), d("2026-04-30"), d("2026-03-10"));
    let got: Vec<_> = cs.iter().map(|c| (c.due_date.to_string(), c.amount_minor, c.period_id.as_str(), c.status)).collect();
    assert_eq!(
        got,
        [
            ("2026-02-05".to_string(), 1399, "old", ChargeStatus::Paid),
            ("2026-03-05".to_string(), 499, "new", ChargeStatus::Paid),
            ("2026-04-05".to_string(), 499, "new", ChargeStatus::Upcoming),
        ]
    );
}

#[test]
fn free_trial_has_no_charges_paid_trial_does() {
    let free = period("f", PeriodStatus::Trial, monthly(0), "2026-01-01", "2026-01-01", Some("2026-01-31"));
    assert!(run(&[free], "2026-01-01", "2026-01-31").is_empty());
    let paid = period("t", PeriodStatus::Trial, monthly(100), "2026-01-01", "2026-01-01", Some("2026-01-31"));
    assert_eq!(run(&[paid], "2026-01-01", "2026-01-31"), ["2026-01-01"]);
}

fn ov(kind: OverrideKind, due: &str) -> ChargeOverride {
    ChargeOverride {
        id: format!("o-{due}"),
        period_id: "p".into(),
        due_date: d(due),
        kind,
        new_due_date: None,
        actual_amount_minor: None,
        actual_currency: None,
        settled_amount_minor: None,
        settled_currency: None,
    }
}

#[test]
fn overrides_skip_move_amount_and_settled() {
    let p = active("p", monthly(1000), "2026-01-10", "2026-01-10", None);
    let skip = ov(OverrideKind::Skipped, "2026-02-10");
    let moved = ChargeOverride { new_due_date: Some(d("2026-03-14")), ..ov(OverrideKind::Moved, "2026-03-10") };
    let amount = ChargeOverride {
        actual_amount_minor: Some(1250),
        actual_currency: Some("USD".into()),
        ..ov(OverrideKind::AmountChanged, "2026-04-10")
    };
    let settled = ChargeOverride {
        settled_amount_minor: Some(1003),
        settled_currency: Some("EUR".into()),
        ..ov(OverrideKind::Settled, "2026-05-10")
    };
    let cs = occurrences(&[p], &[skip, moved, amount, settled], d("2026-01-01"), d("2026-05-31"), d("2026-12-31"));
    assert_eq!(dates(&cs), ["2026-01-10", "2026-03-14", "2026-04-10", "2026-05-10"]);
    assert_eq!(cs[1].scheduled_date, d("2026-03-10"));
    assert!(cs[1].is_override && !cs[0].is_override);
    assert_eq!((cs[2].amount_minor, cs[2].currency.as_str()), (1250, "USD"));
    assert_eq!((cs[3].amount_minor, cs[3].settled_amount_minor), (1000, Some(1003)));
}

#[test]
fn a_moved_charge_keeps_its_date_when_its_amount_is_set() {
    let p = active("p", monthly(1000), "2026-01-31", "2026-01-31", None);
    let paid = ChargeOverride {
        new_due_date: Some(d("2026-02-02")),
        settled_amount_minor: Some(990),
        settled_currency: Some("EUR".into()),
        ..ov(OverrideKind::Settled, "2026-01-31")
    };
    let cs = occurrences(&[p], &[paid], d("2026-02-01"), d("2026-02-28"), d("2026-12-31"));
    assert_eq!(dates(&cs), ["2026-02-02", "2026-02-28"]);
    assert_eq!(cs[0].settled_amount_minor, Some(990));
}

#[test]
fn moved_across_the_range_edge() {
    let p = active("p", monthly(1000), "2026-01-31", "2026-01-31", None);
    let into = ChargeOverride { new_due_date: Some(d("2026-02-02")), ..ov(OverrideKind::Moved, "2026-01-31") };
    let out = ChargeOverride { new_due_date: Some(d("2026-03-02")), ..ov(OverrideKind::Moved, "2026-02-28") };
    let cs = occurrences(&[p], &[into, out], d("2026-02-01"), d("2026-02-28"), d("2026-01-01"));
    assert_eq!(dates(&cs), ["2026-02-02"]);
}

#[test]
fn status_follows_today_and_next_charge() {
    let p = active("p", monthly(100), "2026-01-10", "2026-01-10", None);
    let cs = occurrences(std::slice::from_ref(&p), &[], d("2026-02-01"), d("2026-03-31"), d("2026-02-10"));
    assert_eq!(cs.iter().map(|c| c.status).collect::<Vec<_>>(), [ChargeStatus::Paid, ChargeStatus::Upcoming]);
    assert_eq!(next_charge(&[p], &[], d("2026-02-11")).unwrap().due_date, d("2026-03-10"));
}
