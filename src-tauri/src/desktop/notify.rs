//! Reminder notifications (Befikr's, with Open and Snooze): tap or Open
//! brings the window up on the subscription, Snooze on its snooze sheet.
//! Each notification waits for its answer on its own thread.

use hisaab_core::Reminder;
use notify_rust::{Notification, NotificationResponse, Timeout};
use tauri::AppHandle;

pub fn show(app: &AppHandle, r: &Reminder) {
    let mut n = Notification::new();
    n.appname("Hisaab").summary(&r.title).body(&r.text);
    n.action("default", "Open").action("open", "Open").action("snooze", "Snooze");
    n.timeout(Timeout::Never);
    // Installed builds have a Start menu entry under the app identifier;
    // dev builds borrow PowerShell's.
    #[cfg(windows)]
    if !cfg!(debug_assertions) {
        n.app_id(&app.config().identifier);
    }
    let handle = match n.show() {
        Ok(h) => h,
        Err(e) => return eprintln!("notification failed: {e}"),
    };
    let (app, route) = (app.clone(), r.route.clone());
    std::thread::spawn(move || {
        let _ = handle.wait_for_response(|resp: &NotificationResponse| match resp {
            NotificationResponse::Action(a) if a == "snooze" => super::open_route(&app, &format!("{route}?snooze=1")),
            NotificationResponse::Action(_) => super::open_route(&app, &route),
            _ => {}
        });
    });
}
