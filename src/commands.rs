use serde::{Deserialize, Serialize};
use tauri::{AppHandle, command, Runtime};

use crate::models::*;
use crate::Result;
use crate::CacheExt;

/// Set a value in the cache with optional TTL
#[command]
pub(crate) async fn set<R: Runtime>(
    app: AppHandle<R>,
    key: String,
    value: serde_json::Value,
    options: Option<SetItemOptions>,
) -> Result<EmptyResponse> {
    app.cache().set(key, value, options)
}

/// Get a value from the cache by key
#[command]
pub(crate) async fn get<R: Runtime>(
    app: AppHandle<R>,
    key: String,
) -> Result<Option<serde_json::Value>> {
    app.cache().get(&key)
}

/// Check if a key exists in the cache and is not expired
#[command]
pub(crate) async fn has<R: Runtime>(
    app: AppHandle<R>,
    key: String,
) -> Result<BooleanResponse> {
    app.cache().has(&key)
}

/// Remove a value from the cache
#[command]
pub(crate) async fn remove<R: Runtime>(
    app: AppHandle<R>,
    key: String,
) -> Result<EmptyResponse> {
    app.cache().remove(&key)
}

/// Clear all values from the cache
#[command]
pub(crate) async fn clear<R: Runtime>(
    app: AppHandle<R>,
) -> Result<EmptyResponse> {
    app.cache().clear()
}

/// Get cache statistics
#[command]
pub(crate) async fn stats<R: Runtime>(
    app: AppHandle<R>,
) -> Result<CacheStats> {
    #[cfg(desktop)]
    {
        let total_size = app.cache().size()?;
        let active_size = app.cache().active_size()?;
        Ok(CacheStats { total_size, active_size })
    }
    
    #[cfg(mobile)]
    {
        app.cache().stats()
    }
}
