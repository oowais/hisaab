#[cfg(desktop)]
mod desktop;
mod fx;
#[cfg(target_os = "android")]
mod jni;
mod sync;

use std::sync::Mutex;

use chrono::NaiveDate;
use hisaab_core::*;
use hisaab_core::snooze_picks as snooze_picks_at;
use serde::Serialize;
use tauri::{AppHandle, Manager, State};
#[cfg(mobile)]
use tauri_plugin_reminders::RemindersExt;

pub(crate) struct Db {
    pub(crate) store: Mutex<Store>,
    /// For the Android receivers (they open the database themselves).
    #[cfg_attr(desktop, allow(dead_code))]
    path: String,
}

type Res<T> = std::result::Result<T, String>;

fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

fn with<T>(db: &State<Db>, f: impl FnOnce(&mut Store) -> hisaab_core::Result<T>) -> Res<T> {
    let mut store = db.store.lock().map_err(err)?;
    f(&mut store).map_err(err)
}

fn now_ms() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_millis() as i64)
}

/// This device's IANA zone (reminders are at a local time).
fn local_tz() -> String {
    iana_time_zone::get_timezone().unwrap_or_else(|_| "UTC".into())
}

/// Arm every subscription's next reminder: on start and after every change.
/// Scheduling failures must not fail the user's write.
pub(crate) fn rearm(app: &AppHandle) {
    let db = app.state::<Db>();
    let reminders = match db.store.lock().map_err(err).and_then(|s| s.reminders(now_ms(), &local_tz()).map_err(err)) {
        Ok(r) => r,
        Err(e) => return eprintln!("reminders: {e}"),
    };
    #[cfg(desktop)]
    app.state::<desktop::Scheduler>().replace_all(reminders);
    #[cfg(mobile)]
    if let Err(e) = app.reminders().arm(&db.path, &serde_json::to_value(&reminders).unwrap_or_default()) {
        eprintln!("reminders: {e}");
    }
}

/// A write that can change reminders: then re-arm them, and sync it soon.
fn write<T>(app: &AppHandle, f: impl FnOnce(&mut Store) -> hisaab_core::Result<T>) -> Res<T> {
    let out = with(&app.state::<Db>(), f)?;
    rearm(app);
    sync::nudge(app);
    Ok(out)
}

// ---- Reads ----

#[tauri::command]
fn overview(db: State<Db>, today: NaiveDate) -> Res<Overview> {
    with(&db, |s| s.overview(today))
}

/// Charges of the month `day` is in (Calendar).
#[tauri::command]
fn month_charges(db: State<Db>, day: NaiveDate, today: NaiveDate) -> Res<MonthCharges> {
    with(&db, |s| s.month_charges(day, today))
}

/// `months`: how far back the monthly history goes (0 = all).
#[tauri::command]
fn stats(db: State<Db>, today: NaiveDate, months: u32) -> Res<Stats> {
    with(&db, |s| s.stats(today, months))
}

#[tauri::command]
fn archived(db: State<Db>, today: NaiveDate) -> Res<Vec<SubscriptionView>> {
    with(&db, |s| s.archived(today))
}

