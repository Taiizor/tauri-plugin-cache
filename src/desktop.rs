use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::{plugin::PluginApi, AppHandle, Manager, Runtime};

use crate::models::*;
use crate::Error;

// Store the value and its optional expiry time in a single struct for better organization
#[derive(Clone, Serialize, Deserialize)]
struct CacheEntry {
    value: serde_json::Value,
    expires_at: Option<u64>,
}

type CacheStorage = Arc<RwLock<HashMap<String, CacheEntry>>>;

pub fn init<R: Runtime, C: DeserializeOwned>(
  app: &AppHandle<R>,
  _api: PluginApi<R, C>,
) -> crate::Result<Cache<R>> {
  // Try to get configuration from the plugin API
  let config = CacheConfig::default();
  
  // Determine the cache directory
  let cache_dir = if let Some(custom_dir) = config.cache_dir {
    PathBuf::from(custom_dir)
  } else {
    app.path().app_cache_dir()
      .map_err(|e| Error::Cache(format!("Failed to get app cache directory: {}", e)))?
  };
  
  // Create the cache directory if it doesn't exist
  fs::create_dir_all(&cache_dir)
    .map_err(|e| Error::Cache(format!("Failed to create cache directory: {}", e)))?;
  
  // Determine the cache file name
  let cache_file_name = config.cache_file_name.as_deref().unwrap_or("tauri_cache.json");
  let cache_file_path = cache_dir.join(cache_file_name);
  
  // Load existing cache from file if it exists
  let data: HashMap<String, CacheEntry> = if cache_file_path.exists() {
    match fs::read_to_string(&cache_file_path) {
      Ok(content) => {
        serde_json::from_str(&content).unwrap_or_else(|_| HashMap::new())
      }
      Err(_) => HashMap::new()
    }
  } else {
    HashMap::new()
  };
  
  let cleanup_interval = config.cleanup_interval.unwrap_or(60);

  let cache = Cache {
    app: app.clone(),
    data: Arc::new(RwLock::new(data)),
    cache_file_path,
    cleanup_interval,
  };

  // Set up a background task to clean expired entries periodically
  cache.start_cleanup_task();

  Ok(cache)
}

pub fn init_with_config<R: Runtime, C: DeserializeOwned>(
  app: &AppHandle<R>,
  _api: PluginApi<R, C>,
  cache_file_path: PathBuf,
  cleanup_interval: u64
) -> crate::Result<Cache<R>> {
  // Load existing cache from file if it exists
  let data: HashMap<String, CacheEntry> = if cache_file_path.exists() {
    match fs::read_to_string(&cache_file_path) {
      Ok(content) => {
        serde_json::from_str(&content).unwrap_or_else(|_| HashMap::new())
      }
      Err(_) => HashMap::new()
    }
  } else {
    HashMap::new()
  };

  let cache = Cache {
    app: app.clone(),
    data: Arc::new(RwLock::new(data)),
    cache_file_path,
    cleanup_interval,
  };

  // Set up a background task to clean expired entries periodically
  cache.start_cleanup_task();

  Ok(cache)
}

/// Access to the cache APIs.
pub struct Cache<R: Runtime> {
  app: AppHandle<R>,
  data: CacheStorage,
  cache_file_path: PathBuf,
  cleanup_interval: u64,
}

impl<R: Runtime> Cache<R> {
  /// Start a background task to periodically clean up expired cache entries
  fn start_cleanup_task(&self) {
    let data_clone = self.data.clone();
    let interval = self.cleanup_interval;
    let cache_file_path = self.cache_file_path.clone();
    
    // Use a background thread to periodically clean up expired items
    std::thread::spawn(move || {
      loop {
        std::thread::sleep(Duration::from_secs(interval));
        
        // Clean up expired entries
        let now = SystemTime::now()
          .duration_since(UNIX_EPOCH)
          .unwrap()
          .as_secs();
        
        let mut data = data_clone.write().unwrap();
        let expired_keys: Vec<String> = data
          .iter()
          .filter_map(|(key, entry)| {
            if let Some(expires_at) = entry.expires_at {
              if expires_at < now {
                Some(key.clone())
              } else {
                None
              }
            } else {
              None
            }
          })
          .collect();
        
        let mut modified = false;
        for key in expired_keys {
          data.remove(&key);
          modified = true;
        }
        
        // Save to file if cache was modified
        if modified {
          if let Ok(json) = serde_json::to_string(&*data) {
            let _ = fs::write(&cache_file_path, json);
          }
        }
      }
    });
  }
  
