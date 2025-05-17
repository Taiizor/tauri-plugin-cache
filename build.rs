const COMMANDS: &[&str] = &["set", "get", "has_key", "remove", "clear", "keys"];

fn main() {
  tauri_plugin::Builder::new(COMMANDS)
    .android_path("android")
    .ios_path("ios")
    .build();
}
