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
#[cfg(feature = "lzma2-native")]
use xz2::read::XzDecoder;
#[cfg(feature = "lzma2-native")]
use xz2::write::XzEncoder;
#[cfg(feature = "zstd")]
use zstd::stream::read::Decoder as ZstdDecoder;
#[cfg(feature = "zstd")]
use zstd::stream::write::Encoder as ZstdEncoder;

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
        app: Some(app.clone()),
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
    app: Option<AppHandle<R>>,
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

    /// Compress a JSON value using a specific configuration
    fn compress_value_with_config(
        &self,
        value: &serde_json::Value,
        config: &CompressionConfig,
    ) -> crate::Result<Vec<u8>> {
        // First serialize to JSON string to determine size
        let json_string = serde_json::to_string(value)
            .map_err(|e| Error::Cache(format!("Failed to serialize value: {}", e)))?;

        // Check if value is below the compression threshold
        if !config.enabled || json_string.len() < config.threshold {
            // Return a special marker that indicates this value wasn't compressed
            let mut result = Vec::with_capacity(json_string.len() + 2);
            result.push(0); // Marker for uncompressed data
            result.push(0); // Method marker (unused for uncompressed)
            result.extend_from_slice(json_string.as_bytes());
            return Ok(result);
        }

        // For large data, use chunked processing to avoid memory spikes
        let bytes = json_string.as_bytes();
        const CHUNK_SIZE: usize = 64 * 1024; // 64KB chunks

        match config.method {
            CompressionMethod::Zlib => {
                // Apply Zlib compression with the configured level
                let zlib_level = Compression::new(config.level);
                let mut encoder = ZlibEncoder::new(Vec::new(), zlib_level);

                if bytes.len() > CHUNK_SIZE {
                    // Process in chunks for large data
                    for chunk in bytes.chunks(CHUNK_SIZE) {
                        encoder.write_all(chunk).map_err(|e| {
                            Error::Cache(format!("Failed to compress value chunk: {}", e))
                        })?;
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

                // Prepend markers (1 = compressed, 1 = Zlib)
                let mut result = Vec::with_capacity(compressed.len() + 2);
                result.push(1); // Compressed marker
                result.push(1); // Method marker: 1 = Zlib
                result.append(&mut compressed);

                Ok(result)
            }
            #[cfg(feature = "lzma2-native")]
            CompressionMethod::Lzma2 => {
                // Apply LZMA2 compression with the configured level
                let mut encoder = XzEncoder::new(Vec::new(), config.level);

                if bytes.len() > CHUNK_SIZE {
                    // Process in chunks for large data
                    for chunk in bytes.chunks(CHUNK_SIZE) {
                        encoder.write_all(chunk).map_err(|e| {
                            Error::Cache(format!("Failed to compress value chunk: {}", e))
                        })?;
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

                // Prepend markers (1 = compressed, 2 = LZMA2)
                let mut result = Vec::with_capacity(compressed.len() + 2);
                result.push(1); // Compressed marker
                result.push(2); // Method marker: 2 = LZMA2
                result.append(&mut compressed);

                Ok(result)
            }
            #[cfg(not(feature = "lzma2-native"))]
            CompressionMethod::Lzma2 => Err(Error::Cache(
                "LZMA2 support is not compiled in; enable the `lzma2-native` feature of tauri-plugin-cache"
                    .to_string(),
            )),
            #[cfg(feature = "zstd")]
            CompressionMethod::Zstd => {
                let zstd_level = if config.level == 0 {
                    zstd::DEFAULT_COMPRESSION_LEVEL
                } else {
                    config.level as i32
                };
                let mut encoder = ZstdEncoder::new(Vec::new(), zstd_level)
                    .map_err(|e| Error::Cache(format!("Failed to initialize Zstd encoder: {}", e)))?;

                if bytes.len() > CHUNK_SIZE {
                    for chunk in bytes.chunks(CHUNK_SIZE) {
                        encoder.write_all(chunk).map_err(|e| {
                            Error::Cache(format!("Failed to compress value chunk: {}", e))
                        })?;
                    }
                } else {
                    encoder
                        .write_all(bytes)
                        .map_err(|e| Error::Cache(format!("Failed to compress value: {}", e)))?;
                }

                let mut compressed = encoder
                    .finish()
                    .map_err(|e| Error::Cache(format!("Failed to finish Zstd compression: {}", e)))?;

                // Prepend markers (1 = compressed, 3 = Zstd)
                let mut result = Vec::with_capacity(compressed.len() + 2);
                result.push(1); // Compressed marker
                result.push(3); // Method marker: 3 = Zstd
                result.append(&mut compressed);

                Ok(result)
            }
            #[cfg(not(feature = "zstd"))]
            CompressionMethod::Zstd => Err(Error::Cache(
                "Zstd support is not compiled in; enable the `zstd` feature of tauri-plugin-cache"
                    .to_string(),
            )),
        }
    }

    /// Compress a JSON value using the default compression configuration
    #[allow(dead_code)]
    fn compress_value(&self, value: &serde_json::Value) -> crate::Result<Vec<u8>> {
        self.compress_value_with_config(value, &self.compression)
    }

    /// Decompress a compressed value back to JSON
    fn decompress_value(&self, data: &[u8]) -> crate::Result<serde_json::Value> {
        if data.is_empty() {
            return Err(Error::Cache(
                "Empty data provided for decompression".to_string(),
            ));
        }

        // First byte indicates if data is compressed
        let is_compressed = data[0] == 1;

        if !is_compressed {
            // Data is not compressed - parse the JSON directly
            let json_data = &data[2..]; // Skip the marker bytes

            let string_data = std::str::from_utf8(json_data)
                .map_err(|e| Error::Cache(format!("Failed to decode uncompressed data: {}", e)))?;

            return serde_json::from_str(string_data)
                .map_err(|e| Error::Cache(format!("Failed to deserialize value: {}", e)));
        }

        // Second byte indicates compression method
        let method_marker = data[1];
        let compressed_data = &data[2..]; // Skip the marker bytes

        match method_marker {
            1 => {
                // Zlib decompression
                let mut decoder = ZlibDecoder::new(compressed_data);
                let mut json_string = String::new();

                decoder
                    .read_to_string(&mut json_string)
                    .map_err(|e| Error::Cache(format!("Failed to decompress Zlib data: {}", e)))?;

                serde_json::from_str(&json_string)
                    .map_err(|e| Error::Cache(format!("Failed to parse decompressed JSON: {}", e)))
            }
            #[cfg(feature = "lzma2-native")]
            2 => {
                // LZMA2 decompression
                let mut decoder = XzDecoder::new(compressed_data);
                let mut json_string = String::new();

                decoder
                    .read_to_string(&mut json_string)
                    .map_err(|e| Error::Cache(format!("Failed to decompress LZMA2 data: {}", e)))?;

                serde_json::from_str(&json_string)
                    .map_err(|e| Error::Cache(format!("Failed to parse decompressed JSON: {}", e)))
            }
            #[cfg(not(feature = "lzma2-native"))]
            2 => Err(Error::Cache(
                "Cache entry is LZMA2 compressed, but LZMA2 support is not compiled in; enable the `lzma2-native` feature of tauri-plugin-cache"
                    .to_string(),
            )),
            #[cfg(feature = "zstd")]
            3 => {
                // Zstd decompression
                let mut decoder = ZstdDecoder::new(compressed_data)
                    .map_err(|e| Error::Cache(format!("Failed to initialize Zstd decoder: {}", e)))?;
                let mut json_string = String::new();

                decoder
                    .read_to_string(&mut json_string)
                    .map_err(|e| Error::Cache(format!("Failed to decompress Zstd data: {}", e)))?;

                serde_json::from_str(&json_string)
                    .map_err(|e| Error::Cache(format!("Failed to parse decompressed JSON: {}", e)))
            }
            #[cfg(not(feature = "zstd"))]
            3 => Err(Error::Cache(
                "Cache entry is Zstd compressed, but Zstd support is not compiled in; enable the `zstd` feature of tauri-plugin-cache"
                    .to_string(),
            )),
            _ => Err(Error::Cache(format!(
                "Unknown compression method marker: {}",
                method_marker
            ))),
        }
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

        // Create a temporary compression config based on options
        let temp_compression = CompressionConfig {
            enabled: should_compress,
            level: self.compression.level,
            threshold: self.compression.threshold,
            method: options
                .as_ref()
                .and_then(|opt| opt.compression_method.clone())
                .unwrap_or(self.compression.method.clone()),
        };

        // Process the value based on compression settings
        let entry = if should_compress {
            // Compress the value using the temporary compression config
            let processed_data = self.compress_value_with_config(&value_json, &temp_compression)?;
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

    /// Get detailed information and compression metrics about a cached item
    pub fn details(&self, key: &str) -> crate::Result<Option<ItemDetails>> {
        let _guard = self.file_mutex.lock().unwrap();

        let data = Self::read_from_file(&self.cache_file_path)
            .map_err(|e| Error::Cache(format!("Failed to read cache file: {}", e)))?;

        if let Some(entry) = data.get(key) {
            // Check if expired
            if let Some(expires_at) = entry.expires_at {
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs();
                if expires_at < now {
                    return Ok(None);
                }
            }

            let is_compressed = entry.is_compressed.unwrap_or(false);

            if is_compressed {
                if let serde_json::Value::String(ref encoded_str) = entry.value {
                    let processed_data = STANDARD
                        .decode(encoded_str)
                        .map_err(|e| Error::Cache(format!("Failed to decode base64: {}", e)))?;

                    let method = match processed_data.get(1) {
                        Some(1) => Some(CompressionMethod::Zlib),
                        Some(2) => Some(CompressionMethod::Lzma2),
                        Some(3) => Some(CompressionMethod::Zstd),
                        _ => None,
                    };

                    // Payload size excluding the 2 marker bytes
                    let compressed_size = if processed_data.len() >= 2 {
                        processed_data.len() - 2
                    } else {
                        processed_data.len()
                    };

                    // Decompress to determine the exact uncompressed serialized JSON size
                    let decompressed_val = self.decompress_value(&processed_data)?;
                    let original_size = serde_json::to_vec(&decompressed_val)
                        .map_err(|e| Error::Cache(format!("Failed to serialize decompressed value: {}", e)))?
                        .len();

                    let compression_ratio = if original_size > 0 {
                        ((1.0 - (compressed_size as f64 / original_size as f64)) * 100.0).max(0.0)
                    } else {
                        0.0
                    };

                    Ok(Some(ItemDetails {
                        key: key.to_string(),
                        original_size,
                        compressed_size,
                        is_compressed: true,
                        compression_method: method,
                        compression_ratio,
                    }))
                } else {
                    Err(Error::Cache(
                        "Compressed value is not in expected format".to_string(),
                    ))
                }
            } else {
                let original_size = serde_json::to_vec(&entry.value)
                    .map_err(|e| Error::Cache(format!("Failed to serialize value: {}", e)))?
                    .len();

                Ok(Some(ItemDetails {
                    key: key.to_string(),
                    original_size,
                    compressed_size: original_size,
                    is_compressed: false,
                    compression_method: None,
                    compression_ratio: 0.0,
                }))
            }
        } else {
            Ok(None)
        }
    }

    /// Configure the cache with compression settings
    pub fn init_with_config(
        &mut self,
        default_compression: bool,
        compression_level: Option<u32>,
        threshold: Option<usize>,
        compression_method: Option<CompressionMethod>,
    ) {
        self.compression = CompressionConfig {
            enabled: default_compression,
            level: compression_level.unwrap_or(6),
            threshold: threshold.unwrap_or(COMPRESSION_THRESHOLD),
            method: compression_method.unwrap_or(CompressionMethod::Zlib),
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn create_test_cache(dir: &tempfile::TempDir) -> Cache<tauri::Wry> {
        let cache_path = dir.path().join("test_cache.json");

        Cache {
            app: None,
            cache_file_path: cache_path,
            cleanup_interval: 60,
            file_mutex: Arc::new(Mutex::new(())),
            compression: CompressionConfig::default(),
            value_cache: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    #[test]
    fn test_basic_cache_operations() {
        let dir = tempdir().unwrap();
        let cache = create_test_cache(&dir);

        // Test set and get
        let key = "user_name".to_string();
        let val = "Taiizor".to_string();
        cache.set(key.clone(), val.clone(), None).unwrap();

        let retrieved = cache.get(&key).unwrap().unwrap();
        assert_eq!(retrieved, serde_json::Value::String(val.clone()));

        // Test has
        let has_resp = cache.has(&key).unwrap();
        assert!(has_resp.value);

        // Test non-existent key
        let non_existent = cache.has("non_existent").unwrap();
        assert!(!non_existent.value);

        // Test size
        assert_eq!(cache.size().unwrap(), 1);
        assert_eq!(cache.active_size().unwrap(), 1);

        // Test remove
        cache.remove(&key).unwrap();
        assert!(!cache.has(&key).unwrap().value);
        assert_eq!(cache.size().unwrap(), 0);
    }

    #[test]
    fn test_clear() {
        let dir = tempdir().unwrap();
        let cache = create_test_cache(&dir);

        cache.set("k1".into(), "v1".to_string(), None).unwrap();
        cache.set("k2".into(), "v2".to_string(), None).unwrap();
        assert_eq!(cache.size().unwrap(), 2);

        cache.clear().unwrap();
        assert_eq!(cache.size().unwrap(), 0);
        assert!(!cache.has("k1").unwrap().value);
        assert!(!cache.has("k2").unwrap().value);
    }

    #[test]
    fn test_ttl_expiration() {
        let dir = tempdir().unwrap();
        let cache = create_test_cache(&dir);

        // Set item with TTL = 1 second
        cache
            .set(
                "temp_item".into(),
                "expires_fast".to_string(),
                Some(SetItemOptions {
                    ttl: Some(1),
                    compress: None,
                    compression_method: None,
                }),
            )
            .unwrap();

        assert!(cache.has("temp_item").unwrap().value);
        assert_eq!(cache.active_size().unwrap(), 1);

        // Wait 2 seconds for it to expire
        std::thread::sleep(Duration::from_secs(2));

        // In-memory and file should see it as expired
        assert!(!cache.has("temp_item").unwrap().value);
        assert_eq!(cache.get("temp_item").unwrap(), None);
        assert_eq!(cache.active_size().unwrap(), 0);
        // Total size before disk cleanup might still be 1
        assert_eq!(cache.size().unwrap(), 1);
    }

    #[test]
    fn test_persistence_across_instances() {
        let dir = tempdir().unwrap();
        let cache_path = dir.path().join("persist_cache.json");

        // Instance 1 writes
        {
            let cache: Cache<tauri::Wry> = Cache {
                app: None,
                cache_file_path: cache_path.clone(),
                cleanup_interval: 60,
                file_mutex: Arc::new(Mutex::new(())),
                compression: CompressionConfig::default(),
                value_cache: Arc::new(Mutex::new(HashMap::new())),
            };
            cache.set("persisted".into(), 42i32, None).unwrap();
        }

        // Instance 2 reads from the same file
        {
            let cache: Cache<tauri::Wry> = Cache {
                app: None,
                cache_file_path: cache_path,
                cleanup_interval: 60,
                file_mutex: Arc::new(Mutex::new(())),
                compression: CompressionConfig::default(),
                value_cache: Arc::new(Mutex::new(HashMap::new())),
            };
            let val = cache.get("persisted").unwrap().unwrap();
            assert_eq!(val, serde_json::json!(42));
        }
    }

    #[test]
    fn test_zlib_compression() {
        let dir = tempdir().unwrap();
        let mut cache = create_test_cache(&dir);
        cache.init_with_config(true, Some(6), Some(100), Some(CompressionMethod::Zlib));

        // Data larger than 100 bytes threshold
        let large_string = "Hello World! ".repeat(50);
        cache
            .set(
                "zlib_data".into(),
                large_string.clone(),
                Some(SetItemOptions {
                    ttl: None,
                    compress: Some(true),
                    compression_method: Some(CompressionMethod::Zlib),
                }),
            )
            .unwrap();

        let retrieved = cache.get("zlib_data").unwrap().unwrap();
        assert_eq!(retrieved, serde_json::Value::String(large_string));
    }

    #[cfg(feature = "zstd")]
    #[test]
    fn test_zstd_compression() {
        let dir = tempdir().unwrap();
        let mut cache = create_test_cache(&dir);
        cache.init_with_config(true, Some(3), Some(100), Some(CompressionMethod::Zstd));

        let large_string = "Zstandard Test Data Content! Fast and efficient! ".repeat(50);
        cache
            .set(
                "zstd_data".into(),
                large_string.clone(),
                Some(SetItemOptions {
                    ttl: None,
                    compress: Some(true),
                    compression_method: Some(CompressionMethod::Zstd),
                }),
            )
            .unwrap();

        let retrieved = cache.get("zstd_data").unwrap().unwrap();
        assert_eq!(retrieved, serde_json::Value::String(large_string));
    }

    #[cfg(not(feature = "zstd"))]
    #[test]
    fn test_zstd_disabled_error() {
        let dir = tempdir().unwrap();
        let mut cache = create_test_cache(&dir);
        cache.init_with_config(true, Some(3), Some(100), Some(CompressionMethod::Zstd));

        let large_string = "Zstd Disabled Content! ".repeat(50);
        let result = cache.set(
            "zstd_data".into(),
            large_string,
            Some(SetItemOptions {
                ttl: None,
                compress: Some(true),
                compression_method: Some(CompressionMethod::Zstd),
            }),
        );
        assert!(result.is_err());
    }

    #[cfg(feature = "lzma2-native")]
    #[test]
    fn test_lzma2_compression() {
        let dir = tempdir().unwrap();
        let mut cache = create_test_cache(&dir);
        cache.init_with_config(true, Some(6), Some(100), Some(CompressionMethod::Lzma2));

        let large_string = "LZMA2 Test Data Content! ".repeat(50);
        cache
            .set(
                "lzma2_data".into(),
                large_string.clone(),
                Some(SetItemOptions {
                    ttl: None,
                    compress: Some(true),
                    compression_method: Some(CompressionMethod::Lzma2),
                }),
            )
            .unwrap();

        let retrieved = cache.get("lzma2_data").unwrap().unwrap();
        assert_eq!(retrieved, serde_json::Value::String(large_string));
    }

    #[cfg(not(feature = "lzma2-native"))]
    #[test]
    fn test_lzma2_disabled_error() {
        let dir = tempdir().unwrap();
        let mut cache = create_test_cache(&dir);
        cache.init_with_config(true, Some(6), Some(100), Some(CompressionMethod::Lzma2));

        let large_string = "LZMA2 Disabled Content! ".repeat(50);
        let result = cache.set(
            "lzma2_data".into(),
            large_string,
            Some(SetItemOptions {
                ttl: None,
                compress: Some(true),
                compression_method: Some(CompressionMethod::Lzma2),
            }),
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_compression_threshold_uncompressed() {
        let dir = tempdir().unwrap();
        let mut cache = create_test_cache(&dir);
        // Set threshold to 10000 bytes so smaller data stays uncompressed
        cache.init_with_config(true, Some(6), Some(10000), Some(CompressionMethod::Zlib));

        let small_string = "short text".to_string();
        cache
            .set(
                "small_data".into(),
                small_string.clone(),
                Some(SetItemOptions {
                    ttl: None,
                    compress: Some(true),
                    compression_method: Some(CompressionMethod::Zlib),
                }),
            )
            .unwrap();

        let retrieved = cache.get("small_data").unwrap().unwrap();
        assert_eq!(retrieved, serde_json::Value::String(small_string));
    }

    #[test]
    fn test_item_details() {
        let dir = tempdir().unwrap();
        let mut cache = create_test_cache(&dir);
        cache.init_with_config(true, Some(6), Some(10), Some(CompressionMethod::Zlib));

        // Test uncompressed item details
        let small = "hello".to_string();
        cache
            .set(
                "uncompressed".into(),
                small.clone(),
                Some(SetItemOptions {
                    ttl: None,
                    compress: Some(false),
                    compression_method: None,
                }),
            )
            .unwrap();

        let details = cache.details("uncompressed").unwrap().unwrap();
        assert_eq!(details.key, "uncompressed");
        assert!(!details.is_compressed);
        assert_eq!(details.original_size, details.compressed_size);
        assert_eq!(details.compression_ratio, 0.0);

        // Test compressed item details
        let large = "Repeat repetition test string for compression! ".repeat(50);
        cache
            .set(
                "compressed".into(),
                large.clone(),
                Some(SetItemOptions {
                    ttl: None,
                    compress: Some(true),
                    compression_method: Some(CompressionMethod::Zlib),
                }),
            )
            .unwrap();

        let details = cache.details("compressed").unwrap().unwrap();
        assert_eq!(details.key, "compressed");
        assert!(details.is_compressed);
        assert_eq!(details.compression_method, Some(CompressionMethod::Zlib));
        assert!(details.compressed_size < details.original_size);
        assert!(details.compression_ratio > 0.0);
    }
}
