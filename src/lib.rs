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
  Builder::new("cache")
    .invoke_handler(tauri::generate_handler![
      commands::set,
      commands::get,
      commands::has,
      commands::remove,
      commands::clear,
      commands::stats
    ])
    .setup(|app, api| {
      #[cfg(mobile)]
      let cache = mobile::init(app, api)?;
      #[cfg(desktop)]
      let cache = desktop::init(app, api)?;
      app.manage(cache);
      Ok(())
    })
    .build()
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
        let cache_dir = if let Some(custom_dir) = config_clone.cache_dir.as_deref() {
          std::path::PathBuf::from(custom_dir)
        } else {
          app.path().app_cache_dir()
            .map_err(|e| crate::Error::Cache(format!("Failed to get app cache directory: {}", e)))?
        };
        
        // Create the cache directory if it doesn't exist
        std::fs::create_dir_all(&cache_dir)
          .map_err(|e| crate::Error::Cache(format!("Failed to create cache directory: {}", e)))?;
        
        // Determine the cache file name
        let cache_file_name = config_clone.cache_file_name.as_deref().unwrap_or("tauri_cache.json");
        let cache_file_path = cache_dir.join(cache_file_name);
        
        desktop::init_with_config(app, api, cache_file_path, config_clone.cleanup_interval.unwrap_or(60))?
      };
      
      #[cfg(mobile)]
      let cache = mobile::init(app, api)?;
      
      app.manage(cache);
      Ok(())
    })
    .build()
}