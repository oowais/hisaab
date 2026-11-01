mod common;
use common::*;
use hisaab_core::invariants::status_on;
use hisaab_core::*;

fn store() -> Store {
    Store::open_in_memory().unwrap().with_clock(|| 1_000)
}

fn add(s: &mut Store, name: &str, t: Terms, start: &str, trial_end: Option<&str>) -> SubscriptionDetail {
    s.add_subscription(&NewSubscription {
        fields: SubscriptionFields { name: name.into(), ..Default::default() },
        terms: t,
        start_date: d(start),
        anchor_date: None,
        trial_end: trial_end.map(d),
    })
    .unwrap()
}

/// (status, amount, start, end, anchor) per period.
fn shape(sd: &SubscriptionDetail) -> Vec<(PeriodStatus, i64, String, Option<String>, String)> {
    sd.periods
        .iter()
        .map(|p| {
            (p.status, p.terms.amount_minor, p.start_date.to_string(), p.end_date.map(|e| e.to_string()), p.anchor_date.to_string())
        })
        .collect()
}

fn row(st: PeriodStatus, amount: i64, start: &str, end: Option<&str>, anchor: &str) -> (PeriodStatus, i64, String, Option<String>, String) {
    (st, amount, start.into(), end.map(Into::into), anchor.into())
}

use PeriodStatus::{Active, Paused, Trial};

#[test]
fn add_creates_one_open_period() {
    let mut s = store();
    let sub = add(&mut s, "Netflix", monthly(1399), "2026-01-05", None);
    assert_eq!(sub.subscription.kind, Kind::Subscription);
    assert_eq!(shape(&sub), [row(Active, 1399, "2026-01-05", None, "2026-01-05")]);
    assert_eq!(s.subscriptions().unwrap().len(), 1);
}

#[test]
fn add_rejects_empty_name_and_bad_currency() {
    let mut s = store();
    let mut new = NewSubscription {
        fields: SubscriptionFields { name: " ".into(), ..Default::default() },
        terms: monthly(100),
        start_date: d("2026-01-01"),
        anchor_date: None,
        trial_end: None,
    };
    assert!(matches!(s.add_subscription(&new), Err(Error::Invalid(m)) if m == "name is empty"));
    new.fields.name = "X".into();
    new.terms.currency = "XYZ".into();
    assert!(s.add_subscription(&new).is_err());
}

#[test]
fn trial_then_paid_and_convert_early() {
    let mut s = store();
    let sub = add(&mut s, "Claude", monthly(2000), "2026-01-01", Some("2026-01-14"));
    assert_eq!(
        shape(&sub),
        [row(Trial, 0, "2026-01-01", Some("2026-01-14"), "2026-01-01"), row(Active, 2000, "2026-01-15", None, "2026-01-15")]
    );
    let sub = s.convert_trial(&sub.subscription.id, d("2026-01-10"), None).unwrap();
    assert_eq!(
        shape(&sub),
        [row(Trial, 0, "2026-01-01", Some("2026-01-09"), "2026-01-01"), row(Active, 2000, "2026-01-10", None, "2026-01-10")]
    );
}

#[test]
fn change_price_from_next_charge_keeps_anchor_and_history() {
    let mut s = store();
    let id = add(&mut s, "Netflix", monthly(1399), "2025-03-05", None).subscription.id;
    let from = s.next_charge(&id, d("2026-01-20")).unwrap().unwrap().due_date;
    assert_eq!(from, d("2026-02-05"));
    let sub = s.change_terms(&id, from, &monthly(499)).unwrap();
    assert_eq!(
        shape(&sub),
        [row(Active, 1399, "2025-03-05", Some("2026-02-04"), "2025-03-05"), row(Active, 499, "2026-02-05", None, "2025-03-05")]
    );
    let cs = s.charges(d("2026-01-01"), d("2026-03-31"), d("2026-01-20")).unwrap();
    assert_eq!(cs.iter().map(|c| c.amount_minor).collect::<Vec<_>>(), [1399, 499, 499]);
}

