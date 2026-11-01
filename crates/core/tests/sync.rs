mod common;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;

use common::*;
use hisaab_core::*;
use serde_json::json;

const T0: i64 = 1_792_000_000_000;

/// Store with its own clock (devices' clocks differ).
fn device() -> (Store, Arc<AtomicI64>) {
    let clock = Arc::new(AtomicI64::new(T0));
    let c = clock.clone();
    (Store::open_in_memory().unwrap().with_clock(move || c.load(Ordering::SeqCst)), clock)
}

/// Send `from`'s changes after `*seen` to `to`.
fn sync(from: &Store, to: &mut Store, seen: &mut i64) -> MergeOutcome {
    let changes = from.changes_since(*seen, 10_000, None).unwrap();
    if let Some(last) = changes.last() {
        *seen = last.seq;
    }
    to.merge(&changes).unwrap()
}

fn add(s: &mut Store, name: &str) -> String {
    s.add_subscription(&NewSubscription {
        fields: SubscriptionFields { name: name.into(), ..Default::default() },
        terms: monthly(999),
        start_date: d("2026-01-15"),
        anchor_date: None,
        trial_end: None,
    })
    .unwrap()
    .subscription
    .id
}

fn names(s: &Store) -> Vec<String> {
    s.subscriptions().unwrap().into_iter().map(|x| x.subscription.name).collect()
}

fn rename(s: &mut Store, id: &str, name: &str) {
    let mut f = SubscriptionFields { name: name.into(), ..Default::default() };
    f.kind = Some(s.subscription(id).unwrap().subscription.kind);
    s.update_subscription(id, &f).unwrap();
}

#[test]
fn subscriptions_travel_both_ways() {
    let (mut a, _) = device();
    let (mut b, _) = device();
    let (mut a_seen, mut b_seen) = (0, 0);
    let id = add(&mut a, "Netflix");
    a.update_setting("main_currency", &json!("USD")).unwrap();

    assert!(sync(&a, &mut b, &mut a_seen).applied > 0);
    assert_eq!(names(&b), ["Netflix"]);
    assert_eq!(b.subscription(&id).unwrap().periods, a.subscription(&id).unwrap().periods);
    assert_eq!(b.settings().unwrap().main_currency, "USD");

    b.change_terms(&id, d("2026-03-15"), &monthly(1299)).unwrap();
    sync(&b, &mut a, &mut b_seen);
    assert_eq!(a.subscription(&id).unwrap().periods.len(), 2);

    // Echo back: nothing new.
    assert_eq!(sync(&a, &mut b, &mut a_seen).applied, 0);
    assert!(a.conflicts().unwrap().is_empty() && b.conflicts().unwrap().is_empty());
}

#[test]
fn newest_edit_wins_and_the_loser_can_take_it_back() {
    let (mut a, ca) = device();
    let (mut b, cb) = device();
    let (mut a_seen, mut b_seen) = (0, 0);
    let id = add(&mut a, "Spotify");
    sync(&a, &mut b, &mut a_seen);

    // Both rename offline; B's is later.
    ca.store(T0 + 1_000, Ordering::SeqCst);
    rename(&mut a, &id, "Spotify Family");
    cb.store(T0 + 2_000, Ordering::SeqCst);
    rename(&mut b, &id, "Spotify Duo");
    sync(&a, &mut b, &mut a_seen);
    sync(&b, &mut a, &mut b_seen);
    assert_eq!(names(&a), ["Spotify Duo"]);
    assert_eq!(names(&b), ["Spotify Duo"]);

    // A lost its edit: asked about it. B won: nothing to ask.
    assert!(b.conflicts().unwrap().is_empty());
    let cs = a.conflicts().unwrap();
    assert_eq!(cs.len(), 1);
    assert_eq!((cs[0].field.as_str(), cs[0].label.as_str()), ("name", "Spotify Duo"));
    assert_eq!(cs[0].lost_value, json!("Spotify Family"));

    ca.store(T0 + 3_000, Ordering::SeqCst);
    a.resolve_conflict(&cs[0].id, true).unwrap();
    assert!(a.conflicts().unwrap().is_empty());
    sync(&a, &mut b, &mut a_seen);
    assert_eq!(names(&b), ["Spotify Family"]);
    assert!(b.conflicts().unwrap().is_empty(), "A had seen B's edit: no conflict");
}

