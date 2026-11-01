//! P2P sync glue (copied from Befikr): the app side of `hisaab-sync` (store
//! access, reminders, events) and the commands behind Settings → Sync.

use hisaab_core::{Conflict, MergeOutcome, Store, SyncMode};
use hisaab_sync::{Host, Status, SyncService};
use tauri::{AppHandle, Emitter, Manager, State};

use crate::{err, rearm, Db, Res};

pub(crate) struct AppHost(AppHandle);

impl Host for AppHost {
    fn store<R>(&self, f: impl FnOnce(&mut Store) -> hisaab_core::Result<R>) -> hisaab_sync::Result<R> {
        let db = self.0.state::<Db>();
        let mut s = db.store.lock().map_err(err)?;
        f(&mut s).map_err(err)
    }

    fn merged(&self, outcome: MergeOutcome) {
        if outcome.applied > 0 {
            rearm(&self.0);
            let _ = self.0.emit("data-changed", ());
        }
    }

    fn status_changed(&self) {
        let _ = self.0.emit("sync-changed", ());
    }
}

pub(crate) type Sync = SyncService<AppHost>;

/// The app's own sync runs in this process: background sync (JNI
/// `syncOnce`) must not start a second endpoint with the same key.
#[cfg_attr(desktop, allow(dead_code))]
pub(crate) static APP_SYNC: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Device key: a file next to the database, not in it (backups must not
/// carry it to another device).
pub(crate) fn setup(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let path = app.path().app_data_dir()?.join("sync.key");
    let key = load_key(&path)?;
    app.manage(SyncService::new(AppHost(app.clone()), key));
    APP_SYNC.store(true, std::sync::atomic::Ordering::SeqCst);
    let app = app.clone();
    tauri::async_runtime::spawn(async move { restart(&app).await });
    Ok(())
}

/// The key in `path`, made on first use.
pub(crate) fn load_key(path: &std::path::Path) -> std::io::Result<[u8; 32]> {
    if let Some(k) = std::fs::read(path).ok().and_then(|b| b.try_into().ok()) {
        return Ok(k);
    }
    let k = hisaab_sync::new_secret_key();
    std::fs::write(path, k)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
    }
    Ok(k)
}

/// Bring the endpoint in line with the stored mode.
async fn restart(app: &AppHandle) {
    if let Err(e) = app.state::<Sync>().apply_mode().await {
        eprintln!("sync: {e}");
    }
}

/// A local write happened: send it to paired devices soon.
pub(crate) fn nudge(app: &AppHandle) {
    if let Some(sync) = app.try_state::<Sync>() {
        sync.nudge();
    }
}

#[tauri::command]
pub(crate) fn sync_status(sync: State<'_, Sync>) -> Status {
    sync.status()
}

#[tauri::command]
pub(crate) async fn set_sync_mode(app: AppHandle, db: State<'_, Db>, mode: SyncMode) -> Res<()> {
    db.store.lock().map_err(err)?.set_sync_mode(mode).map_err(err)?;
    restart(&app).await;
    Ok(())
}

#[tauri::command]
pub(crate) fn set_device_name(sync: State<'_, Sync>, db: State<'_, Db>, name: String) -> Res<()> {
    db.store.lock().map_err(err)?.set_device_name(&name).map_err(err)?;
    sync.stop_pairing(); // an open link carries the old name
    Ok(())
}

#[tauri::command]
pub(crate) fn start_pairing(sync: State<'_, Sync>) -> Res<()> {
    sync.start_pairing()
}

#[tauri::command]
pub(crate) fn stop_pairing(sync: State<'_, Sync>) {
    sync.stop_pairing()
}

#[tauri::command]
pub(crate) async fn pair_with_link(sync: State<'_, Sync>, link: String) -> Res<()> {
    sync.pair_with_link(&link).await.map(|_| ())
}

#[tauri::command]
pub(crate) async fn unpair(sync: State<'_, Sync>, endpoint: String) -> Res<()> {
    sync.unpair(&endpoint).await
}

#[tauri::command]
pub(crate) fn conflicts(db: State<'_, Db>) -> Res<Vec<Conflict>> {
    db.store.lock().map_err(err)?.conflicts().map_err(err)
}

/// `use_mine`: write this device's lost value back (it then syncs out).
#[tauri::command]
pub(crate) fn resolve_conflict(app: AppHandle, db: State<'_, Db>, id: String, use_mine: bool) -> Res<()> {
    db.store.lock().map_err(err)?.resolve_conflict(&id, use_mine).map_err(err)?;
    rearm(&app);
    let _ = app.emit("data-changed", ());
    let _ = app.emit("sync-changed", ());
    nudge(&app);
    Ok(())
}

#[tauri::command]
pub(crate) async fn sync_now(sync: State<'_, Sync>) -> Res<()> {
    sync.sync_now().await;
    match sync.status().last_error {
        Some(e) => Err(e),
        None => Ok(()),
    }
}
