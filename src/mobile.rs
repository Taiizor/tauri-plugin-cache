use serde::{de::DeserializeOwned, Serialize};
use std::path::PathBuf;
use tauri::{
    plugin::{PluginApi, PluginHandle},
    AppHandle, Runtime,
};

use crate::models::*;
use crate::Error;

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

// Initialize the plugin with a custom cache file path
pub fn init_with_config<R: Runtime, C: DeserializeOwned>(
    _app: &AppHandle<R>,
    api: PluginApi<R, C>,
    cache_file_path: PathBuf,
    cleanup_interval: u64,
) -> crate::Result<Cache<R>> {
    // Create config for mobile platforms
    let config = CacheConfig {
        cache_dir: cache_file_path
            .parent()
            .map(|p| p.to_string_lossy().to_string()),
        cache_file_name: cache_file_path
            .file_name()
            .map(|f| f.to_string_lossy().to_string()),
        cleanup_interval: Some(cleanup_interval),
        default_compression: Some(true),
        compression_level: Some(6),
        compression_threshold: Some(crate::models::COMPRESSION_THRESHOLD),
        compression_method: Some(CompressionMethod::Zlib),
    };

    // Register the plugin with API
    #[cfg(target_os = "android")]
    let handle = {
        // Pass configuration to Android
        let config_json = serde_json::to_string(&config)
            .map_err(|e| Error::Cache(format!("Failed to serialize config: {}", e)))?;

        // Register the plugin
        let handle = api
            .register_android_plugin("app.tauri.plugin.cache", "CachePlugin")
            .map_err(|e| Error::Cache(format!("Failed to register Android plugin: {}", e)))?;

        // If registration successful, send configuration through method call
        if let Err(e) = handle.run_mobile_plugin::<String>("configure", config_json) {
            // Log the error but continue
            eprintln!("Warning: Failed to configure Android cache plugin: {}", e);
        }

        handle
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
    /// Configure the cache with compression settings
    pub fn init_with_config(
        &mut self,
        default_compression: bool,
        compression_level: Option<u32>,
        compression_threshold: Option<usize>,
        compression_method: Option<CompressionMethod>,
    ) {
        // Update compression settings on mobile side
        // Let's update the config to send these settings to the native side
        let config = CompressionConfig {
            enabled: default_compression,
            level: compression_level.unwrap_or(6),
            threshold: compression_threshold.unwrap_or(crate::models::COMPRESSION_THRESHOLD),
            method: compression_method.unwrap_or(CompressionMethod::Zlib),
        };

        // Send configuration to native side
        let _ = self
            .0
            .run_mobile_plugin::<EmptyResponse>("updateCompressionConfig", config);
        // Error is ignored because this feature might not exist in older versions of mobile plugins
        // Error handling should be added in a real application
    }

    /// Sets a value in the cache with optional TTL
    pub fn set<T: Serialize + DeserializeOwned + std::fmt::Debug>(
        &self,
        key: String,
        value: T,
        options: Option<SetItemOptions>,
    ) -> crate::Result<EmptyResponse> {
        let request = SetRequest::<T> {
            key,
            value,
            options,
        };
        self.0
            .run_mobile_plugin::<EmptyResponse>("set", request)
            .map_err(|e| crate::Error::PluginInvoke(e))
    }

    /// Gets a value from the cache
    pub fn get(&self, key: &str) -> crate::Result<Option<serde_json::Value>> {
        let request = GetRequest {
            key: key.to_string(),
        };
        self.0
            .run_mobile_plugin::<Option<serde_json::Value>>("get", request)
            .map_err(|e| crate::Error::PluginInvoke(e))
    }

    /// Checks if a key exists in the cache
    pub fn has(&self, key: &str) -> crate::Result<BooleanResponse> {
        let request = HasRequest {
            key: key.to_string(),
        };
        self.0
            .run_mobile_plugin::<BooleanResponse>("has", request)
            .map_err(|e| crate::Error::PluginInvoke(e))
    }

    /// Removes a value from the cache
    pub fn remove(&self, key: &str) -> crate::Result<EmptyResponse> {
        let request = RemoveRequest {
            key: key.to_string(),
        };
        self.0
            .run_mobile_plugin::<EmptyResponse>("remove", request)
            .map_err(|e| crate::Error::PluginInvoke(e))
    }

    /// Clears all values from the cache
    pub fn clear(&self) -> crate::Result<EmptyResponse> {
        self.0
            .run_mobile_plugin::<EmptyResponse>("clear", ())
            .map_err(|e| crate::Error::PluginInvoke(e))
    }

    /// Get cache statistics
    pub fn stats(&self) -> crate::Result<CacheStats> {
        self.0
            .run_mobile_plugin::<CacheStats>("stats", ())
            .map_err(|e| crate::Error::PluginInvoke(e))
    }
}