#[test]
fn price_changes_on_both_devices_overlap_until_one_is_kept() {
    let (mut a, ca) = device();
    let (mut b, cb) = device();
    let (mut a_seen, mut b_seen) = (0, 0);
    let id = add(&mut a, "Disney+");
    sync(&a, &mut b, &mut a_seen);

    ca.store(T0 + 1_000, Ordering::SeqCst);
    a.change_terms(&id, d("2026-03-15"), &monthly(1199)).unwrap();
    cb.store(T0 + 2_000, Ordering::SeqCst);
    b.change_terms(&id, d("2026-04-15"), &monthly(1399)).unwrap();
    sync(&a, &mut b, &mut a_seen);
    sync(&b, &mut a, &mut b_seen);

    // Both see the overlap, each with its own period as "mine".
    let ca_ = a.conflicts().unwrap();
    let cb_ = b.conflicts().unwrap();
    assert_eq!(ca_.len(), 1, "{ca_:?}");
    assert_eq!(cb_.len(), 1, "{cb_:?}");
    assert_eq!((ca_[0].field.as_str(), ca_[0].label.as_str()), ("overlap", "Disney+"));
    assert_eq!(ca_[0].lost_value["amountMinor"], json!(1199));
    assert_eq!(ca_[0].won_value["amountMinor"], json!(1399));
    assert_eq!(cb_[0].lost_value["amountMinor"], json!(1399));

    // A keeps its price change.
    ca.store(T0 + 3_000, Ordering::SeqCst);
    a.resolve_conflict(&ca_[0].id, true).unwrap();
    let ps = a.subscription(&id).unwrap().periods;
    invariants::validate_periods(&ps).unwrap();
    assert_eq!(ps.len(), 2);
    assert_eq!((ps[0].end_date, ps[1].start_date, ps[1].terms.amount_minor), (Some(d("2026-03-14")), d("2026-03-15"), 1199));

    sync(&a, &mut b, &mut a_seen);
    assert_eq!(b.subscription(&id).unwrap().periods, ps);
    assert!(b.conflicts().unwrap().is_empty(), "settled on the other device");
    assert!(a.conflicts().unwrap().is_empty());
}

#[test]
fn restore_keeps_device_and_new_changes_still_sync() {
    let dir = std::env::temp_dir().join(format!("hisaab-sync-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("snap.db");
    let (mut a, _) = device();
    let (mut b, _) = device();
    let (mut a_seen, mut b_seen) = (0, 0);
    add(&mut a, "Proton");
    a.add_peer("endpoint-b", &b.device_id().unwrap(), "Phone").unwrap();
    let me = a.device_id().unwrap();
    a.export_to(file.to_str().unwrap()).unwrap();
    add(&mut a, "Kagi");
    sync(&a, &mut b, &mut a_seen);

    a.import_from(file.to_str().unwrap()).unwrap();
    assert_eq!(names(&a), ["Proton"]);
    assert_eq!(a.device_id().unwrap(), me);
    assert_eq!(a.peers().unwrap().len(), 1);

    add(&mut a, "Mullvad VPN");
    sync(&a, &mut b, &mut a_seen);
    let mut got = names(&b);
    got.sort();
    assert_eq!(got, ["Kagi", "Mullvad VPN", "Proton"]);
    // Asking B for everything again brings back what the restore rolled back.
    sync(&b, &mut a, &mut b_seen);
    assert_eq!(names(&a).len(), 3);
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn not_a_backup_is_refused() {
    let (mut a, _) = device();
    let file = std::env::temp_dir().join(format!("hisaab-junk-{}.db", std::process::id()));
    std::fs::write(&file, b"hello").unwrap();
    assert!(a.import_from(file.to_str().unwrap()).is_err());
    std::fs::remove_file(file).unwrap();
}
