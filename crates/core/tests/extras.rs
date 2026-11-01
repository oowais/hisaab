//! One-time charges: besides the schedule, in charges, history and stats,
//! not in the yearly cost or reminders; synced.
mod common;
use common::*;
use hisaab_core::*;

fn add(s: &mut Store, name: &str) -> String {
    s.add_subscription(&NewSubscription {
        fields: SubscriptionFields { name: name.into(), ..Default::default() },
        terms: monthly(1000),
        start_date: d("2026-01-10"),
        anchor_date: None,
        trial_end: None,
    })
    .unwrap()
    .subscription
    .id
}

fn extra(s: &mut Store, sub: &str, day: &str, amount: i64, label: &str) -> ExtraCharge {
    s.add_extra(&NewExtraCharge {
        subscription_id: sub.into(),
        due_date: d(day),
        amount_minor: amount,
        currency: "EUR".into(),
        label: label.into(),
    })
    .unwrap()
}

#[test]
fn many_one_time_charges_join_the_charges() {
    let mut s = Store::open_in_memory().unwrap();
    let a = add(&mut s, "Netflix");
    extra(&mut s, &a, "2026-02-03", 499, "Extra member");
    extra(&mut s, &a, "2026-02-10", 250, "");
    extra(&mut s, &a, "2026-02-10", 100, "Late fee");
    let cs = s.charges(d("2026-02-01"), d("2026-02-28"), d("2026-02-05")).unwrap();
    let got: Vec<_> = cs.iter().map(|c| (c.due_date.to_string(), c.amount_minor, c.extra_id.is_some(), c.status)).collect();
    assert_eq!(
        got,
        [
            ("2026-02-03".into(), 499, true, ChargeStatus::Paid),
            ("2026-02-10".into(), 1000, false, ChargeStatus::Upcoming),
            ("2026-02-10".into(), 250, true, ChargeStatus::Upcoming),
            ("2026-02-10".into(), 100, true, ChargeStatus::Upcoming),
        ]
    );
    assert_eq!(cs[0].label.as_deref(), Some("Extra member"));
    assert_eq!(cs[2].label, None);
    // The regular next charge and the yearly cost ignore them.
    let o = s.overview(d("2026-02-05")).unwrap();
    assert_eq!(o.yearly_total, 12_000);
    assert_eq!(o.subscriptions[0].next_charge.as_ref().map(|c| c.amount_minor), Some(1000));
    assert_eq!(o.upcoming.iter().filter(|c| c.charge.extra_id.is_some()).count(), 2);
}

#[test]
fn history_total_spent_month_and_stats_count_them() {
    let mut s = Store::open_in_memory().unwrap();
    let a = add(&mut s, "Gym");
    // Before the first period too (a joining fee).
    extra(&mut s, &a, "2026-01-05", 2000, "Joining fee");
    extra(&mut s, &a, "2026-03-20", 500, "");
    let dt = s.detail(&a, d("2026-02-15")).unwrap();
    assert_eq!(dt.history_from, Some(d("2026-01-05")));
    assert_eq!(dt.total_spent, 2000 + 1000 + 1000);
    assert_eq!(dt.history.last().and_then(|h| h.charge.label.clone()).as_deref(), Some("Joining fee"));
    assert_eq!(dt.coming.iter().map(|h| h.charge.amount_minor).collect::<Vec<_>>(), [1000, 500]);

    assert_eq!(s.month_charges(d("2026-03-01"), d("2026-02-15")).unwrap().total, 1500);
    let st = s.stats(d("2026-02-15"), 0).unwrap();
    let months: Vec<_> = st.months.iter().map(|m| (m.month.as_str(), m.total)).collect();
    assert_eq!(months, [("2026-01", 3000), ("2026-02", 1000), ("2026-03", 1500)]);
    assert_eq!(st.yearly, 12_000);
}

#[test]
fn shared_payment_counts_your_part_of_them() {
    let mut s = Store::open_in_memory().unwrap();
    let a = add(&mut s, "Rent");
    let fields = SubscriptionFields { name: "Rent".into(), my_share: Some(50), ..Default::default() };
    s.update_subscription(&a, &fields).unwrap();
    extra(&mut s, &a, "2026-01-20", 400, "Repair");
    assert_eq!(s.detail(&a, d("2026-01-31")).unwrap().total_spent, 500 + 200);
}

#[test]
fn edit_delete_restore_and_checks() {
    let mut s = Store::open_in_memory().unwrap();
    let a = add(&mut s, "Netflix");
    let mut e = extra(&mut s, &a, "2026-02-03", 499, "Extra member");
    e.amount_minor = 599;
    e.due_date = d("2026-02-04");
    s.update_extra(&e).unwrap();
    assert_eq!(s.subscription(&a).unwrap().extras, [e.clone()]);
    s.delete_extra(&e.id).unwrap();
    assert!(s.subscription(&a).unwrap().extras.is_empty());
    assert!(s.delete_extra(&e.id).is_err());
    s.restore_extra(&e.id).unwrap();
    assert_eq!(s.subscription(&a).unwrap().extras.len(), 1);

    let bad = |s: &mut Store, amount: i64, cur: &str, sub: &str| {
        s.add_extra(&NewExtraCharge {
            subscription_id: sub.into(),
            due_date: d("2026-02-01"),
            amount_minor: amount,
            currency: cur.into(),
            label: String::new(),
        })
        .is_err()
    };
    assert!(bad(&mut s, 0, "EUR", &a));
    assert!(bad(&mut s, 100, "XYZ", &a));
    assert!(bad(&mut s, 100, "EUR", "nope"));
}

#[test]
fn no_reminder_for_a_one_time_charge() {
    let mut s = Store::open_in_memory().unwrap();
    let a = add(&mut s, "Netflix");
    extra(&mut s, &a, "2026-02-03", 499, "");
    let now = reminders::local_ms(d("2026-02-01"), chrono::NaiveTime::from_hms_opt(8, 0, 0).unwrap(), reminders::parse_tz("UTC"));
    let rs = s.reminders(now, "UTC").unwrap();
    assert!(rs.iter().all(|r| !r.text.contains("4.99")), "{rs:?}");
}

#[test]
fn syncs_to_another_device() {
    let mut a = Store::open_in_memory().unwrap();
    let mut b = Store::open_in_memory().unwrap();
    let id = add(&mut a, "Netflix");
    extra(&mut a, &id, "2026-02-03", 499, "Extra member");
    b.merge(&a.changes_since(0, 10_000, None).unwrap()).unwrap();
    assert_eq!(b.subscription(&id).unwrap().extras, a.subscription(&id).unwrap().extras);
}
