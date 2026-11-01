const COMMANDS: &[&str] = &[
    "arm",
    "status",
    "request_notifications",
    "open_exact_alarm_settings",
    "open_battery_settings",
    "take_launch_route",
    "install_update",
    "move_to_back",
    // addPluginListener (update download progress, launch route)
    "register_listener",
    "remove_listener",
];

fn main() {
    tauri_plugin::Builder::new(COMMANDS)
        .android_path("android")
        .build();
}
