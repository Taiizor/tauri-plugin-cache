use base64::{engine::general_purpose::STANDARD, Engine as _};
use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use flate2::Compression;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::{self, BufReader, BufWriter, Read, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::{plugin::PluginApi, AppHandle, Runtime};

use crate::models::*;
use crate::Error;

// Define a type alias for the complex cache value type
type CacheValueEntry = (serde_json::Value, Option<u64>);
type CacheValueMap = HashMap<String, CacheValueEntry>;
type ThreadSafeCacheMap = Arc<Mutex<CacheValueMap>>;

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
        compression: CompressionConfig::default(),
        value_cache: Arc::new(Mutex::new(HashMap::new())),
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
    compression: CompressionConfig,
    value_cache: ThreadSafeCacheMap,
}

impl<R: Runtime> Cache<R> {
    /// Start a background task to periodically clean up expired cache entries
    fn start_cleanup_task(&self) {
        let file_mutex = self.file_mutex.clone();
        let value_cache = self.value_cache.clone();
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

                // Also clean up the in-memory value cache
                {
                    let mut cache = value_cache.lock().unwrap();
                    let expired_keys: Vec<String> = cache
                        .iter()
                        .filter_map(|(key, (_, expires_at))| {
                            if let Some(expires) = expires_at {
                                if *expires < now {
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
                        cache.remove(&key);
                    }
                }

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

        let file = fs::File::open(path)?;
        let file_size = file.metadata()?.len();

        // For large files, use a buffered reader for better performance
        let mut reader = BufReader::with_capacity(
            std::cmp::min(file_size as usize, 128 * 1024), // 128KB buffer or file size
            file,
        );

        let mut contents = String::with_capacity(file_size as usize);
        reader.read_to_string(&mut contents)?;

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
        let file = fs::File::create(path)?;

        // Use a buffered writer for better performance
        let mut writer = BufWriter::with_capacity(128 * 1024, file); // 128KB buffer

        serde_json::to_writer(&mut writer, data)?;
        writer.flush()?;
        Ok(())
    }

    /// Compress a JSON value using zlib with configurable compression
    fn compress_value(&self, value: &serde_json::Value) -> crate::Result<Vec<u8>> {
        // First serialize to JSON string to determine size
        let json_string = serde_json::to_string(value)
            .map_err(|e| Error::Cache(format!("Failed to serialize value: {}", e)))?;

        // Check if value is below the compression threshold
        if !self.compression.enabled || json_string.len() < self.compression.threshold {
            // Return a special marker that indicates this value wasn't compressed
            let mut result = Vec::with_capacity(json_string.len() + 1);
            result.push(0); // Marker for uncompressed data
            result.extend_from_slice(json_string.as_bytes());
            return Ok(result);
        }

        // Apply compression with the configured level
        let compression_level = Compression::new(self.compression.level);
        let mut encoder = ZlibEncoder::new(Vec::new(), compression_level);

        // For large data, write in chunks to avoid memory spikes
        let bytes = json_string.as_bytes();
        const CHUNK_SIZE: usize = 64 * 1024; // 64KB chunks

        if bytes.len() > CHUNK_SIZE {
            // Process in chunks for large data
            for chunk in bytes.chunks(CHUNK_SIZE) {
                encoder
                    .write_all(chunk)
                    .map_err(|e| Error::Cache(format!("Failed to compress value chunk: {}", e)))?;
            }
        } else {
            // Small data can be written at once
            encoder
                .write_all(bytes)
                .map_err(|e| Error::Cache(format!("Failed to compress value: {}", e)))?;
        }

        // Add marker for compressed data
        let mut compressed = encoder
            .finish()
            .map_err(|e| Error::Cache(format!("Failed to finish compression: {}", e)))?;

        // Prepend marker (1 = compressed)
        let mut result = Vec::with_capacity(compressed.len() + 1);
        result.push(1); // Marker for compressed data
        result.append(&mut compressed);

        Ok(result)
    }

    /// Decompress a compressed value back to JSON
    fn decompress_value(&self, data: &[u8]) -> crate::Result<serde_json::Value> {
        if data.is_empty() {
            return Err(Error::Cache(
                "Empty data provided for decompression".to_string(),
            ));
        }

        // Check the compression marker
        let is_compressed = data[0] == 1;
        let actual_data = &data[1..]; // Skip the marker byte

        if !is_compressed {
            // Data wasn't compressed, parse directly
            let string_data = std::str::from_utf8(actual_data)
                .map_err(|e| Error::Cache(format!("Failed to decode uncompressed data: {}", e)))?;

            return serde_json::from_str(string_data)
                .map_err(|e| Error::Cache(format!("Failed to deserialize value: {}", e)));
        }

        // Data was compressed, decompress it
        let mut decoder = ZlibDecoder::new(actual_data);
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
        // Serialize the value to JSON first (do this outside the lock)
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

        // Update the in-memory cache first
        {
            let mut cache = self.value_cache.lock().unwrap();
            cache.insert(key.clone(), (value_json.clone(), expires_at));
        }

        // Acquire lock for file operations
        let _guard = self.file_mutex.lock().unwrap();

        // Get current cache data
        let mut data = Self::read_from_file(&self.cache_file_path)
            .map_err(|e| Error::Cache(format!("Failed to read cache file: {}", e)))?;

        // Check if compression is requested
        let should_compress = options
            .as_ref()
            .and_then(|opt| opt.compress)
            .unwrap_or(self.compression.enabled);

        let entry = if should_compress {
            // Compress the value
            let processed_data = self.compress_value(&value_json)?;
            // Store the processed data as a base64 string
            let encoded_str = STANDARD.encode(&processed_data);
            CacheEntry {
                value: serde_json::Value::String(encoded_str),
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
        // First check the in-memory cache
        {
            let cache = self.value_cache.lock().unwrap();
            if let Some((value, expires_at)) = cache.get(key) {
                // Check if expired
                if let Some(expires) = expires_at {
                    let now = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_secs();

                    if *expires < now {
                        // Item has expired, remove from in-memory cache
                        drop(cache); // Release the lock before modifying
                        let mut cache = self.value_cache.lock().unwrap();
                        cache.remove(key);
                    } else {
                        // Not expired, return the cached value
                        return Ok(Some(value.clone()));
                    }
                } else {
                    // No expiration, return the cached value
                    return Ok(Some(value.clone()));
                }
            }
        }

        // If not in memory cache, check the file
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

                    // Cache the decompressed value in memory for future use
                    {
                        let mut cache = self.value_cache.lock().unwrap();
                        cache.insert(key.to_string(), (value.clone(), entry.expires_at));
                    }

                    return Ok(Some(value));
                } else {
                    return Err(Error::Cache(
                        "Compressed value is not in expected format".to_string(),
                    ));
                }
            }

            // Store uncompressed value in memory cache for future use
            {
                let mut cache = self.value_cache.lock().unwrap();
                cache.insert(key.to_string(), (entry.value.clone(), entry.expires_at));
            }

            // Return the value as is (not compressed)
            Ok(Some(entry.value.clone()))
        } else {
            Ok(None)
        }
    }

    /// Checks if a key exists in the cache and hasn't expired
    pub fn has(&self, key: &str) -> crate::Result<BooleanResponse> {
        // First check the in-memory cache
        {
            let cache = self.value_cache.lock().unwrap();
            if let Some((_, expires_at)) = cache.get(key) {
                // Check if expired
                if let Some(expires) = expires_at {
                    let now = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_secs();

                    if *expires < now {
                        // Item has expired
                        drop(cache); // Release the lock before modifying
                        let mut cache = self.value_cache.lock().unwrap();
                        cache.remove(key);
                    } else {
                        // Not expired
                        return Ok(BooleanResponse { value: true });
                    }
                } else {
                    // No expiration
                    return Ok(BooleanResponse { value: true });
                }
            }
        }

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

            // Add to memory cache
            {
                let mut cache = self.value_cache.lock().unwrap();
                cache.insert(key.to_string(), (entry.value.clone(), entry.expires_at));
            }

            Ok(BooleanResponse { value: true })
        } else {
            Ok(BooleanResponse { value: false })
        }
    }

    /// Removes a value from the cache
    pub fn remove(&self, key: &str) -> crate::Result<EmptyResponse> {
        // Remove from in-memory cache first
        {
            let mut cache = self.value_cache.lock().unwrap();
            cache.remove(key);
        }

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
        // Clear the in-memory cache
        {
            let mut cache = self.value_cache.lock().unwrap();
            cache.clear();
        }

        // Acquire lock for file operations
        let _guard = self.file_mutex.lock().unwrap();

        // Just write an empty cache
        Self::write_to_file(&self.cache_file_path, &HashMap::new())
            .map_err(|e| Error::Cache(format!("Failed to write cache file: {}", e)))?;

        Ok(EmptyResponse {})
    }

    /// Get the total number of items in the cache
    pub fn size(&self) -> crate::Result<usize> {
        // Acquire lock for file operations
        let _guard = self.file_mutex.lock().unwrap();

        // Load data from file
        let data = Self::read_from_file(&self.cache_file_path)
            .map_err(|e| Error::Cache(format!("Failed to read cache file: {}", e)))?;

        Ok(data.len())
    }

    /// Get the number of non-expired items in the cache
    pub fn active_size(&self) -> crate::Result<usize> {
        // Acquire lock for file operations
        let _guard = self.file_mutex.lock().unwrap();

        // Load data from file
        let data = Self::read_from_file(&self.cache_file_path)
            .map_err(|e| Error::Cache(format!("Failed to read cache file: {}", e)))?;

        // Get current time
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| Error::Cache(e.to_string()))?
            .as_secs();

        // Count only non-expired items
        let active_count = data
            .iter()
            .filter(|(_, entry)| {
                if let Some(expires_at) = entry.expires_at {
                    expires_at > now
                } else {
                    true // Items without expiration are always active
                }
            })
            .count();

        Ok(active_count)
    }

    /// Get the path to the cache file
    pub fn get_cache_file_path(&self) -> PathBuf {
        self.cache_file_path.clone()
    }

    /// Configure the cache with compression settings
    pub fn init_with_config(
        &mut self,
        default_compression: bool,
        compression_level: Option<u32>,
        threshold: Option<usize>,
    ) {
        self.compression = CompressionConfig {
            enabled: default_compression,
            level: compression_level.unwrap_or(6),
            threshold: threshold.unwrap_or(COMPRESSION_THRESHOLD),
        };
    }
}
