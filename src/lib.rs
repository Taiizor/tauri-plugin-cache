// Copyright 2025 Taiizor
// All Rights Reserved
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
  /// Access the cache APIs.
  fn cache(&self) -> &Cache<R>;
}

impl<R: Runtime, T: Manager<R>> CacheExt<R> for T {
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
      commands::haskey,
      commands::remove,
      commands::clear,
      commands::keys,
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