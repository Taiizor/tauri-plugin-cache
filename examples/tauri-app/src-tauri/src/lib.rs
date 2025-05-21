// Learn more about Tauri commands at https://v2.tauri.app/develop/calling-rust/#commands
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let cache_config = tauri_plugin_cache::CacheConfig {
        cache_dir: Some("/custom/cache/path".into()),
        cache_file_name: Some("my_app_cache.json".into()),
        cleanup_interval: Some(120),
    };
    
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![greet])
        .plugin(tauri_plugin_cache::init_with_config(cache_config))
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}