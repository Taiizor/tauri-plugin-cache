use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::{self, Read, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
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

// Disk-based cache store
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
  
  let cleanup_interval = config.cleanup_interval.unwrap_or(60);

  let cache = Cache {
    app: app.clone(),
    cache_file_path,
    cleanup_interval,
    file_mutex: Arc::new(Mutex::new(())),
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
  let cache = Cache {
    app: app.clone(),
    cache_file_path,
    cleanup_interval,
    file_mutex: Arc::new(Mutex::new(())),
  };

  // Set up a background task to clean expired entries periodically
  cache.start_cleanup_task();

  Ok(cache)
}

/// Access to the cache APIs.
pub struct Cache<R: Runtime> {
  app: AppHandle<R>,
  cache_file_path: PathBuf,
  cleanup_interval: u64,
  file_mutex: Arc<Mutex<()>>,
}

impl<R: Runtime> Cache<R> {
  /// Start a background task to periodically clean up expired cache entries
  fn start_cleanup_task(&self) {
    let file_mutex = self.file_mutex.clone();
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
        
        // Lock the file for exclusive access
        let _guard = file_mutex.lock().unwrap();
        
        // Read the current cache
        let mut data: HashMap<String, CacheEntry> = match Self::read_from_file(&cache_file_path) {
          Ok(data) => data,
          Err(_) => continue, // Skip this cleanup cycle if file cannot be read
        };
        
        // Filter out expired entries
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
          let _ = Self::write_to_file(&cache_file_path, &data);
        }
      }
    });
  }
  
  /// Read cache data from file
  fn read_from_file(path: &PathBuf) -> io::Result<HashMap<String, CacheEntry>> {
    if !path.exists() {
      return Ok(HashMap::new());
    }
    
    let mut file = fs::File::open(path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    
    if contents.is_empty() {
      return Ok(HashMap::new());
    }
    
    match serde_json::from_str(&contents) {
      Ok(data) => Ok(data),
      Err(_) => Ok(HashMap::new())
    }
  }
  
  /// Write cache data to file
  fn write_to_file(path: &PathBuf, data: &HashMap<String, CacheEntry>) -> io::Result<()> {
    let json = serde_json::to_string(data)?;
    let mut file = fs::File::create(path)?;
    file.write_all(json.as_bytes())?;
    Ok(())
  }

  /// Sets a value in the cache with an optional TTL
  pub fn set<T: Serialize + std::fmt::Debug>(&self, key: String, value: T, options: Option<SetItemOptions>) -> crate::Result<EmptyResponse> {
    // Acquire lock for file operations
    let _guard = self.file_mutex.lock().unwrap();
    
    // Get current cache data
    let mut data = Self::read_from_file(&self.cache_file_path)
      .map_err(|e| Error::Cache(format!("Failed to read cache file: {}", e)))?;
    
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
    data.insert(key, entry);
    
    // Save to file
    Self::write_to_file(&self.cache_file_path, &data)
      .map_err(|e| Error::Cache(format!("Failed to write cache file: {}", e)))?;

    Ok(EmptyResponse {})
  }

  /// Gets a value from the cache
  pub fn get(&self, key: &str) -> crate::Result<Option<serde_json::Value>> {
    // Acquire lock for file operations
    let _guard = self.file_mutex.lock().unwrap();
    
    // Get current time
    let now = SystemTime::now()
      .duration_since(UNIX_EPOCH)
      .map_err(|e| Error::Cache(e.to_string()))?
      .as_secs();
      
    // Load data from file
    let data = Self::read_from_file(&self.cache_file_path)
      .map_err(|e| Error::Cache(format!("Failed to read cache file: {}", e)))?;
    
    if let Some(entry) = data.get(key) {
      // Check if the entry has expired
      if let Some(expires_at) = entry.expires_at {
        if expires_at < now {
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
    // Acquire lock for file operations
    let _guard = self.file_mutex.lock().unwrap();
    
    // Get current time
    let now = SystemTime::now()
      .duration_since(UNIX_EPOCH)
      .map_err(|e| Error::Cache(e.to_string()))?
      .as_secs();
    
    // Load data from file
    let data = Self::read_from_file(&self.cache_file_path)
      .map_err(|e| Error::Cache(format!("Failed to read cache file: {}", e)))?;
    
    if let Some(entry) = data.get(key) {
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
    // Acquire lock for file operations
    let _guard = self.file_mutex.lock().unwrap();
    
    // Load data from file
    let mut data = Self::read_from_file(&self.cache_file_path)
      .map_err(|e| Error::Cache(format!("Failed to read cache file: {}", e)))?;
    
    // Remove item if exists
    if data.remove(key).is_some() {
      // Save changes to file
      Self::write_to_file(&self.cache_file_path, &data)
        .map_err(|e| Error::Cache(format!("Failed to write cache file: {}", e)))?;
    }
    
    Ok(EmptyResponse {})
  }

  /// Clears the entire cache
  pub fn clear(&self) -> crate::Result<EmptyResponse> {
    // Acquire lock for file operations
    let _guard = self.file_mutex.lock().unwrap();
    
    // Just write an empty cache
    Self::write_to_file(&self.cache_file_path, &HashMap::new())
      .map_err(|e| Error::Cache(format!("Failed to write cache file: {}", e)))?;
    
    Ok(EmptyResponse {})
  }
  
  /// Gets the current size of the cache (number of entries)
  pub fn size(&self) -> crate::Result<usize> {
    // Acquire lock for file operations
    let _guard = self.file_mutex.lock().unwrap();
    
    // Load data from file
    let data = Self::read_from_file(&self.cache_file_path)
      .map_err(|e| Error::Cache(format!("Failed to read cache file: {}", e)))?;
    
    Ok(data.len())
  }

  /// Gets the number of active (non-expired) items in the cache
  pub fn active_size(&self) -> crate::Result<usize> {
    // Acquire lock for file operations
    let _guard = self.file_mutex.lock().unwrap();
    
    // Get current time
    let now = SystemTime::now()
      .duration_since(UNIX_EPOCH)
      .map_err(|e| Error::Cache(e.to_string()))?
      .as_secs();
    
    // Load data from file
    let data = Self::read_from_file(&self.cache_file_path)
      .map_err(|e| Error::Cache(format!("Failed to read cache file: {}", e)))?;
    
    let active_count = data.iter().filter(|(_, entry)| {
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