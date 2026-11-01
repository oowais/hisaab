mod common;
use chrono::NaiveTime;
use common::*;
use hisaab_core::reminders::{local_ms, money_text, parse_tz};
use hisaab_core::*;
use serde_json::json;

const TZ: &str = "Europe/Berlin";

/// UTC ms of a Berlin wall-clock time.
fn at(day: &str, hm: &str) -> i64 {
    local_ms(d(day), NaiveTime::parse_from_str(hm, "%H:%M").unwrap(), parse_tz(TZ))
}

fn add(s: &mut Store, name: &str, t: Terms, start: &str, trial_end: Option<&str>, days: Option<u32>) -> String {
    s.add_subscription(&NewSubscription {
        fields: SubscriptionFields { name: name.into(), remind_days_before: days, ..Default::default() },
        terms: t,
        start_date: d(start),
        anchor_date: None,
        trial_end: trial_end.map(d),
    })
    .unwrap()
    .subscription
    .id
}

#[test]
fn next_reminder_is_days_before_at_the_reminder_time() {
    let mut s = Store::open_in_memory().unwrap();
    add(&mut s, "Netflix", monthly(499), "2026-01-15", None, None);
    let rs = s.reminders(at("2026-02-01", "12:00"), TZ).unwrap();
    assert_eq!(rs.len(), 1);
    assert_eq!(rs[0].at_ms, at("2026-02-12", "09:00"));
    assert_eq!(rs[0].title, "Netflix · €4.99 in 3 days");
    assert_eq!(rs[0].text, "Sun 15 Feb");
    assert!(rs[0].route.starts_with("/sub/"));
}

#[test]
fn settings_and_per_subscription_days() {
    let mut s = Store::open_in_memory().unwrap();
    s.update_setting("reminder_time", &json!("18:30")).unwrap();
    s.update_setting("default_remind_days", &json!(1)).unwrap();
    assert!(s.update_setting("reminder_time", &json!("25:00")).is_err());
    add(&mut s, "A", monthly(100), "2026-01-15", None, None);
    add(&mut s, "B", monthly(100), "2026-01-15", None, Some(0));
    let mut rs = s.reminders(at("2026-02-01", "12:00"), TZ).unwrap();
    rs.sort_by_key(|r| r.at_ms);
    assert_eq!(rs.iter().map(|r| r.at_ms).collect::<Vec<_>>(), [at("2026-02-14", "18:30"), at("2026-02-15", "18:30")]);
    assert_eq!(rs[0].title, "A · €1.00 tomorrow");
    assert_eq!(rs[1].title, "B · €1.00 today");
}

#[test]
fn nothing_from_the_past_fires_on_first_sight() {
    let mut s = Store::open_in_memory().unwrap();
    // Charge tomorrow; its 3-days-before reminder was two days ago.
    add(&mut s, "Gym", monthly(3000), "2026-01-16", None, None);
    let now = at("2026-02-15", "10:00");
    let rs = s.reminders(now, TZ).unwrap();
    assert_eq!(rs[0].at_ms, at("2026-03-13", "09:00"));
}

#[test]
fn fire_then_next_and_early_alarms() {
    let mut s = Store::open_in_memory().unwrap();
    let id = add(&mut s, "Netflix", monthly(499), "2026-01-15", None, None);
    s.reminders(at("2026-02-01", "12:00"), TZ).unwrap();
    // Too early: nothing to show, same alarm again.
    let early = s.on_fired(&id, at("2026-02-12", "08:00"), TZ).unwrap();
    assert_eq!((early.show, early.next.map(|r| r.at_ms)), (None, Some(at("2026-02-12", "09:00"))));
    let f = s.on_fired(&id, at("2026-02-12", "09:00") + 500, TZ).unwrap();
    assert_eq!(f.show.unwrap().title, "Netflix · €4.99 in 3 days");
    assert_eq!(f.next.unwrap().at_ms, at("2026-03-12", "09:00"));
    // Fired twice (duplicate alarm): shown once.
    assert_eq!(s.on_fired(&id, at("2026-02-12", "09:01"), TZ).unwrap().show, None);
}

#[test]
fn snooze_shows_the_same_reminder_later() {
    let mut s = Store::open_in_memory().unwrap();
    let id = add(&mut s, "Netflix", monthly(499), "2026-01-15", None, None);
    s.reminders(at("2026-02-01", "12:00"), TZ).unwrap();
    s.on_fired(&id, at("2026-02-12", "09:00"), TZ).unwrap();
    let until = at("2026-02-13", "09:00");
    let next = s.snooze_reminder(&id, until, at("2026-02-12", "09:05"), TZ).unwrap().unwrap();
    assert_eq!((next.at_ms, next.title.as_str()), (until, "Netflix · €4.99 in 3 days"));
    let f = s.on_fired(&id, until, TZ).unwrap();
    assert_eq!(f.show.unwrap().title, "Netflix · €4.99 in 3 days");
    assert_eq!(f.next.unwrap().at_ms, at("2026-03-12", "09:00"));
}

#[test]
fn trial_ending_and_free_trial_without_charges() {
    let mut s = Store::open_in_memory().unwrap();
    add(&mut s, "Claude", monthly(2000), "2026-02-01", Some("2026-02-14"), None);
    let rs = s.reminders(at("2026-02-01", "08:00"), TZ).unwrap();
    assert_eq!(rs[0].title, "Claude trial ends in 2 days");
    assert_eq!(rs[0].at_ms, at("2026-02-12", "09:00"));
}

#[test]
fn archived_and_paused_have_no_reminders() {
    let mut s = Store::open_in_memory().unwrap();
    let a = add(&mut s, "A", monthly(100), "2026-01-15", None, None);
    let b = add(&mut s, "B", monthly(100), "2026-01-15", None, None);
    s.set_archived(&a, true).unwrap();
    s.pause(&b, d("2026-02-01"), None).unwrap();
    assert!(s.reminders(at("2026-02-01", "12:00"), TZ).unwrap().is_empty());
}

#[test]
fn money_text_and_picks() {
    assert_eq!(money_text(499, "EUR"), "€4.99");
    assert_eq!(money_text(1500, "JPY"), "¥1500");
    assert_eq!(money_text(1200, "CHF"), "CHF 12.00");
    let picks = snooze_picks(at("2026-02-12", "09:00"), TZ);
    let labels: Vec<_> = picks.iter().map(|p| p.label.as_str()).collect();
    assert_eq!(labels, ["In 1 hour", "This evening", "Tomorrow morning", "In 2 days", "Next week"]);
    assert!(picks.windows(2).all(|w| w[0].at_ms < w[1].at_ms));
}

#[test]
fn dst_gap_moves_on_an_hour() {
    // 2026-03-29 02:30 doesn't exist in Berlin.
    assert_eq!(at("2026-03-29", "02:30"), at("2026-03-29", "03:30"));
}

#[test]
fn trial_notice_wins_a_tie_with_the_first_charge() {
    let mut s = Store::open_in_memory().unwrap();
    let id = add(&mut s, "Claude", monthly(2000), "2026-02-01", Some("2026-02-14"), None);
    s.reminders(at("2026-02-01", "08:00"), TZ).unwrap();
    let f = s.on_fired(&id, at("2026-02-12", "09:00"), TZ).unwrap();
    assert_eq!(f.show.unwrap().title, "Claude trial ends in 2 days");
    assert_eq!(f.next.unwrap().at_ms, at("2026-03-12", "09:00"));
}
