//! Reminder notifications (Befikr's, with Open and Snooze): tap or Open
//! brings the window up on the subscription, Snooze on its snooze sheet.
//! A charge's reminder also has Pay now: on Windows it's paid right there,
//! then "Paid today" with Undo; on Linux the window opens on the Pay sheet
//! (actions that don't open a window are unreliable across notification
//! daemons). Each notification waits for its answer on its own thread.

use hisaab_core::{Paid, Reminder};
use notify_rust::{Notification, NotificationHandle, NotificationResponse, Timeout};
use tauri::{AppHandle, Emitter};

/// How long "Paid today" stays up with its Undo.
const PAID_UNDO_MS: u32 = 10_000;

pub fn show(app: &AppHandle, r: &Reminder) {
    let mut n = notification(app);
    n.summary(&r.title).body(&r.text);
    n.action("default", "Open");
    if r.charge.is_some() {
        n.action("pay", "Pay now");
    }
    n.action("snooze", "Snooze").action("open", "Open");
    n.timeout(Timeout::Never);
    let Some(handle) = present(&n) else { return };
    let (app, r) = (app.clone(), r.clone());
    std::thread::spawn(move || {
        let _ = handle.wait_for_response(|resp: &NotificationResponse| match resp {
            NotificationResponse::Action(a) if a == "snooze" => super::open_route(&app, &format!("{}?snooze=1", r.route)),
            NotificationResponse::Action(a) if a == "pay" => pay(&app, &r),
            NotificationResponse::Action(_) => super::open_route(&app, &r.route),
            _ => {}
        });
    });
}

fn pay(app: &AppHandle, r: &Reminder) {
    let Some(c) = &r.charge else { return };
    if !cfg!(windows) {
        return super::open_route(app, &format!("{}?pay={}&due={}", r.route, c.period_id, c.scheduled_date));
    }
    match crate::pay_charge(app, &r.subscription_id, c) {
        Ok(paid) => {
            let _ = app.emit("data-changed", ());
            show_paid(app, r, paid);
        }
        Err(e) => eprintln!("pay now: {e}"),
    }
}

/// After Pay now: what was paid, and Undo (when something changed) puts
/// the charge and its reminder back.
fn show_paid(app: &AppHandle, r: &Reminder, paid: Paid) {
    let mut n = notification(app);
    n.summary(&paid.label);
    if paid.undo.is_some() {
        n.action("undo", "Undo");
    }
    n.timeout(Timeout::Milliseconds(PAID_UNDO_MS));
    let Some(handle) = present(&n) else { return };
    let Some(undo) = paid.undo else { return };
    let (app, r) = (app.clone(), r.clone());
    std::thread::spawn(move || {
        let _ = handle.wait_for_response(|resp: &NotificationResponse| {
            if matches!(resp, NotificationResponse::Action(a) if a == "undo") {
                match crate::undo_charge(&app, &r.subscription_id, &undo) {
                    Ok(()) => {
                        let _ = app.emit("data-changed", ());
                        show(&app, &r);
                    }
                    Err(e) => eprintln!("undo pay: {e}"),
                }
            }
        });
    });
}

fn notification(app: &AppHandle) -> Notification {
    let mut n = Notification::new();
    n.appname("Hisaab");
    // Installed builds have a Start menu entry under the app identifier;
    // dev builds borrow PowerShell's.
    #[cfg(windows)]
    if !cfg!(debug_assertions) {
        n.app_id(&app.config().identifier);
    }
    #[cfg(not(windows))]
    let _ = app;
    n
}

fn present(n: &Notification) -> Option<NotificationHandle> {
    n.show().map_err(|e| eprintln!("notification failed: {e}")).ok()
}
