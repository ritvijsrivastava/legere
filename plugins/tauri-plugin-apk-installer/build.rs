const COMMANDS: &[&str] = &["can_request_installs", "request_install_permission", "install"];

fn main() {
  tauri_plugin::Builder::new(COMMANDS)
    .android_path("android")
    .build();
}
