#[cfg(desktop)]
mod desktop;
mod fx;

use std::sync::Mutex;

use chrono::NaiveDate;
use hisaab_core::*;
use serde::Serialize;
use tauri::{AppHandle, Manager, State};

pub(crate) struct Db {
    pub(crate) store: Mutex<Store>,
}

type Res<T> = std::result::Result<T, String>;

fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

fn with<T>(db: &State<Db>, f: impl FnOnce(&mut Store) -> hisaab_core::Result<T>) -> Res<T> {
    let mut store = db.store.lock().map_err(err)?;
    f(&mut store).map_err(err)
}

// ---- Reads ----

#[tauri::command]
fn overview(db: State<Db>, today: NaiveDate) -> Res<Overview> {
    with(&db, |s| s.overview(today))
}

#[tauri::command]
fn timeline(db: State<Db>, today: NaiveDate) -> Res<Vec<TimelineLane>> {
    with(&db, |s| s.timeline(today))
}

#[tauri::command]
fn detail(db: State<Db>, id: String, today: NaiveDate) -> Res<Detail> {
    with(&db, |s| s.detail(&id, today))
}

/// Default dates for the action sheets: next charge (price change) and the
/// end of the running billing cycle (cancel).
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Defaults {
    next_charge: Option<NaiveDate>,
    cycle_end: NaiveDate,
}

#[tauri::command]
fn defaults(db: State<Db>, id: String, today: NaiveDate) -> Res<Defaults> {
    with(&db, |s| {
        Ok(Defaults { next_charge: s.next_charge(&id, today)?.map(|c| c.due_date), cycle_end: s.cycle_end(&id, today)? })
    })
}

#[tauri::command]
fn categories(db: State<Db>) -> Res<Vec<Category>> {
    with(&db, |s| s.categories())
}

#[tauri::command]
fn payment_methods(db: State<Db>) -> Res<Vec<PaymentMethod>> {
    with(&db, |s| s.payment_methods())
}

#[tauri::command]
fn settings(db: State<Db>) -> Res<Settings> {
    with(&db, |s| s.settings())
}

// ---- Writes ----

#[tauri::command]
fn add_subscription(db: State<Db>, new: NewSubscription) -> Res<String> {
    with(&db, |s| {
        let id = s.add_subscription(&new)?.subscription.id;
        s.update_setting("last_currency", &serde_json::json!(new.terms.currency))?;
        Ok(id)
    })
}

#[tauri::command]
fn update_subscription(db: State<Db>, id: String, fields: SubscriptionFields) -> Res<()> {
    with(&db, |s| s.update_subscription(&id, &fields))
}

#[tauri::command]
fn change_terms(db: State<Db>, id: String, from: NaiveDate, terms: Terms) -> Res<()> {
    with(&db, |s| s.change_terms(&id, from, &terms).map(drop))
}

#[tauri::command]
fn pause(db: State<Db>, id: String, from: NaiveDate, resume_on: Option<NaiveDate>) -> Res<()> {
    with(&db, |s| s.pause(&id, from, resume_on).map(drop))
}

#[tauri::command]
fn resume(db: State<Db>, id: String, on: NaiveDate, anchor: Option<NaiveDate>) -> Res<()> {
    with(&db, |s| s.resume(&id, on, anchor).map(drop))
}

#[tauri::command]
fn cancel(db: State<Db>, id: String, ends_on: NaiveDate) -> Res<()> {
    with(&db, |s| s.cancel(&id, ends_on).map(drop))
}

#[tauri::command]
fn restart(db: State<Db>, id: String, on: NaiveDate, terms: Option<Terms>) -> Res<()> {
    with(&db, |s| s.restart(&id, on, terms.as_ref()).map(drop))
}

#[tauri::command]
fn convert_trial(db: State<Db>, id: String, on: NaiveDate, terms: Option<Terms>) -> Res<()> {
    with(&db, |s| s.convert_trial(&id, on, terms.as_ref()).map(drop))
}

#[tauri::command]
fn set_archived(db: State<Db>, id: String, archived: bool) -> Res<()> {
    with(&db, |s| s.set_archived(&id, archived))
}

#[tauri::command]
fn delete_subscription(db: State<Db>, id: String) -> Res<()> {
    with(&db, |s| s.delete_subscription(&id))
}

#[tauri::command]
fn restore_subscription(db: State<Db>, id: String) -> Res<()> {
    with(&db, |s| s.restore_subscription(&id))
}

#[tauri::command]
fn set_override(db: State<Db>, o: NewOverride) -> Res<ChargeOverride> {
    with(&db, |s| s.set_override(&o))
}

#[tauri::command]
fn delete_override(db: State<Db>, id: String) -> Res<()> {
    with(&db, |s| s.delete_override(&id))
}