#[test]
fn future_price_rise_is_a_future_period() {
    let mut s = store();
    let id = add(&mut s, "Spotify", monthly(1099), "2026-01-01", None).subscription.id;
    let sub = s.change_terms(&id, d("2026-07-01"), &monthly(1199)).unwrap();
    assert_eq!(status_on(&sub.periods, d("2026-03-01")), SubscriptionStatus::Active);
    assert_eq!(sub.periods.len(), 2);
}

#[test]
fn pause_with_resume_date_then_cancel_and_restart() {
    let mut s = store();
    let id = add(&mut s, "Gym", monthly(3000), "2026-01-01", None).subscription.id;
    let sub = s.pause(&id, d("2026-03-01"), Some(d("2026-05-15"))).unwrap();
    assert_eq!(
        shape(&sub),
        [
            row(Active, 3000, "2026-01-01", Some("2026-02-28"), "2026-01-01"),
            row(Paused, 3000, "2026-03-01", Some("2026-05-14"), "2026-01-01"),
            row(Active, 3000, "2026-05-15", None, "2026-05-15"),
        ]
    );
    assert_eq!(status_on(&sub.periods, d("2026-04-01")), SubscriptionStatus::Paused);
    let cs = s.charges(d("2026-01-01"), d("2026-06-30"), d("2026-01-01")).unwrap();
    assert_eq!(dates(&cs), ["2026-01-01", "2026-02-01", "2026-05-15", "2026-06-15"]);

    // Cancel at the end of the current cycle.
    let end = s.cycle_end(&id, d("2026-06-20")).unwrap();
    assert_eq!(end, d("2026-07-14"));
    let sub = s.cancel(&id, end).unwrap();
    assert_eq!(status_on(&sub.periods, d("2026-08-01")), SubscriptionStatus::Ended);

    assert!(s.restart(&id, d("2026-07-01"), None).is_err());
    let sub = s.restart(&id, d("2026-10-03"), Some(&monthly(3500))).unwrap();
    assert_eq!(sub.periods.last().map(|p| (p.terms.amount_minor, p.anchor_date)), Some((3500, d("2026-10-03"))));
    assert_eq!(status_on(&sub.periods, d("2026-10-03")), SubscriptionStatus::Active);
}

#[test]
fn open_pause_then_resume() {
    let mut s = store();
    let id = add(&mut s, "Gym", monthly(3000), "2026-01-10", None).subscription.id;
    s.change_terms(&id, d("2026-09-10"), &monthly(3200)).unwrap();
    let sub = s.pause(&id, d("2026-03-01"), None).unwrap();
    // The scheduled price rise went with the open-ended pause.
    assert_eq!(sub.periods.len(), 2);
    assert!(s.change_terms(&id, d("2026-04-01"), &monthly(1)).is_err());
    let sub = s.resume(&id, d("2026-04-20"), None).unwrap();
    assert_eq!(
        shape(&sub)[1..],
        [row(Paused, 3000, "2026-03-01", Some("2026-04-19"), "2026-01-10"), row(Active, 3000, "2026-04-20", None, "2026-04-20")]
    );
    assert!(s.resume(&id, d("2026-05-01"), None).is_err());
}

#[test]
fn cancel_drops_later_periods_and_overrides_follow_splits() {
    let mut s = store();
    let sub = add(&mut s, "Phone", monthly(1500), "2026-01-20", None);
    let id = sub.subscription.id.clone();
    let p0 = sub.periods[0].id.clone();
    let o = s
        .set_override(&NewOverride {
            period_id: p0.clone(),
            due_date: d("2026-05-20"),
            kind: OverrideKind::Skipped,
            new_due_date: None,
            actual_amount_minor: None,
            actual_currency: None,
            settled_amount_minor: None,
            settled_currency: None,
        })
        .unwrap();
    // Price change splits the period: the May override moves to the new one.
    let sub = s.change_terms(&id, d("2026-04-20"), &monthly(1700)).unwrap();
    assert_eq!(sub.overrides.len(), 1);
    assert_eq!(sub.overrides[0].id, o.id);
    assert_eq!(sub.overrides[0].period_id, sub.periods[1].id);
    let cs = s.charges(d("2026-04-01"), d("2026-06-30"), d("2026-01-01")).unwrap();
    assert_eq!(dates(&cs), ["2026-04-20", "2026-06-20"]);

    s.change_terms(&id, d("2026-09-20"), &monthly(1900)).unwrap();
    let sub = s.cancel(&id, d("2026-04-30")).unwrap();
    assert_eq!(sub.periods.len(), 2);
    assert_eq!(sub.periods[1].end_date, Some(d("2026-04-30")));
    assert!(sub.overrides.is_empty(), "the May charge no longer exists");
}

