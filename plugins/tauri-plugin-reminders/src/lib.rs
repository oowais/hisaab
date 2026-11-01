//! Native Android side of Hisaab: in-app updates and the back button for
//! now; charge reminders (exact alarms, notification actions) come in
//! milestone 5. Rust owns the data; Kotlin calls back into Rust via JNI.

use tauri::{
    plugin::{Builder, TauriPlugin},
    Manager, Runtime,
};

#[cfg(target_os = "android")]
pub struct Reminders<R: Runtime>(#[allow(dead_code)] tauri::plugin::PluginHandle<R>);
#[cfg(not(target_os = "android"))]
pub struct Reminders<R: Runtime>(std::marker::PhantomData<fn() -> R>);

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
