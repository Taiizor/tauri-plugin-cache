use serde::{de::DeserializeOwned, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::{plugin::PluginApi, AppHandle, Runtime};

use crate::models::*;
use crate::Error;

// Store the value and its optional expiry time in a single struct for better organization
#[derive(Clone)]
struct CacheEntry {
    value: serde_json::Value,
    expires_at: Option<u64>,
}

type CacheStorage = Arc<RwLock<HashMap<String, CacheEntry>>>;

pub fn init<R: Runtime, C: DeserializeOwned>(
  app: &AppHandle<R>,
  _api: PluginApi<R, C>,
) -> crate::Result<Cache<R>> {
  let cache = Cache {
    app: app.clone(),
    data: Arc::new(RwLock::new(HashMap::new())),
    cleanup_interval: 60, // Default cleanup every 60 seconds
  };

  // Set up a background task to clean expired entries periodically
  cache.start_cleanup_task();

  Ok(cache)
}

/// Access to the cache APIs.
pub struct Cache<R: Runtime> {
  app: AppHandle<R>,
  data: CacheStorage,
  cleanup_interval: u64,
}

impl<R: Runtime> Cache<R> {
  /// Start a background task to periodically clean up expired cache entries
  fn start_cleanup_task(&self) {
    let data_clone = self.data.clone();
    let interval = self.cleanup_interval;
    
    // Use tauri's event loop to periodically clean up expired items
    let app_handle = self.app.clone();
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
        
        for key in expired_keys {
          data.remove(&key);
        }
      }
    });
  }

  /// Sets a value in the cache with an optional TTL
  pub fn set<T: Serialize>(&self, key: String, value: T, options: Option<SetItemOptions>) -> crate::Result<EmptyResponse> {
    // Serialize the value to JSON
    let json_value = serde_json::to_value(value)
      .map_err(|e| Error::Json(e))?;

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
    Ok(EmptyResponse {})
  }

  /// Clears the entire cache
  pub fn clear(&self) -> crate::Result<EmptyResponse> {
    let mut data_map = self.data.write().unwrap();
    data_map.clear();
    Ok(EmptyResponse {})
  }
  
  /// Gets the current size of the cache (number of entries)
  pub fn size(&self) -> crate::Result<usize> {
    let data_map = self.data.read().unwrap();
    Ok(data_map.len())
  }
  
  /// Sets the cleanup interval in seconds
  pub fn set_cleanup_interval(&mut self, seconds: u64) {
    self.cleanup_interval = seconds;
  }
}
}
