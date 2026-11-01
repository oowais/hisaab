mod common;
use common::*;
use hisaab_core::*;

const TODAY: &str = "2026-03-15";

fn add(s: &mut Store, fields: SubscriptionFields, t: Terms) -> String {
    s.add_subscription(&NewSubscription {
        fields,
        terms: t,
        start_date: d("2026-01-01"),
        anchor_date: None,
        trial_end: None,
    })
    .unwrap()
    .subscription
    .id
}

fn named(s: &mut Store, name: &str) -> String {
    add(s, SubscriptionFields { name: name.into(), ..Default::default() }, monthly(1000))
}

fn found(s: &Store, q: &str) -> Vec<String> {
    s.search(q, d(TODAY)).unwrap().into_iter().map(|v| v.subscription.name).collect()
}

#[test]
fn matches_name_notes_link_plan_category_and_method_case_insensitive() {
    let mut s = Store::open_in_memory().unwrap();
    let cat = s.add_category("Streaming", "").unwrap();
    let card = s.add_payment_method("Visa ..4417").unwrap();
    named(&mut s, "Netflix");
    add(&mut s, SubscriptionFields { name: "Rent".into(), notes: "flat in Berlin".into(), ..Default::default() }, monthly(90000));
    add(
        &mut s,
        SubscriptionFields { name: "Proton".into(), url: Some("https://account.proton.me".into()), ..Default::default() },
        monthly(500),
    );
    add(&mut s, SubscriptionFields { name: "Spotify".into(), ..Default::default() }, Terms { plan_label: Some("Duo".into()), ..monthly(1499) });
    add(&mut s, SubscriptionFields { name: "Disney+".into(), category_id: Some(cat.id), ..Default::default() }, monthly(999));
    add(&mut s, SubscriptionFields { name: "Gym".into(), payment_method_id: Some(card.id), ..Default::default() }, monthly(3000));

    assert_eq!(found(&s, "NETFLIX"), ["Netflix"]);
    assert_eq!(found(&s, "berlin"), ["Rent"]);
    assert_eq!(found(&s, "proton.me"), ["Proton"]);
    assert_eq!(found(&s, "duo"), ["Spotify"]);
    assert_eq!(found(&s, "streaming"), ["Disney+"]);
    assert_eq!(found(&s, "4417"), ["Gym"]);
}

#[test]
fn every_word_must_appear_and_wildcards_are_literal() {
    let mut s = Store::open_in_memory().unwrap();
    named(&mut s, "Apple Music");
    named(&mut s, "Apple TV+");
    named(&mut s, "50% off");
    assert_eq!(found(&s, "apple music"), ["Apple Music"]);
    assert_eq!(found(&s, "music apple"), ["Apple Music"]);
    assert_eq!(found(&s, "%"), ["50% off"]);
    assert_eq!(found(&s, "_"), Vec::<String>::new());
    assert_eq!(found(&s, "   "), Vec::<String>::new());
}

#[test]
fn running_first_then_ended_then_archived_and_never_deleted() {
    let mut s = Store::open_in_memory().unwrap();
    let a = named(&mut s, "Pay A");
    let b = named(&mut s, "Pay B");
    named(&mut s, "Pay C");
    let d_ = named(&mut s, "Pay D");
    s.cancel(&a, d("2026-02-28")).unwrap();
    s.set_archived(&b, true).unwrap();
    s.delete_subscription(&d_).unwrap();
    assert_eq!(found(&s, "pay"), ["Pay C", "Pay A", "Pay B"]);
    let hits = s.search("pay", d(TODAY)).unwrap();
    assert_eq!(hits[1].status, SubscriptionStatus::Ended);
    assert!(hits[2].subscription.archived_at.is_some());
}
