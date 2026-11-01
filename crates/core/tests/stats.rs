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
    // Paid only: up to today (Mar 20), nothing coming.
    let months: Vec<_> = st.months.iter().map(|m| (m.month.as_str(), m.total)).collect();
    assert_eq!(months, [("2026-01", 1000), ("2026-02", 1500), ("2026-03", 2000)]);
    assert_eq!(st.months[2].by_category, [Slice { key: Some(cat.clone()), amount: 1500 }, Slice { key: None, amount: 500 }]);
    assert_eq!((st.total, st.this_month, st.this_year), (4500, 2000, 4500));
    assert_eq!(st.by_category, [Slice { key: Some(cat), amount: 1000 + 1000 + 1500 }, Slice { key: None, amount: 1000 }]);
    assert_eq!(st.by_currency, [Slice { key: Some("EUR".into()), amount: 4500 }]);
    assert_eq!(st.from, Some(d("2026-01-10")));

    // Mar 5: only B (Mar 1) is paid this month so far, not A (Mar 10).
    let early = s.stats(d("2026-03-05"), 0).unwrap();
    assert_eq!((early.this_month, early.months.last().map(|m| m.total)), (500, Some(500)));

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
    // Paid: gym Jan + Apr, Netflix Jan.
    assert_eq!(st.total, 3000 + 3000 + 1000);
}
