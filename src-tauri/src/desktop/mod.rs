//! Desktop (Linux, Windows): window handling and the signed in-app updater.
//! Reminders (a scheduler thread + tray) come in milestone 5. Android uses
//! the Kotlin plugin instead.

use tauri::{AppHandle, Manager};

/// Show, unminimize and focus the main window.
pub fn show_main(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}
