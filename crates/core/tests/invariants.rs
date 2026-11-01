mod common;
use common::*;
use hisaab_core::invariants::{status_on, validate_periods};
use hisaab_core::*;

#[test]
fn accepts_back_to_back_and_gaps() {
    let ps = [
        active("a", monthly(100), "2026-01-01", "2026-01-01", Some("2026-01-31")),
        active("b", monthly(200), "2026-01-01", "2026-02-01", Some("2026-03-31")),
        active("c", monthly(200), "2026-06-01", "2026-06-01", None),
    ];
    validate_periods(&ps).unwrap();
}

#[test]
fn rejects_overlap_open_ended_in_the_middle_and_bad_terms() {
    let overlap = [
        active("a", monthly(100), "2026-01-01", "2026-01-01", Some("2026-02-01")),
        active("b", monthly(100), "2026-02-01", "2026-02-01", None),
    ];
    assert!(matches!(validate_periods(&overlap), Err(Error::Invalid(m)) if m.contains("overlap")));
    let open_first = [
        active("a", monthly(100), "2026-01-01", "2026-01-01", None),
        active("b", monthly(100), "2026-05-01", "2026-05-01", None),
    ];
    assert!(validate_periods(&open_first).is_err());
    assert!(validate_periods(&[active("a", monthly(100), "2026-01-01", "2026-02-01", Some("2026-01-31"))]).is_err());
    assert!(validate_periods(&[active("a", terms(100, "EUR", 0, Unit::Month), "2026-01-01", "2026-01-01", None)]).is_err());
    assert!(validate_periods(&[active("a", terms(-1, "EUR", 1, Unit::Month), "2026-01-01", "2026-01-01", None)]).is_err());
    assert!(validate_periods(&[active("a", terms(1, "EURO", 1, Unit::Month), "2026-01-01", "2026-01-01", None)]).is_err());
}

#[test]
fn status_on_a_day() {
    let ps = [
        period("t", PeriodStatus::Trial, monthly(0), "2026-01-01", "2026-01-01", Some("2026-01-31")),
        active("a", monthly(100), "2026-02-01", "2026-02-01", Some("2026-03-31")),
        period("p", PeriodStatus::Paused, monthly(100), "2026-02-01", "2026-04-01", Some("2026-04-30")),
    ];
    assert_eq!(status_on(&ps, d("2025-12-01")), SubscriptionStatus::Trial);
    assert_eq!(status_on(&ps, d("2026-01-15")), SubscriptionStatus::Trial);
    assert_eq!(status_on(&ps, d("2026-02-15")), SubscriptionStatus::Active);
    assert_eq!(status_on(&ps, d("2026-04-15")), SubscriptionStatus::Paused);
    assert_eq!(status_on(&ps, d("2026-05-01")), SubscriptionStatus::Ended);
    assert_eq!(status_on(&[], d("2026-05-01")), SubscriptionStatus::Ended);
}
