use base64::{engine::general_purpose::STANDARD, Engine as _};
use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use flate2::Compression;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::{self, Read, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::{plugin::PluginApi, AppHandle, Runtime};

use crate::models::*;
use crate::Error;

// Store the value and its optional expiry time in a single struct for better organization
#[derive(Clone, Serialize, Deserialize)]
struct CacheEntry {
    value: serde_json::Value,
    expires_at: Option<u64>,
    is_compressed: Option<bool>,
}

// Initialize the cache with a custom configuration
pub fn init_with_config<R: Runtime, C: DeserializeOwned>(
    app: &AppHandle<R>,
    _api: PluginApi<R, C>,
    cache_file_path: PathBuf,
    cleanup_interval: u64,
) -> crate::Result<Cache<R>> {
    let cache = Cache {
        app: app.clone(),
        cache_file_path,
        cleanup_interval,
        file_mutex: Arc::new(Mutex::new(())),
        default_compression: false,
    };

    // Set up a background task to clean expired entries periodically
    cache.start_cleanup_task();

    Ok(cache)
}

/// Access to the cache APIs.
#[allow(dead_code)]
pub struct Cache<R: Runtime> {
    app: AppHandle<R>,
    cache_file_path: PathBuf,
    cleanup_interval: u64,
    file_mutex: Arc<Mutex<()>>,
    default_compression: bool,
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
                let mut data: HashMap<String, CacheEntry> =
                    match Self::read_from_file(&cache_file_path) {
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
            Err(_) => Ok(HashMap::new()),
        }
    }

    /// Write cache data to file
    fn write_to_file(path: &PathBuf, data: &HashMap<String, CacheEntry>) -> io::Result<()> {
        let json = serde_json::to_string(data)?;
        let mut file = fs::File::create(path)?;
        file.write_all(json.as_bytes())?;
        Ok(())
    }

    /// Compress a JSON value using zlib
    fn compress_value(&self, value: &serde_json::Value) -> crate::Result<Vec<u8>> {
        let json_string = serde_json::to_string(value)
            .map_err(|e| Error::Cache(format!("Failed to serialize value: {}", e)))?;

        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder
            .write_all(json_string.as_bytes())
            .map_err(|e| Error::Cache(format!("Failed to compress value: {}", e)))?;

        encoder
            .finish()
            .map_err(|e| Error::Cache(format!("Failed to finish compression: {}", e)))
    }

    /// Decompress a compressed value back to JSON
    fn decompress_value(&self, compressed_data: &[u8]) -> crate::Result<serde_json::Value> {
        let mut decoder = ZlibDecoder::new(compressed_data);
        let mut decompressed_data = String::new();

        decoder
            .read_to_string(&mut decompressed_data)
            .map_err(|e| Error::Cache(format!("Failed to decompress value: {}", e)))?;

        serde_json::from_str(&decompressed_data)
            .map_err(|e| Error::Cache(format!("Failed to deserialize value: {}", e)))
    }

    /// Sets a value in the cache with an optional TTL
    pub fn set<T: Serialize + std::fmt::Debug>(
        &self,
        key: String,
        value: T,
        options: Option<SetItemOptions>,
    ) -> crate::Result<EmptyResponse> {
        // Acquire lock for file operations
        let _guard = self.file_mutex.lock().unwrap();

        // Get current cache data
        let mut data = Self::read_from_file(&self.cache_file_path)
            .map_err(|e| Error::Cache(format!("Failed to read cache file: {}", e)))?;

        // Serialize the value to JSON
        let value_json = serde_json::to_value(value)
            .map_err(|e| Error::Cache(format!("Failed to serialize value: {}", e)))?;

        // Calculate expiration time if TTL is set
        let expires_at = options.as_ref().and_then(|opt| {
            opt.ttl.map(|ttl| {
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
                    + ttl
            })
        });

        // Check if compression is requested
        let should_compress = options
            .as_ref()
            .and_then(|opt| opt.compress)
            .unwrap_or(self.default_compression);

        let entry = if should_compress {
            // Compress the value
            let compressed_data = self.compress_value(&value_json)?;
            // Store the compressed data as a base64 string
            let compressed_str = STANDARD.encode(&compressed_data);
            CacheEntry {
                value: serde_json::Value::String(compressed_str),
                expires_at,
                is_compressed: Some(true),
            }
        } else {
            CacheEntry {
                value: value_json,
                expires_at,
                is_compressed: Some(false),
            }
        };

        // Update the cache
        data.insert(key, entry);

        // Save the updated cache to file
        Self::write_to_file(&self.cache_file_path, &data)
            .map_err(|e| Error::Cache(format!("Failed to write cache file: {}", e)))?;

        Ok(EmptyResponse::default())
    }

    /// Gets a value from the cache
    pub fn get(&self, key: &str) -> crate::Result<Option<serde_json::Value>> {
        // Acquire lock for file operations
        let _guard = self.file_mutex.lock().unwrap();

        // Get current cache data
        let data = Self::read_from_file(&self.cache_file_path)
            .map_err(|e| Error::Cache(format!("Failed to read cache file: {}", e)))?;

        if let Some(entry) = data.get(key) {
            // Check if the item has expired
            if let Some(expires_at) = entry.expires_at {
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs();

                if expires_at < now {
                    // Item has expired
                    return Ok(None);
                }
            }

            // Check if the value is compressed
            if entry.is_compressed.unwrap_or(false) {
                // Value is compressed - need to decompress
                if let serde_json::Value::String(compressed_str) = &entry.value {
                    // Decode base64
                    let compressed_data = STANDARD
                        .decode(compressed_str)
                        .map_err(|e| Error::Cache(format!("Failed to decode base64: {}", e)))?;

                    // Decompress
                    let value = self.decompress_value(&compressed_data)?;
                    return Ok(Some(value));
                } else {
                    return Err(Error::Cache(
                        "Compressed value is not in expected format".to_string(),
                    ));
                }
            }

            // Return the value as is (not compressed)
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

        let active_count = data
            .iter()
            .filter(|(_, entry)| {
                if let Some(expires_at) = entry.expires_at {
                    expires_at >= now // Not expired
                } else {
                    true // No expiration set
                }
            })
            .count();

        Ok(active_count)
    }

    /// Gets the path to the cache file
    pub fn get_cache_file_path(&self) -> PathBuf {
        self.cache_file_path.clone()
    }

    /// Initialize with custom configuration
    pub fn init_with_config(&mut self, default_compression: bool) {
        self.default_compression = default_compression;
    }
}
