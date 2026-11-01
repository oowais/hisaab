#[cfg(desktop)]
mod desktop;

use std::sync::Mutex;

use hisaab_core::Store;
use tauri::{AppHandle, Manager};

pub(crate) struct Db {
    #[allow(dead_code)] // read by the commands of milestone 2
    pub(crate) store: Mutex<Store>,
}

type Res<T> = Result<T, String>;

fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

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
        .invoke_handler(tauri::generate_handler![install_update])
        .run(tauri::generate_context!())
        .expect("error while running Hisaab");
}