  /// Save the current cache to file
  fn save_to_file(&self) -> crate::Result<()> {
    let data = self.data.read().unwrap();
    let json = serde_json::to_string(&*data)
      .map_err(|e| Error::Json(e))?;
    
    fs::write(&self.cache_file_path, json)
      .map_err(|e| Error::Cache(format!("Failed to write cache to file: {}", e)))?;
    
    Ok(())
  }

  /// Sets a value in the cache with an optional TTL
  pub fn set<T: Serialize + std::fmt::Debug>(&self, key: String, value: T, options: Option<SetItemOptions>) -> crate::Result<EmptyResponse> {
    // Check if T is already serde_json::Value to avoid double serialization
    let json_value = match serde_json::to_value(value) {
      Ok(v) => v,
      Err(e) => return Err(Error::Json(e)),
    };

    // Calculate expiration time if TTL is provided
    let expires_at = if let Some(opts) = options {
      if let Some(ttl) = opts.ttl {
        if ttl > 0 {
          let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| Error::Cache(e.to_string()))?
            .as_secs();
          
          Some(now + ttl)
        } else {
          None
        }
      } else {
        None
      }
    } else {
      None
    };

    // Create the cache entry
    let entry = CacheEntry {
      value: json_value,
      expires_at,
    };

    // Store the entry
    let mut data_map = self.data.write().unwrap();
    data_map.insert(key, entry);
    
    // Save to file
    drop(data_map); // Release the write lock before saving
    self.save_to_file()?;

    Ok(EmptyResponse {})
  }

  /// Gets a value from the cache
  pub fn get(&self, key: &str) -> crate::Result<Option<serde_json::Value>> {
    let now = SystemTime::now()
      .duration_since(UNIX_EPOCH)
      .map_err(|e| Error::Cache(e.to_string()))?
      .as_secs();
      
    let data_map = self.data.read().unwrap();
    
    if let Some(entry) = data_map.get(key) {
      // Check if the entry has expired
      if let Some(expires_at) = entry.expires_at {
        if expires_at < now {
          // We can't remove it here because we only have a read lock
          // The cleanup task will remove it eventually
          return Ok(None);
        }
      }
      
      // Return a clone of the value
      Ok(Some(entry.value.clone()))
    } else {
      Ok(None)
    }
  }

  /// Checks if a key exists in the cache and hasn't expired
  pub fn has(&self, key: &str) -> crate::Result<BooleanResponse> {
    let now = SystemTime::now()
      .duration_since(UNIX_EPOCH)
      .map_err(|e| Error::Cache(e.to_string()))?
      .as_secs();
      
    let data_map = self.data.read().unwrap();
    
    if let Some(entry) = data_map.get(key) {
      // Check if the entry has expired
      if let Some(expires_at) = entry.expires_at {
        if expires_at < now {
          return Ok(BooleanResponse { value: false });
        }
      }
      
      Ok(BooleanResponse { value: true })
    } else {
      Ok(BooleanResponse { value: false })
    }
  }

  /// Removes a value from the cache
  pub fn remove(&self, key: &str) -> crate::Result<EmptyResponse> {
    let mut data_map = self.data.write().unwrap();
    data_map.remove(key);
    
    // Save changes to file
    drop(data_map); // Release the write lock before saving
    self.save_to_file()?;
    
    Ok(EmptyResponse {})
  }

  /// Clears the entire cache
  pub fn clear(&self) -> crate::Result<EmptyResponse> {
    let mut data_map = self.data.write().unwrap();
    data_map.clear();
    
    // Save changes to file
    drop(data_map); // Release the write lock before saving
    self.save_to_file()?;
    
    Ok(EmptyResponse {})
  }
  
  /// Gets the current size of the cache (number of entries)
  pub fn size(&self) -> crate::Result<usize> {
    let data_map = self.data.read().unwrap();
    Ok(data_map.len())
  }

  /// Gets the number of active (non-expired) items in the cache
  pub fn active_size(&self) -> crate::Result<usize> {
    let data_map = self.data.read().unwrap();
    
    let now = SystemTime::now()
      .duration_since(UNIX_EPOCH)
      .map_err(|e| Error::Cache(e.to_string()))?
      .as_secs();
    
    let active_count = data_map.iter().filter(|(_, entry)| {
      if let Some(expires_at) = entry.expires_at {
        expires_at >= now // Not expired
      } else {
        true // No expiration set
      }
    }).count();
    
    Ok(active_count)
  }
  
  /// Gets the path to the cache file
  pub fn get_cache_file_path(&self) -> PathBuf {
    self.cache_file_path.clone()
  }
}