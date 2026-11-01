//! Native Android side of Hisaab: reminder alarms and notifications (with a
//! snooze overlay), permissions, in-app updates, the back button. Rust owns
//! the data and decides what to remind; Kotlin calls back into Rust via JNI
//! (src-tauri/src/jni.rs) when an alarm fires with no webview.

use tauri::{
    plugin::{Builder, TauriPlugin},
    Manager, Runtime,
};

#[cfg(target_os = "android")]
pub struct Reminders<R: Runtime>(tauri::plugin::PluginHandle<R>);
#[cfg(not(target_os = "android"))]
pub struct Reminders<R: Runtime>(std::marker::PhantomData<fn() -> R>);

impl<R: Runtime> Reminders<R> {
    /// Arm exactly `reminders` (core `Reminder`s, serialized); cancel the rest.
    /// Desktop: nothing to do here (the app's own scheduler arms them).
    pub fn arm(&self, db_path: &str, reminders: &serde_json::Value) -> Result<(), String> {
        #[cfg(target_os = "android")]
        {
            self.0
                .run_mobile_plugin::<serde_json::Value>(
                    "arm",
                    serde_json::json!({ "dbPath": db_path, "reminders": reminders.to_string() }),
                )
                .map(|_| ())
                .map_err(|e| e.to_string())
        }
        #[cfg(not(target_os = "android"))]
        {
            let _ = (db_path, reminders);
            Ok(())
        }
    }
}

pub trait RemindersExt<R: Runtime> {
    fn reminders(&self) -> &Reminders<R>;
}

impl<R: Runtime, T: Manager<R>> RemindersExt<R> for T {
    fn reminders(&self) -> &Reminders<R> {
        self.state::<Reminders<R>>().inner()
    }
}

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("reminders")
        .setup(|app, api| {
            #[cfg(target_os = "android")]
            let reminders =
                Reminders(api.register_android_plugin("com.hisaab.reminders", "RemindersPlugin")?);
            #[cfg(not(target_os = "android"))]
            let reminders = {
                let _ = api;
                Reminders::<R>(std::marker::PhantomData)
            };
            app.manage(reminders);
            Ok(())
        })
        .build()
}