/// Search payments: name, notes, link, plans, category, payment method.
#[tauri::command]
fn search(db: State<Db>, query: String, today: NaiveDate) -> Res<Vec<SubscriptionView>> {
    with(&db, |s| s.search(&query, today))
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
fn add_subscription(app: AppHandle, new: NewSubscription) -> Res<String> {
    write(&app, |s| {
        let id = s.add_subscription(&new)?.subscription.id;
        s.update_setting("last_currency", &serde_json::json!(new.terms.currency))?;
        Ok(id)
    })
}

#[tauri::command]
fn update_subscription(app: AppHandle, id: String, fields: SubscriptionFields) -> Res<()> {
    write(&app, |s| s.update_subscription(&id, &fields))
}

#[tauri::command]
fn change_terms(app: AppHandle, id: String, from: NaiveDate, terms: Terms) -> Res<()> {
    write(&app, |s| s.change_terms(&id, from, &terms).map(drop))
}

#[tauri::command]
fn pause(app: AppHandle, id: String, from: NaiveDate, resume_on: Option<NaiveDate>) -> Res<()> {
    write(&app, |s| s.pause(&id, from, resume_on).map(drop))
}

#[tauri::command]
fn resume(app: AppHandle, id: String, on: NaiveDate, anchor: Option<NaiveDate>) -> Res<()> {
    write(&app, |s| s.resume(&id, on, anchor).map(drop))
}

#[tauri::command]
fn cancel(app: AppHandle, id: String, ends_on: NaiveDate) -> Res<()> {
    write(&app, |s| s.cancel(&id, ends_on).map(drop))
}

#[tauri::command]
fn restart(app: AppHandle, id: String, on: NaiveDate, terms: Option<Terms>) -> Res<()> {
    write(&app, |s| s.restart(&id, on, terms.as_ref()).map(drop))
}

#[tauri::command]
fn convert_trial(app: AppHandle, id: String, on: NaiveDate, terms: Option<Terms>) -> Res<()> {
    write(&app, |s| s.convert_trial(&id, on, terms.as_ref()).map(drop))
}

#[tauri::command]
fn set_archived(app: AppHandle, id: String, archived: bool) -> Res<()> {
    write(&app, |s| s.set_archived(&id, archived))
}

#[tauri::command]
fn delete_subscription(app: AppHandle, id: String) -> Res<()> {
    write(&app, |s| s.delete_subscription(&id))
}

#[tauri::command]
fn restore_subscription(app: AppHandle, id: String) -> Res<()> {
    write(&app, |s| s.restore_subscription(&id))
}

#[tauri::command]
fn set_override(app: AppHandle, o: NewOverride) -> Res<ChargeOverride> {
    write(&app, |s| s.set_override(&o))
}

#[tauri::command]
fn delete_override(app: AppHandle, id: String) -> Res<()> {
    write(&app, |s| s.delete_override(&id))
}

#[tauri::command]
fn add_extra(app: AppHandle, e: NewExtraCharge) -> Res<ExtraCharge> {
    write(&app, |s| s.add_extra(&e))
}

#[tauri::command]
fn update_extra(app: AppHandle, e: ExtraCharge) -> Res<()> {
    write(&app, |s| s.update_extra(&e))
}

#[tauri::command]
fn delete_extra(app: AppHandle, id: String) -> Res<()> {
    write(&app, |s| s.delete_extra(&id))
}

#[tauri::command]
fn restore_extra(app: AppHandle, id: String) -> Res<()> {
    write(&app, |s| s.restore_extra(&id))
}

#[tauri::command]
fn add_category(db: State<Db>, name: String, color: String) -> Res<Category> {
    with(&db, |s| s.add_category(&name, &color))
}

#[tauri::command]
fn update_category(app: AppHandle, cat: Category) -> Res<()> {
    write(&app, |s| s.update_category(&cat))
}

/// → the payments that were in it (for undo).
#[tauri::command]
fn delete_category(app: AppHandle, id: String) -> Res<Vec<String>> {
    write(&app, |s| s.delete_category(&id))
}

#[tauri::command]
fn restore_category(app: AppHandle, id: String, subs: Vec<String>) -> Res<()> {
    write(&app, |s| s.restore_category(&id, &subs))
}

#[tauri::command]
fn sort_categories(app: AppHandle, ids: Vec<String>) -> Res<()> {
    write(&app, |s| s.sort_categories(&ids))
}

#[tauri::command]
fn rename_payment_method(app: AppHandle, id: String, name: String) -> Res<()> {
    write(&app, |s| s.rename_payment_method(&id, &name))
}

/// → the payments that used it (for undo).
#[tauri::command]
fn delete_payment_method(app: AppHandle, id: String) -> Res<Vec<String>> {
    write(&app, |s| s.delete_payment_method(&id))
}

#[tauri::command]
fn restore_payment_method(app: AppHandle, id: String, subs: Vec<String>) -> Res<()> {
    write(&app, |s| s.restore_payment_method(&id, &subs))
}

#[tauri::command]
fn add_payment_method(db: State<Db>, name: String) -> Res<PaymentMethod> {
    with(&db, |s| s.add_payment_method(&name))
}

/// A price's cost a month in the main currency (Quick add's budget note).
#[tauri::command]
fn monthly_in_main(db: State<Db>, amount_minor: i64, currency: String, every_n: u32, unit: Unit) -> Res<Option<i64>> {
    with(&db, |s| s.monthly_in_main(amount_minor, &currency, every_n, unit))
}

#[tauri::command]
fn update_setting(app: AppHandle, key: String, value: serde_json::Value) -> Res<()> {
    write(&app, |s| s.update_setting(&key, &value))
}

// ---- Reminders ----

#[tauri::command]
fn snooze_picks() -> Vec<SnoozePick> {
    snooze_picks_at(now_ms(), &local_tz())
}

/// Show the subscription's reminder again at `until_ms`.
#[tauri::command]
fn snooze(app: AppHandle, id: String, until_ms: i64) -> Res<()> {
    write(&app, |s| s.snooze_reminder(&id, until_ms, now_ms(), &local_tz()).map(drop))
}

/// Reminder time of day, "HH:MM", as UTC ms on `day` here (snooze to a day).
#[tauri::command]
fn reminder_at(db: State<Db>, day: NaiveDate) -> Res<i64> {
    with(&db, |s| {
        let t = chrono::NaiveTime::parse_from_str(&s.reminder_time()?, "%H:%M").unwrap_or_default();
        Ok(hisaab_core::reminders::local_ms(day, t, hisaab_core::reminders::parse_tz(&local_tz())))
    })
}

// ---- Desktop startup (this computer only) ----

#[cfg(desktop)]
#[tauri::command]
fn desktop_prefs(app: AppHandle) -> desktop::prefs::DesktopPrefs {
    desktop::prefs::get(&app)
}

#[cfg(desktop)]
#[tauri::command]
fn set_autostart(app: AppHandle, on: bool) -> Res<()> {
    desktop::prefs::set_autostart(&app, on)
}

#[cfg(desktop)]
#[tauri::command]
fn set_start_hidden(app: AppHandle, on: bool) -> Res<()> {
    desktop::prefs::set_start_hidden(&app, on)
}

#[cfg(mobile)]
const DESKTOP_ONLY: &str = "desktop only";

#[cfg(mobile)]
#[tauri::command]
fn desktop_prefs() -> Res<()> {
    Err(DESKTOP_ONLY.into())
}

#[cfg(mobile)]
#[tauri::command]
fn set_autostart() -> Res<()> {
    Err(DESKTOP_ONLY.into())
}

#[cfg(mobile)]
#[tauri::command]
fn set_start_hidden() -> Res<()> {
    Err(DESKTOP_ONLY.into())
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
    Err(DESKTOP_ONLY.into())
}

/// Replace all data with a backup file (path from the native file picker).
/// The current data is snapshotted to the cache first; reminders are re-armed
/// and paired devices get everything again.
#[tauri::command]
fn import_backup(app: AppHandle, db: State<Db>, path: String) -> Res<()> {
    {
        let mut s = db.store.lock().map_err(err)?;
        if let Ok(dir) = app.path().app_cache_dir() {
            let _ = std::fs::create_dir_all(&dir);
            let _ = s.export_to(&dir.join("hisaab-before-import.db").to_string_lossy());
        }
        s.import_from(&path).map_err(err)?;
    }
    let _ = std::fs::remove_file(&path);
    rearm(&app);
    sync::nudge(&app);
    Ok(())
}

// ---- Backups on this computer (Android: the reminders plugin's commands).
// Dialogs block, so these run off the main thread (async).

#[cfg(desktop)]
#[tauri::command]
fn backup_status(app: AppHandle) -> desktop::backup::BackupStatus {
    desktop::backup::status(&app)
}

#[cfg(desktop)]
#[tauri::command]
async fn pick_backup_folder(app: AppHandle) -> Res<desktop::backup::BackupStatus> {
    desktop::backup::pick_folder(&app)
}

#[cfg(desktop)]
#[tauri::command]
fn clear_backup_folder(app: AppHandle) -> Res<desktop::backup::BackupStatus> {
    desktop::backup::clear_folder(&app)
}

#[cfg(desktop)]
#[tauri::command]
async fn backup_now(app: AppHandle) -> Res<desktop::backup::BackupStatus> {
    desktop::backup::run_now(&app)
}

#[cfg(desktop)]
#[tauri::command]
async fn export_backup(app: AppHandle) -> Res<serde_json::Value> {
    Ok(serde_json::json!({ "saved": desktop::backup::export(&app)? }))
}

#[cfg(desktop)]
#[tauri::command]
async fn pick_import_file(app: AppHandle) -> Res<serde_json::Value> {
    Ok(serde_json::json!({ "path": desktop::backup::pick_import(&app)? }))
}

/// A web page (a payment's site or cancel page) in the default browser.
/// Android: the reminders plugin's `open_url`.
#[cfg(desktop)]
#[tauri::command]
fn open_url(url: String) -> Res<()> {
    if !(url.starts_with("https://") || url.starts_with("http://")) || url.chars().any(char::is_whitespace) {
        return Err("not a web link".into());
    }
    #[cfg(windows)]
    let mut cmd = std::process::Command::new("explorer");
    #[cfg(not(windows))]
    let mut cmd = std::process::Command::new("xdg-open");
    cmd.arg(&url).spawn().map(|_| ()).map_err(err)
}

#[cfg(mobile)]
#[tauri::command]
fn open_url() -> Res<()> {
    Err(DESKTOP_ONLY.into())
}

#[cfg(mobile)]
#[tauri::command]
fn backup_status() -> Res<()> {
    Err(DESKTOP_ONLY.into())
}

#[cfg(mobile)]
#[tauri::command]
fn pick_backup_folder() -> Res<()> {
    Err(DESKTOP_ONLY.into())
}

#[cfg(mobile)]
#[tauri::command]
fn clear_backup_folder() -> Res<()> {
    Err(DESKTOP_ONLY.into())
}

#[cfg(mobile)]
#[tauri::command]
fn backup_now() -> Res<()> {
    Err(DESKTOP_ONLY.into())
}

#[cfg(mobile)]
#[tauri::command]
fn export_backup() -> Res<()> {
    Err(DESKTOP_ONLY.into())
}

#[cfg(mobile)]
#[tauri::command]
fn pick_import_file() -> Res<()> {
    Err(DESKTOP_ONLY.into())
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
        .plugin(tauri_plugin_autostart::init(tauri_plugin_autostart::MacosLauncher::LaunchAgent, Some(vec!["--autostart"])))
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .on_window_event(desktop::on_window_event);
    builder
        .plugin(tauri_plugin_reminders::init())
        .setup(|app| {
            let dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&dir)?;
            let path = dir.join("hisaab.db").to_string_lossy().into_owned();
            let store = Store::open(&path)?;
            app.manage(Db { store: Mutex::new(store), path });
            #[cfg(desktop)]
            {
                app.manage(desktop::Scheduler::start(app.handle().clone()));
                desktop::setup(app.handle());
            }
            rearm(app.handle());
            sync::setup(app.handle())?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            overview,
            timeline,
            archived,
            search,
            stats,
            month_charges,
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
            add_extra,
            update_extra,
            delete_extra,
            restore_extra,
            add_category,
            add_payment_method,
            update_category,
            delete_category,
            restore_category,
            sort_categories,
            rename_payment_method,
            delete_payment_method,
            restore_payment_method,
            update_setting,
            monthly_in_main,
            refresh_rates,
            ensure_rates,
            snooze_picks,
            snooze,
            reminder_at,
            desktop_prefs,
            set_autostart,
            set_start_hidden,
            install_update,
            import_backup,
            open_url,
            backup_status,
            pick_backup_folder,
            clear_backup_folder,
            backup_now,
            export_backup,
            pick_import_file,
            sync::sync_status,
            sync::set_sync_mode,
            sync::set_device_name,
            sync::start_pairing,
            sync::stop_pairing,
            sync::pair_with_link,
            sync::unpair,
            sync::sync_now,
            sync::conflicts,
            sync::resolve_conflict,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Hisaab");
}
