mod common;
use common::*;
use hisaab_core::currency::{convert, is_currency_code, minor_digits};
use hisaab_core::occurrence::occurrences;
use hisaab_core::*;

#[test]
fn codes_and_minor_digits() {
    assert!(is_currency_code("EUR") && is_currency_code("JPY") && !is_currency_code("eur") && !is_currency_code("XYZ"));
    assert_eq!((minor_digits("JPY"), minor_digits("EUR"), minor_digits("KWD")), (0, 2, 3));
}

#[test]
fn converts_through_eur_with_minor_units() {
    // $9.99 at 1.08 USD/EUR = €9.25.
    assert_eq!(convert(999, "USD", "EUR", 1.08, 1.0), 925);
    // ¥1500 (no minor units) at 160 JPY/EUR, into USD at 1.08: $10.13.
    assert_eq!(convert(1500, "JPY", "USD", 160.0, 1.08), 1013);
    // And back into yen: whole yen.
    assert_eq!(convert(1013, "USD", "JPY", 1.08, 160.0), 1501);
    assert_eq!(convert(500, "EUR", "EUR", 1.0, 1.0), 500);
}

fn store_with_rates() -> Store {
    let mut s = Store::open_in_memory().unwrap();
    // Friday and the next Monday: the weekend uses Friday's rate.
    s.put_rates(d("2026-01-09"), &[("USD".into(), 1.10), ("JPY".into(), 160.0)]).unwrap();
    s.put_rates(d("2026-01-12"), &[("USD".into(), 1.20), ("JPY".into(), 170.0)]).unwrap();
    s
}

#[test]
fn rate_on_a_weekend_is_the_previous_business_day() {
    let s = store_with_rates();
    use hisaab_core::currency::RateWhen::*;
    assert_eq!(s.rate("USD", On(d("2026-01-10"))).unwrap(), Some(1.10));
    assert_eq!(s.rate("USD", On(d("2026-01-12"))).unwrap(), Some(1.20));
    assert_eq!(s.rate("USD", Latest).unwrap(), Some(1.20));
    assert_eq!(s.rate("USD", On(d("2026-01-01"))).unwrap(), None);
    assert_eq!(s.rate("EUR", Latest).unwrap(), Some(1.0));
    assert_eq!(s.rate("ARS", Latest).unwrap(), None);
}

#[test]
fn charges_use_due_date_settled_or_latest_rates() {
    let s = store_with_rates();
    let p = active("p", terms(1100, "USD", 1, Unit::Week), "2026-01-03", "2026-01-03", None);
    let settled = ChargeOverride {
        id: "o".into(),
        period_id: "p".into(),
        due_date: d("2026-01-03"),
        kind: OverrideKind::Settled,
        new_due_date: None,
        actual_amount_minor: None,
        actual_currency: None,
        settled_amount_minor: Some(1005),
        settled_currency: Some("EUR".into()),
    };
    let cs = occurrences(&[p], &[settled], d("2026-01-03"), d("2026-01-17"), d("2026-01-12"));
    let eur: Vec<_> = cs.iter().map(|c| s.charge_in(c, "EUR").unwrap()).collect();
    // Jan 3: what the bank took. Jan 10 (Sat, past): Friday's 1.10. Jan 17 (upcoming): latest 1.20.
    assert_eq!(eur, [Some(1005), Some(1000), Some(917)]);
    // Into a zero-decimal main currency.
    assert_eq!(s.amount_in(1100, "USD", "JPY").unwrap(), Some(1558));
    assert_eq!(s.amount_in(1100, "ARS", "EUR").unwrap(), None);
}

#[test]
fn staleness() {
    let s = store_with_rates();
    assert!(!s.rates_stale(d("2026-01-15")).unwrap());
    assert!(s.rates_stale(d("2026-01-16")).unwrap());
    assert!(Store::open_in_memory().unwrap().rates_stale(d("2026-01-01")).unwrap());
}
