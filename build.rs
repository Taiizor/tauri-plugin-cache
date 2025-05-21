const COMMANDS: &[&str] = &["set", "get", "has", "remove", "clear", "stats"];

fn main() {
    tauri_plugin::Builder::new(COMMANDS)
        .android_path("android")
        .ios_path("ios")
        .build();
}
