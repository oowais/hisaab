const COMMANDS: &[&str] = &[
    "arm",
    "status",
    "request_notifications",
    "open_exact_alarm_settings",
    "open_battery_settings",
    "take_launch_route",
    "install_update",
    "move_to_back",
    "open_url",
    "backup_status",
    "pick_backup_folder",
    "clear_backup_folder",
    "backup_now",
    "export_backup",
    "pick_import_file",
    // addPluginListener (update download progress, launch route)
    "register_listener",
    "remove_listener",
];

fn main() {
    tauri_plugin::Builder::new(COMMANDS)
        .android_path("android")
        .build();
}
