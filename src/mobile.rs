use serde::{de::DeserializeOwned, Serialize};
use tauri::{
  plugin::{PluginApi, PluginHandle},
  AppHandle, Runtime,
};

use crate::models::*;

#[cfg(target_os = "ios")]
tauri::ios_plugin_binding!(init_plugin_cache);

// initializes the Kotlin or Swift plugin classes
pub fn init<R: Runtime, C: DeserializeOwned>(
  _app: &AppHandle<R>,
  api: PluginApi<R, C>,
) -> crate::Result<Cache<R>> {
  #[cfg(target_os = "android")]
  let handle = api.register_android_plugin("app.tauri.plugin.cache", "CachePlugin")?;
  #[cfg(target_os = "ios")]
  let handle = api.register_ios_plugin(init_plugin_cache)?;
  Ok(Cache(handle))
}

/// Initializes the plugin with custom configuration
pub fn init_with_config<R: Runtime, C: DeserializeOwned>(
  _app: &AppHandle<R>,
  api: PluginApi<R, C>,
  config: CacheConfig
) -> crate::Result<Cache<R>> {
  // Register the plugin with API
  #[cfg(target_os = "android")]
  let handle = {
    // Pass configuration to Android
    let config_json = serde_json::to_string(&config).unwrap_or_default();
    api.register_android_plugin_with_config("app.tauri.plugin.cache", "CachePlugin", config_json)?
  };
  
  #[cfg(target_os = "ios")]
  let handle = {
    // Pass configuration to iOS
    let config_json = serde_json::to_string(&config).unwrap_or_default();
    api.register_ios_plugin_with_config(init_plugin_cache, config_json)?
  };
  
  Ok(Cache(handle))
}

/// Access to the cache APIs.
pub struct Cache<R: Runtime>(PluginHandle<R>);

impl<R: Runtime> Cache<R> {
  /// Sets a value in the cache with optional TTL
  pub fn set<T: Serialize>(&self, key: String, value: T, options: Option<SetItemOptions>) -> crate::Result<EmptyResponse> {
    let request = SetRequest::<T> {
      key,
      value,
      options,
    };
    self
      .0
      .run_mobile_plugin("set", request)
      .map_err(|e| crate::Error::PluginInvoke(e))
  }

  /// Gets a value from the cache
  pub fn get(&self, key: &str) -> crate::Result<Option<serde_json::Value>> {
    let request = GetRequest {
      key: key.to_string(),
    };
    self
      .0
      .run_mobile_plugin::<_, Option<serde_json::Value>>("get", request)
      .map_err(|e| crate::Error::PluginInvoke(e))
  }

  /// Checks if a key exists in the cache
  pub fn has(&self, key: &str) -> crate::Result<BooleanResponse> {
    let request = HasRequest {
      key: key.to_string(),
    };
    self
      .0
      .run_mobile_plugin("has", request)
      .map_err(|e| crate::Error::PluginInvoke(e))
  }

  /// Removes a value from the cache
  pub fn remove(&self, key: &str) -> crate::Result<EmptyResponse> {
    let request = RemoveRequest {
      key: key.to_string(),
    };
    self
      .0
      .run_mobile_plugin("remove", request)
      .map_err(|e| crate::Error::PluginInvoke(e))
  }

  /// Clears all values from the cache
  pub fn clear(&self) -> crate::Result<EmptyResponse> {
    self
      .0
      .run_mobile_plugin::<_, EmptyResponse>("clear", ())
      .map_err(|e| crate::Error::PluginInvoke(e))
  }
  
  /// Get cache statistics
  pub fn stats(&self) -> crate::Result<CacheStats> {
    self
      .0
      .run_mobile_plugin::<_, CacheStats>("stats", ())
      .map_err(|e| crate::Error::PluginInvoke(e))
  }
}