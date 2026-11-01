const COMMANDS: &[&str] = &[
    "install_update",
    "move_to_back",
    // addPluginListener (update download progress)
    "register_listener",
    "remove_listener",
];

fn main() {
    tauri_plugin::Builder::new(COMMANDS)
        .android_path("android")
        .build();
}
