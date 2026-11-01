//! JNI entry points for `com.hisaab.reminders.Native` (Befikr's bridge).
//! Called from broadcast receivers and the snooze overlay, possibly in a
//! cold process with no Tauri app or webview.

use hisaab_core::Store;
use jni::objects::{JClass, JString};
use jni::sys::{jlong, jstring};
use jni::JNIEnv;
use serde_json::json;

fn arg(env: &mut JNIEnv, s: &JString) -> Result<String, String> {
    env.get_string(s).map(Into::into).map_err(|e| e.to_string())
}

/// Never unwind across FFI: errors and panics become `{"error": ...}`.
fn respond(env: &mut JNIEnv, f: impl FnOnce(&mut JNIEnv) -> Result<serde_json::Value, String>) -> jstring {
    let out = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| f(env)))
        .unwrap_or_else(|_| Err("panic in rust".into()))
        .unwrap_or_else(|e| json!({ "error": e }));
    env.new_string(out.to_string()).map(|s| s.into_raw()).unwrap_or(std::ptr::null_mut())
}

fn store(path: &str) -> Result<Store, String> {
    Store::open(path).map_err(|e| e.to_string())
}

fn now_ms() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_millis() as i64)
}

/// Alarm went off. → `{"show": Reminder?, "next": Reminder?}`
#[no_mangle]
pub extern "system" fn Java_com_hisaab_reminders_Native_onFired<'l>(
    mut env: JNIEnv<'l>,
    _class: JClass<'l>,
    db_path: JString<'l>,
    sub_id: JString<'l>,
    tz: JString<'l>,
) -> jstring {
    respond(&mut env, |env| {
        let (path, id, tz) = (arg(env, &db_path)?, arg(env, &sub_id)?, arg(env, &tz)?);
        let out = store(&path)?.on_fired(&id, now_ms(), &tz).map_err(|e| e.to_string())?;
        serde_json::to_value(out).map_err(|e| e.to_string())
    })
}

/// Every subscription's next reminder. → `[Reminder]`
#[no_mangle]
pub extern "system" fn Java_com_hisaab_reminders_Native_reminders<'l>(
    mut env: JNIEnv<'l>,
    _class: JClass<'l>,
    db_path: JString<'l>,
    tz: JString<'l>,
) -> jstring {
    respond(&mut env, |env| {
        let (path, tz) = (arg(env, &db_path)?, arg(env, &tz)?);
        let rs = store(&path)?.reminders(now_ms(), &tz).map_err(|e| e.to_string())?;
        serde_json::to_value(rs).map_err(|e| e.to_string())
    })
}

/// Snooze overlay quick picks. → `[{"label","atMs"}]`
#[no_mangle]
pub extern "system" fn Java_com_hisaab_reminders_Native_snoozePicks<'l>(
    mut env: JNIEnv<'l>,
    _class: JClass<'l>,
    tz: JString<'l>,
) -> jstring {
    respond(&mut env, |env| {
        let picks = hisaab_core::snooze_picks(now_ms(), &arg(env, &tz)?);
        serde_json::to_value(picks).map_err(|e| e.to_string())
    })
}

/// Snooze overlay: show the reminder again at `until_ms`. → `{"next": Reminder?}`
#[no_mangle]
pub extern "system" fn Java_com_hisaab_reminders_Native_snooze<'l>(
    mut env: JNIEnv<'l>,
    _class: JClass<'l>,
    db_path: JString<'l>,
    sub_id: JString<'l>,
    until_ms: jlong,
    tz: JString<'l>,
) -> jstring {
    respond(&mut env, |env| {
        let (path, id, tz) = (arg(env, &db_path)?, arg(env, &sub_id)?, arg(env, &tz)?);
        let next = store(&path)?.snooze_reminder(&id, until_ms, now_ms(), &tz).map_err(|e| e.to_string())?;
        Ok(json!({ "next": next }))
    })
}