#[tauri::command]
fn add_category(db: State<Db>, name: String, color: String) -> Res<Category> {
    with(&db, |s| s.add_category(&name, &color))
}

#[tauri::command]
fn add_payment_method(db: State<Db>, name: String) -> Res<PaymentMethod> {
    with(&db, |s| s.add_payment_method(&name))
}

#[tauri::command]
fn update_setting(db: State<Db>, key: String, value: serde_json::Value) -> Res<()> {
    with(&db, |s| s.update_setting(&key, &value))
}

// ---- Rates (network off the main thread) ----

/// Fetch the latest rates unless today's (or a newer business day's) are
/// cached. True when new rates came in.
#[tauri::command]
async fn refresh_rates(app: AppHandle, today: NaiveDate) -> Res<bool> {
    let have = app.state::<Db>().store.lock().map_err(err)?.latest_rates_date().map_err(err)?;
    if have.is_some_and(|d| d >= today) {
        return Ok(false);
    }
    let (date, rates) = tauri::async_runtime::spawn_blocking(fx::latest).await.map_err(err)??;
    if have.is_some_and(|d| d >= date) {
        return Ok(false);
    }
    app.state::<Db>().store.lock().map_err(err)?.put_rates(date, &rates).map_err(err)?;
    Ok(true)
}

/// Make sure rates go back to `from` (a subscription's history). True when
/// new rates came in.
#[tauri::command]
async fn ensure_rates(app: AppHandle, from: NaiveDate, today: NaiveDate) -> Res<bool> {
    let earliest = app.state::<Db>().store.lock().map_err(err)?.earliest_rates_date().map_err(err)?;
    // ECB rates start in 1999; a few days' margin covers a weekend before `from`.
    let from = from.checked_sub_days(chrono::Days::new(5)).unwrap_or(from);
    let to = match earliest {
        Some(e) if e <= from => return Ok(false),
        Some(e) => e.pred_opt().unwrap_or(e),
        None => today,
    };
    let days = tauri::async_runtime::spawn_blocking(move || fx::range(from, to)).await.map_err(err)??;
    let db = app.state::<Db>();
    let mut store = db.store.lock().map_err(err)?;
    for (date, rates) in &days {
        store.put_rates(*date, rates).map_err(err)?;
    }
    Ok(!days.is_empty())
}

// ---- Updates ----

/// Windows: download the signed installer from the latest release, run it
/// and restart. False when there is nothing newer. Linux updates come from
/// the package (scripts/install-arch.sh); Android from the plugin.
#[cfg(desktop)]
#[tauri::command]
async fn install_update(app: AppHandle, on_progress: tauri::ipc::Channel<serde_json::Value>) -> Res<bool> {
    use tauri_plugin_updater::UpdaterExt;
    let Some(update) = app.updater().map_err(err)?.check().await.map_err(err)? else {
        return Ok(false);
    };
    let mut done = 0u64;
    let mut last = u64::MAX;
    update
        .download_and_install(
            |chunk, total| {
                done += chunk as u64;
                // At most once per percent (per MB without a total).
                let step = total.map_or(done >> 20, |t| done * 100 / t.max(1));
                if step != last {
                    last = step;
                    let total = total.map_or(-1, |t| t as i64);
                    let _ = on_progress.send(serde_json::json!({ "done": done, "total": total }));
                }
            },
            || {},
        )
        .await
        .map_err(err)?;
    app.restart()
}

#[cfg(mobile)]
#[tauri::command]
fn install_update(_app: AppHandle) -> Res<()> {
    Err("desktop only".into())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Wayland app id / X11 class `com.owaisahmed.hisaab` (matches StartupWMClass in
    // packaging/linux/hisaab.desktop), so launchers find the window.
    #[cfg(target_os = "linux")]
    gtk::glib::set_prgname(Some("com.owaisahmed.hisaab"));
    let builder = tauri::Builder::default();
    // A second launch focuses this one.
    #[cfg(desktop)]
    let builder = builder
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| desktop::show_main(app)))
        .plugin(tauri_plugin_updater::Builder::new().build());
    builder
        .plugin(tauri_plugin_reminders::init())
        .setup(|app| {
            let dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&dir)?;
            let path = dir.join("hisaab.db").to_string_lossy().into_owned();
            let store = Store::open(&path)?;
            app.manage(Db { store: Mutex::new(store) });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            overview,
            timeline,
            detail,
            defaults,
            categories,
            payment_methods,
            settings,
            add_subscription,
            update_subscription,
            change_terms,
            pause,
            resume,
            cancel,
            restart,
            convert_trial,
            set_archived,
            delete_subscription,
            restore_subscription,
            set_override,
            delete_override,
            add_category,
            add_payment_method,
            update_setting,
            refresh_rates,
            ensure_rates,
            install_update,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Hisaab");
}
