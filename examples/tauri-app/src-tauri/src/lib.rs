// Learn more about Tauri commands at https://v2.tauri.app/develop/calling-rust/#commands
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let cache_config = tauri_plugin_cache::CacheConfig {
        cache_dir: Some("my_app_cache".into()),
        cache_file_name: Some("cache_data.json".into()),
        cleanup_interval: Some(120),
        default_compression: Some(true),
    };
    
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![greet])
        //.plugin(tauri_plugin_cache::init())
        .plugin(tauri_plugin_cache::init_with_config(cache_config))
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}