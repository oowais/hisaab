//! JNI entry points for `com.hisaab.reminders.Native` (Befikr's bridge).
//! Called from broadcast receivers, the snooze overlay, backups and
//! background sync, possibly in a cold process with no Tauri app or webview.

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

use hisaab_core::{MergeOutcome, Store, SyncMode};
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

/// Consistent snapshot of the database to `out_path` (backups). → `{}`
#[no_mangle]
pub extern "system" fn Java_com_hisaab_reminders_Native_exportTo<'l>(
    mut env: JNIEnv<'l>,
    _class: JClass<'l>,
    db_path: JString<'l>,
    out_path: JString<'l>,
) -> jstring {
    respond(&mut env, |env| {
        let (db, out) = (arg(env, &db_path)?, arg(env, &out_path)?);
        store(&db)?.export_to(&out).map_err(|e| e.to_string())?;
        Ok(json!({}))
    })
}

/// Sync side of a background run: the store, and how many fields merges wrote.
#[derive(Clone)]
struct BackgroundHost(Arc<(Mutex<Store>, AtomicU32)>);

impl hisaab_sync::Host for BackgroundHost {
    fn store<R>(&self, f: impl FnOnce(&mut Store) -> hisaab_core::Result<R>) -> hisaab_sync::Result<R> {
        let mut s = self.0 .0.lock().map_err(|e| e.to_string())?;
        f(&mut s).map_err(|e| e.to_string())
    }
    fn merged(&self, outcome: MergeOutcome) {
        self.0 .1.fetch_add(outcome.applied, Ordering::SeqCst);
    }
    fn status_changed(&self) {}
}

/// Background sync (WorkManager, app closed): one round with every paired
/// device. → `{"reminders"?: [Reminder] (when something merged: re-arm),
/// "skipped"?: why, "problem"?: error}`.
#[no_mangle]
pub extern "system" fn Java_com_hisaab_reminders_Native_syncOnce<'l>(
    mut env: JNIEnv<'l>,
    _class: JClass<'l>,
    db_path: JString<'l>,
    key_path: JString<'l>,
    tz: JString<'l>,
) -> jstring {
    respond(&mut env, |env| {
        let (db, key_path, tz) = (arg(env, &db_path)?, arg(env, &key_path)?, arg(env, &tz)?);
        if crate::sync::APP_SYNC.load(Ordering::SeqCst) {
            return Ok(json!({ "skipped": "app is syncing" }));
        }
        let s = store(&db)?;
        let off = s.sync_settings().map_err(|e| e.to_string())?.mode == SyncMode::Off;
        if off || s.peers().map_err(|e| e.to_string())?.is_empty() {
            return Ok(json!({ "skipped": "sync off or no paired devices" }));
        }
        let Some(key) = std::fs::read(&key_path).ok().and_then(|b| <[u8; 32]>::try_from(b).ok()) else {
            return Ok(json!({ "skipped": "no device key yet" }));
        };
        let host = BackgroundHost(Arc::new((Mutex::new(s), AtomicU32::new(0))));
        let svc = hisaab_sync::SyncService::new(host.clone(), key);
        let r = tauri::async_runtime::block_on(async {
            tokio_timeout(std::time::Duration::from_secs(90), svc.sync_once()).await
        });
        let mut out = json!({});
        if host.0 .1.load(Ordering::SeqCst) > 0 {
            let rs = host.0 .0.lock().map_err(|e| e.to_string())?.reminders(now_ms(), &tz).map_err(|e| e.to_string())?;
            out["reminders"] = serde_json::to_value(rs).map_err(|e| e.to_string())?;
        }
        if let Err(e) = r {
            out["problem"] = e.into();
        }
        Ok(out)
    })
}

/// Bound a background run (a peer that never answers must not hold the
/// worker for its whole 10 minutes).
async fn tokio_timeout<T>(
    d: std::time::Duration,
    f: impl std::future::Future<Output = hisaab_sync::Result<T>>,
) -> hisaab_sync::Result<T> {
    match tokio::time::timeout(d, f).await {
        Ok(r) => r,
        Err(_) => Err("background sync timed out".into()),
    }
}
