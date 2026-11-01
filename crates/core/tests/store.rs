use hisaab_core::*;
use serde_json::json;

#[test]
fn opens_and_migrates_in_memory() {
    let s = Store::open_in_memory().unwrap();
    assert_eq!(s.setting("main_currency").unwrap(), None);
}

#[test]
fn settings_round_trip_and_overwrite() {
    let mut s = Store::open_in_memory().unwrap().with_clock(|| 1_000);
    s.set_setting("main_currency", &json!("EUR")).unwrap();
    s.set_setting("main_currency", &json!("USD")).unwrap();
    assert_eq!(s.setting("main_currency").unwrap(), Some(json!("USD")));
}

#[test]
fn rejects_empty_setting_key() {
    let mut s = Store::open_in_memory().unwrap();
    assert!(matches!(s.set_setting("", &json!(1)), Err(Error::Invalid(_))));
}

#[test]
fn reopens_a_file_database() {
    let dir = std::env::temp_dir().join(format!("hisaab-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("hisaab.db");
    let path = path.to_str().unwrap();
    {
        let mut s = Store::open(path).unwrap();
        s.set_setting("default_remind_days", &json!(3)).unwrap();
    }
    let s = Store::open(path).unwrap();
    assert_eq!(s.setting("default_remind_days").unwrap(), Some(json!(3)));
    drop(s);
    std::fs::remove_dir_all(&dir).unwrap();
}
