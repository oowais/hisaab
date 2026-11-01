//! The JSON the UI sends (app/composables/api.ts) parses into the core's types.
use hisaab_core::*;

#[test]
fn add_subscription_payload() {
    let new: NewSubscription = serde_json::from_str(
        r##"{"fields":{"kind":"subscription","name":"Netflix","templateKey":"netflix","icon":"si:netflix","color":"#E50914",
        "categoryId":null,"paymentMethodId":null,"url":null,"notes":"","remindDaysBefore":null},
        "terms":{"amountMinor":1399,"currency":"EUR","everyN":1,"unit":"month","planLabel":null,"amountKind":"fixed"},
        "startDate":"2026-11-01","anchorDate":null,"trialEnd":null}"##,
    )
    .unwrap();
    assert_eq!((new.fields.name.as_str(), new.terms.amount_minor, new.terms.unit), ("Netflix", 1399, Unit::Month));
    let mut s = Store::open_in_memory().unwrap();
    s.add_subscription(&new).unwrap();
}

#[test]
fn override_and_fields_payloads() {
    let o: NewOverride = serde_json::from_str(
        r#"{"periodId":"p","dueDate":"2026-11-05","kind":"moved","newDueDate":"2026-11-07"}"#,
    )
    .unwrap();
    assert_eq!(o.kind, OverrideKind::Moved);
    let f: SubscriptionFields = serde_json::from_str(
        r#"{"kind":"bill","name":"Rent","templateKey":null,"icon":"lu:House","color":null,"categoryId":null,
        "paymentMethodId":null,"url":null,"notes":"","remindDaysBefore":2}"#,
    )
    .unwrap();
    assert_eq!((f.kind, f.remind_days_before), (Some(Kind::Bill), Some(2)));
}
