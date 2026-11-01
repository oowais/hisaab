mod common;
use common::*;
use hisaab_core::*;
use serde_json::json;

fn store() -> Store {
    let mut s = Store::open_in_memory().unwrap();
    s.put_rates(d("2026-01-02"), &[("USD".into(), 1.25)]).unwrap();
    s
}

fn add(s: &mut Store, name: &str, t: Terms, start: &str) -> String {
    s.add_subscription(&NewSubscription {
        fields: SubscriptionFields { name: name.into(), ..Default::default() },
        terms: t,
        start_date: d(start),
        anchor_date: None,
        trial_end: None,
    })
    .unwrap()
    .subscription
    .id
}

#[test]
fn yearly_total_converts_and_leaves_out_paused_and_unknown() {
    let mut s = store();
    add(&mut s, "Netflix", monthly(1000), "2026-01-01"); // 120.00 / year
    add(&mut s, "GitHub", terms(10000, "USD", 1, Unit::Year), "2026-01-01"); // $100 = 80.00
    let gym = add(&mut s, "Gym", monthly(3000), "2026-01-01");
    s.pause(&gym, d("2026-01-10"), None).unwrap();
    add(&mut s, "Local", monthly(100000), "2026-01-01");
    s.update_setting("main_currency", &json!("EUR")).unwrap();
    let ars = add(&mut s, "Peso", terms(5000, "ARS", 1, Unit::Month), "2026-01-01");
    let o = s.overview(d("2026-01-15")).unwrap();
    assert_eq!(o.yearly_total, 12000 + 8000 + 1_200_000);
    assert_eq!(o.excluded_currencies, ["ARS"]);
    let peso = o.subscriptions.iter().find(|v| v.subscription.id == ars).unwrap();
    assert_eq!((peso.yearly_in_main, peso.next_in_main), (None, None));
    let g = o.subscriptions.iter().find(|v| v.subscription.id == gym).unwrap();
    assert_eq!((g.status, g.yearly_in_main, g.next_charge.is_none()), (SubscriptionStatus::Paused, None, true));
}

#[test]
fn upcoming_runs_to_the_end_of_next_month() {
    let mut s = store();
    add(&mut s, "Weekly", terms(500, "EUR", 1, Unit::Week), "2026-01-05");
    let o = s.overview(d("2026-01-30")).unwrap();
    let first = o.upcoming.first().unwrap().charge.due_date;
    let last = o.upcoming.last().unwrap().charge.due_date;
    assert_eq!((first, last), (d("2026-02-02"), d("2026-02-23")));
    assert!(o.upcoming.iter().all(|c| c.in_main == Some(500)));
}

#[test]
fn detail_history_and_total_spent() {
    let mut s = store();
    let id = add(&mut s, "Netflix", monthly(1000), "2025-11-15");
    s.change_terms(&id, d("2026-01-15"), &monthly(1200)).unwrap();
    let det = s.detail(&id, d("2026-02-01")).unwrap();
    assert_eq!(dates(&det.history.iter().map(|h| h.charge.clone()).collect::<Vec<_>>()), ["2026-01-15", "2025-12-15", "2025-11-15"]);
    assert_eq!(det.total_spent, 3200);
    assert_eq!(det.history_from, Some(d("2025-11-15")));
    assert_eq!(det.view.next_charge.unwrap().due_date, d("2026-02-15"));
    assert_eq!(det.periods.len(), 2);
}

#[test]
fn settings_defaults_and_validation() {
    let mut s = store();
    let st = s.settings().unwrap();
    assert_eq!((st.main_currency.as_str(), st.default_remind_days, st.last_currency), ("EUR", 3, None));
    s.update_setting("main_currency", &json!("USD")).unwrap();
    s.update_setting("default_remind_days", &json!(5)).unwrap();
    assert!(s.update_setting("main_currency", &json!("usd")).is_err());
    assert!(s.update_setting("nope", &json!(1)).is_err());
    let st = s.settings().unwrap();
    assert_eq!((st.main_currency.as_str(), st.default_remind_days), ("USD", 5));
    assert_eq!(st.monthly_budget, None);
}

#[test]
fn budget_follows_the_main_currency() {
    let mut s = store(); // 1 EUR = 1.25 USD
    s.update_setting("monthly_budget", &json!(10000)).unwrap();
    assert!(s.update_setting("monthly_budget", &json!(-5)).is_err());
    s.update_setting("main_currency", &json!("USD")).unwrap();
    assert_eq!(s.settings().unwrap().monthly_budget, Some(12500));
    s.update_setting("monthly_budget", &json!(null)).unwrap();
    assert_eq!(s.settings().unwrap().monthly_budget, None);
    // $12/year in main (USD) a month: $1.
    assert_eq!(s.monthly_in_main(1200, "USD", 1, Unit::Year).unwrap(), Some(100));
    assert_eq!(s.monthly_in_main(1000, "EUR", 1, Unit::Month).unwrap(), Some(1250));
}

#[test]
fn per_year_factors() {
    assert_eq!(per_year(1, Unit::Month), 12.0);
    assert_eq!(per_year(3, Unit::Month), 4.0);
    assert_eq!(per_year(2, Unit::Year), 0.5);
    assert!((per_year(1, Unit::Week) - 52.1775).abs() < 0.001);
}

#[test]
fn timeline_has_all_periods_but_not_archived() {
    let mut s = store();
    let a = add(&mut s, "A", monthly(100), "2025-01-01");
    s.change_terms(&a, d("2025-06-01"), &monthly(200)).unwrap();
    let b = add(&mut s, "B", monthly(100), "2025-01-01");
    s.cancel(&b, d("2025-03-31")).unwrap();
    let c = add(&mut s, "C", monthly(100), "2025-01-01");
    s.set_archived(&c, true).unwrap();
    let lanes = s.timeline(d("2026-01-01")).unwrap();
    let got: Vec<_> = lanes.iter().map(|l| (l.view.subscription.name.as_str(), l.periods.len(), l.view.status)).collect();
    assert_eq!(got, [("A", 2, SubscriptionStatus::Active), ("B", 1, SubscriptionStatus::Ended)]);
    let archived: Vec<_> = s.archived(d("2026-01-01")).unwrap().into_iter().map(|v| v.subscription.name).collect();
    assert_eq!(archived, ["C"]);
}

#[test]
fn a_months_charges_and_total() {
    let mut s = store();
    add(&mut s, "A", monthly(1000), "2026-01-10");
    add(&mut s, "B", terms(1200, "EUR", 1, Unit::Week), "2026-03-02");
    let m = s.month_charges(d("2026-03-17"), d("2026-03-17")).unwrap();
    let days: Vec<_> = m.charges.iter().map(|c| c.charge.due_date.to_string()).collect();
    assert_eq!(days, ["2026-03-02", "2026-03-09", "2026-03-10", "2026-03-16", "2026-03-23", "2026-03-30"]);
    assert_eq!(m.total, 1000 + 5 * 1200);
}

#[test]
fn widget_shows_the_month_and_next_charges() {
    let mut s = store();
    add(&mut s, "Rent", monthly(95000), "2026-01-01");
    add(&mut s, "Netflix", monthly(1399), "2026-01-20");
    // 2026-03-17 12:00 UTC.
    let w = s.widget(1_773_748_800_000, "Europe/Berlin", 1).unwrap();
    assert_eq!(w.today, d("2026-03-17"));
    assert_eq!(w.monthly_total, 95000 + 1399);
    assert_eq!(w.charges.len(), 1);
    assert_eq!((w.charges[0].name.as_str(), w.charges[0].due_date), ("Netflix", d("2026-03-20")));
}
