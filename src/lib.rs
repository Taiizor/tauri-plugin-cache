use tauri::{
    plugin::{Builder, TauriPlugin},
    Manager, Runtime,
};

pub use models::*;

#[cfg(desktop)]
mod desktop;
#[cfg(mobile)]
mod mobile;

mod commands;
mod error;
mod models;

pub use error::{Error, Result};

#[cfg(desktop)]
use desktop::Cache;
#[cfg(mobile)]
use mobile::Cache;

#[cfg(desktop)]
pub use desktop::CompressionConfig;

/// Extensions to [`tauri::App`], [`tauri::AppHandle`] and [`tauri::Window`] to access the cache APIs.
pub trait CacheExt<R: Runtime> {
    fn cache(&self) -> &Cache<R>;
}

impl<R: Runtime, T: Manager<R>> crate::CacheExt<R> for T {
    fn cache(&self) -> &Cache<R> {
        self.state::<Cache<R>>().inner()
    }
}

/// Initializes the plugin.
pub fn init<R: Runtime>() -> TauriPlugin<R> {
    // Default config
    let config = CacheConfig::default();
    init_with_config(config)
}

/// Initializes the plugin with custom configuration.
pub fn init_with_config<R: Runtime>(config: CacheConfig) -> TauriPlugin<R> {
    // Clone config for use in the closure
    let config_clone = config.clone();

    Builder::new("cache")
        .invoke_handler(tauri::generate_handler![
            commands::set,
            commands::get,
            commands::has,
            commands::remove,
            commands::clear,
            commands::stats
        ])
        .setup(move |app, api| {
            // Provide the config manually to the desktop implementation
            #[cfg(desktop)]
            let cache = {
                // Always start from app's cache directory
                let base_cache_dir = app.path().app_cache_dir().map_err(|e| {
                    crate::Error::Cache(format!("Failed to get app cache directory: {}", e))
                })?;

                // If custom subdirectory is specified, append it to the app cache directory path
                let cache_dir = if let Some(custom_dir) = config_clone.cache_dir.as_deref() {
                    let custom_path = std::path::PathBuf::from(custom_dir);
                    if custom_path.is_absolute() {
                        // Instead of absolute path, take only the last component
                        let path_components: Vec<_> = custom_path
                            .components()
                            .filter(|c| !c.as_os_str().is_empty())
                            .collect();

                        if let Some(last_component) = path_components.last() {
                            base_cache_dir.join(last_component.as_os_str())
                        } else {
                            base_cache_dir
                        }
                    } else {
                        // Add as a relative path
                        base_cache_dir.join(custom_dir)
                    }
                } else {
                    base_cache_dir
                };

                // Create the cache directory if it doesn't exist
                std::fs::create_dir_all(&cache_dir).map_err(|e| {
                    crate::Error::Cache(format!("Failed to create cache directory: {}", e))
                })?;

                // Determine the cache file name
                let cache_file_name = config_clone
                    .cache_file_name
                    .as_deref()
                    .unwrap_or("tauri_cache.json");
                let cache_file_path = cache_dir.join(cache_file_name);

                // Get the default compression settings
                let default_compression = config_clone.default_compression.unwrap_or(false);
                let compression_level = config_clone.compression_level;
                let compression_threshold = config_clone.compression_threshold;

                // Initialize the cache with cleanup interval
                let mut cache = desktop::init_with_config(
                    app,
                    api,
                    cache_file_path,
                    config_clone.cleanup_interval.unwrap_or(60),
                )?;
                
                // Initialize with compression settings
                cache.init_with_config(default_compression, compression_level, compression_threshold);
                cache
            };

            #[cfg(mobile)]
            let cache = {
                // Always start from app's cache directory
                let base_cache_dir = app.path().app_cache_dir().map_err(|e| {
                    crate::Error::Cache(format!("Failed to get app cache directory: {}", e))
                })?;

                // If custom subdirectory is specified, append it to the app cache directory path
                let cache_dir = if let Some(custom_dir) = config_clone.cache_dir.as_deref() {
                    let custom_path = std::path::PathBuf::from(custom_dir);
                    if custom_path.is_absolute() {
                        // Instead of absolute path, take only the last component
                        let path_components: Vec<_> = custom_path
                            .components()
                            .filter(|c| !c.as_os_str().is_empty())
                            .collect();

                        if let Some(last_component) = path_components.last() {
                            base_cache_dir.join(last_component.as_os_str())
                        } else {
                            base_cache_dir
                        }
                    } else {
                        // Add as a relative path
                        base_cache_dir.join(custom_dir)
                    }
                } else {
                    base_cache_dir
                };

                // Create the cache directory if it doesn't exist
                std::fs::create_dir_all(&cache_dir).map_err(|e| {
                    crate::Error::Cache(format!("Failed to create cache directory: {}", e))
                })?;

                // Determine the cache file name
                let cache_file_name = config_clone
                    .cache_file_name
                    .as_deref()
                    .unwrap_or("tauri_cache.json");
                let cache_file_path = cache_dir.join(cache_file_name);

                mobile::init_with_config(
                    app,
                    api,
                    cache_file_path,
                    config_clone.cleanup_interval.unwrap_or(60),
                )?
            };

            app.manage(cache);
            Ok(())
        })
        .build()
}