#[test]
fn overrides_validate_and_replace() {
    let mut s = store();
    let sub = add(&mut s, "Rent", terms(120000, "EUR", 1, Unit::Month), "2026-01-01", None);
    let p = sub.periods[0].id.clone();
    let base = NewOverride {
        period_id: p.clone(),
        due_date: d("2026-02-01"),
        kind: OverrideKind::Moved,
        new_due_date: None,
        actual_amount_minor: None,
        actual_currency: None,
        settled_amount_minor: None,
        settled_currency: None,
    };
    assert!(s.set_override(&base).is_err(), "moved needs a date");
    assert!(s.set_override(&NewOverride { due_date: d("2026-02-02"), kind: OverrideKind::Skipped, ..base.clone() }).is_err());
    let a = s.set_override(&NewOverride { new_due_date: Some(d("2026-02-03")), ..base.clone() }).unwrap();
    let b = s
        .set_override(&NewOverride { kind: OverrideKind::AmountChanged, actual_amount_minor: Some(125000), ..base.clone() })
        .unwrap();
    assert_eq!(a.id, b.id, "one override per charge");
    assert_eq!(s.subscription(&sub.subscription.id).unwrap().overrides.len(), 1);
    s.delete_override(&b.id).unwrap();
    assert!(s.subscription(&sub.subscription.id).unwrap().overrides.is_empty());
}

#[test]
fn archive_hides_charges_delete_and_restore() {
    let mut s = store();
    let id = add(&mut s, "Old", monthly(100), "2026-01-01", None).subscription.id;
    s.set_archived(&id, true).unwrap();
    assert!(s.charges(d("2026-01-01"), d("2026-12-31"), d("2026-01-01")).unwrap().is_empty());
    s.set_archived(&id, false).unwrap();
    s.delete_subscription(&id).unwrap();
    assert!(s.subscriptions().unwrap().is_empty());
    assert!(matches!(s.subscription(&id), Err(Error::NotFound(_))));
    s.restore_subscription(&id).unwrap();
    assert_eq!(s.subscription(&id).unwrap().periods.len(), 1);
}

#[test]
fn categories_and_payment_methods() {
    let mut s = store();
    let streaming = s.add_category("Streaming", "#e50914").unwrap();
    let home = s.add_category("Home", "").unwrap();
    assert_eq!((streaming.sort, home.sort), (0, 1));
    let visa = s.add_payment_method("Visa ..4417").unwrap();
    let sub = s
        .add_subscription(&NewSubscription {
            fields: SubscriptionFields {
                kind: Some(Kind::Bill),
                name: "Rundfunkbeitrag".into(),
                category_id: Some(home.id.clone()),
                payment_method_id: Some(visa.id.clone()),
                ..Default::default()
            },
            terms: terms(5508, "EUR", 3, Unit::Month),
            start_date: d("2026-01-15"),
            anchor_date: None,
            trial_end: None,
        })
        .unwrap();
    assert_eq!(sub.subscription.kind, Kind::Bill);
    s.delete_category(&home.id).unwrap();
    s.delete_payment_method(&visa.id).unwrap();
    let sub = s.subscription(&sub.subscription.id).unwrap();
    assert_eq!((sub.subscription.category_id, sub.subscription.payment_method_id), (None, None));
    assert_eq!(s.categories().unwrap(), [streaming]);
    assert!(s.payment_methods().unwrap().is_empty());
}
