mod common;
use common::*;
use hisaab_core::*;

fn add(s: &mut Store, name: &str, t: Terms, start: &str, category: Option<&str>) -> String {
    s.add_subscription(&NewSubscription {
        fields: SubscriptionFields { name: name.into(), category_id: category.map(String::from), ..Default::default() },
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
fn months_follow_past_prices_and_splits() {
    let mut s = Store::open_in_memory().unwrap();
    let cat = s.add_category("Streaming", "").unwrap().id;
    let a = add(&mut s, "A", monthly(1000), "2026-01-10", Some(&cat));
    s.change_terms(&a, d("2026-03-10"), &monthly(1500)).unwrap();
    add(&mut s, "B", monthly(500), "2026-02-01", None);

    let st = s.stats(d("2026-03-20"), 0).unwrap();
    let months: Vec<_> = st.months.iter().map(|m| (m.month.as_str(), m.total, m.forecast)).collect();
    assert_eq!(
        months,
        [("2026-01", 1000, false), ("2026-02", 1500, false), ("2026-03", 2000, true), ("2026-04", 2000, true)]
    );
    assert_eq!(st.months[2].by_category, [Slice { key: Some(cat.clone()), amount: 1500 }, Slice { key: None, amount: 500 }]);
    assert_eq!(st.yearly, 2000 * 12);
    assert_eq!(st.by_category[0], Slice { key: Some(cat), amount: 1500 * 12 });
    assert_eq!(st.by_currency, [Slice { key: Some("EUR".into()), amount: 2000 * 12 }]);
    assert_eq!(st.from, Some(d("2026-01-10")));

    // At most 2 months back.
    let st = s.stats(d("2026-03-20"), 2).unwrap();
    assert_eq!(st.months.first().map(|m| m.month.as_str()), Some("2026-02"));
}

#[test]
fn saved_counts_pauses_and_cancelling_until_today() {
    let mut s = Store::open_in_memory().unwrap();
    let a = add(&mut s, "Gym", monthly(3000), "2026-01-01", None);
    // Paused Feb–Mar: 2 charges not made.
    s.pause(&a, d("2026-02-01"), Some(d("2026-04-01"))).unwrap();
    let b = add(&mut s, "Netflix", monthly(1000), "2026-01-15", None);
    // Cancelled after January: Feb 15, Mar 15, Apr 15 not made by Apr 20.
    s.cancel(&b, d("2026-02-14")).unwrap();

    let st = s.stats(d("2026-04-20"), 0).unwrap();
    assert_eq!(st.saved, 2 * 3000 + 3 * 1000);
    assert_eq!(st.yearly, 3000 * 12, "only the gym runs");
}
