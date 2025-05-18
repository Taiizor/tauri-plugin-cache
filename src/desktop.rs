use std::collections::HashMap;
use std::fs::{create_dir_all, File};
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::de::DeserializeOwned;
use tauri::{AppHandle, Manager, Runtime, plugin::PluginApi};

use crate::models::CacheEntry;
use crate::{Error, Result};

/// Access to the cache APIs.
pub struct Cache<R: Runtime> {
  app: AppHandle<R>,
  storage_path: PathBuf,
  cache: Arc<Mutex<HashMap<String, CacheEntry>>>,
}

impl<R: Runtime> Cache<R> {
  /// Set a value in the cache
  pub async fn set(&self, key: &str, value: &str, ttl: Option<i64>) -> Result<()> {
    let expires_at = ttl.map(|ttl| {
      SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_else(|_| Duration::from_secs(0))
        .as_secs() as i64 + ttl
    });

    let entry = CacheEntry {
      key: key.to_string(),
      value: value.to_string(),
      expires_at,
    };

    {
      let mut cache = self.cache.lock().map_err(|_| Error::Other("Failed to acquire cache lock".into()))?;
      cache.insert(key.to_string(), entry);
    }

    self.save_cache()?;
    Ok(())
  }

  /// Get a value from the cache
  pub async fn get(&self, key: &str) -> Result<Option<String>> {
    let mut cache = self.cache.lock().map_err(|_| Error::Other("Failed to acquire cache lock".into()))?;
    
    if let Some(entry) = cache.get(key) {
      // Check if the entry has expired
      if let Some(expires_at) = entry.expires_at {
        let now = SystemTime::now()
          .duration_since(UNIX_EPOCH)
          .unwrap_or_else(|_| Duration::from_secs(0))
          .as_secs() as i64;

        if now > expires_at {
          // Entry has expired, remove it
          cache.remove(key);
          self.save_cache()?;
          return Ok(None);
        }
      }

      Ok(Some(entry.value.clone()))
    } else {
      Ok(None)
    }
  }

  /// Check if a key exists in the cache
  pub async fn has(&self, key: &str) -> Result<bool> {
    let value = self.get(key).await?;
    Ok(value.is_some())
  }

  /// Remove a key from the cache
  pub async fn remove(&self, key: &str) -> Result<()> {
    let mut cache = self.cache.lock().map_err(|_| Error::Other("Failed to acquire cache lock".into()))?;
    cache.remove(key);
    self.save_cache()?;
    Ok(())
  }

  /// Clear all entries from the cache
  pub async fn clear(&self) -> Result<()> {
    let mut cache = self.cache.lock().map_err(|_| Error::Other("Failed to acquire cache lock".into()))?;
    cache.clear();
    self.save_cache()?;
    Ok(())
  }

  /// Get all keys in the cache
  pub async fn keys(&self) -> Result<Vec<String>> {
    let cache = self.cache.lock().map_err(|_| Error::Other("Failed to acquire cache lock".into()))?;
    let now = SystemTime::now()
      .duration_since(UNIX_EPOCH)
      .unwrap_or_else(|_| Duration::from_secs(0))
      .as_secs() as i64;
    
    // Filter out expired keys
    let keys: Vec<String> = cache
      .iter()
      .filter(|(_, v)| {
        if let Some(expires_at) = v.expires_at {
          now <= expires_at
        } else {
          true
        }
      })
      .map(|(k, _)| k.clone())
      .collect();

    Ok(keys)
  }

  // Helper method to save the cache to disk
  fn save_cache(&self) -> Result<()> {
    let cache = self.cache.lock().map_err(|_| Error::Other("Failed to acquire cache lock".into()))?;
    let json = serde_json::to_string(&*cache).map_err(Error::from)?;
    
    // Create the directory if it doesn't exist
    if let Some(parent) = self.storage_path.parent() {
      create_dir_all(parent).map_err(Error::from)?;
    }
    
    let mut file = File::create(&self.storage_path).map_err(Error::from)?;
    file.write_all(json.as_bytes()).map_err(Error::from)?;
    
    Ok(())
  }
}

pub fn init<R: Runtime, C: DeserializeOwned>(app: &AppHandle<R>, api: PluginApi<R, C>) -> Result<Cache<R>> {
  // Get app path
  let app_dir = app.path().app_config_dir()
    .map_err(|e| Error::InitError(format!("Failed to get app config directory: {}", e)))?;
  
  // Create cache directory
  let cache_dir = app_dir.join("cache");
  if let Err(e) = create_dir_all(&cache_dir) {
    return Err(Error::InitError(format!("Failed to create cache directory: {}", e)));
  }
  
  // Set cache file path
  let storage_path = cache_dir.join("cache.json");
  
  // Initialize cache data
  let cache = if storage_path.exists() {
    match File::open(&storage_path) {
      Ok(mut file) => {
        let mut contents = String::new();
        match file.read_to_string(&mut contents) {
          Ok(_) => {
            serde_json::from_str(&contents).unwrap_or_else(|_| HashMap::new())
          },
          Err(_) => HashMap::new(),
        }
      },
      Err(_) => HashMap::new(),
    }
  } else {
    HashMap::new()
  };

  Ok(Cache {
    app: app.clone(),
    storage_path,
    cache: Arc::new(Mutex::new(cache)),
  })
}

pub fn inner<R: Runtime>(cache: tauri::State<'_, Cache<R>>) -> &Cache<R> {
  cache.inner()
}