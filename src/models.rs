use serde::{de::DeserializeOwned, Deserialize, Serialize};

// The size threshold in bytes after which compression will be applied
pub const COMPRESSION_THRESHOLD: usize = 1024; // 1KB

/// Options for setting an item in the cache
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetItemOptions {
    /// Time-to-live in seconds
    pub ttl: Option<u64>,
    /// Whether to compress the data before storing
    pub compress: Option<bool>,
}

/// A cache item with its value and expiration time
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CacheItem<T> {
    /// The stored value
    pub value: T,
    /// Unix timestamp in seconds when this item expires (if applicable)
    pub expires_at: Option<u64>,
    /// Whether the data is compressed
    pub is_compressed: Option<bool>,
}

/// Request to set an item in the cache
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetRequest<T>
where
    T: Serialize + DeserializeOwned,
{
    /// The key to store the value under
    pub key: String,
    /// The value to store
    pub value: T,
    /// Options for this cache item
    pub options: Option<SetItemOptions>,
}

impl<'de, T> serde::Deserialize<'de> for SetRequest<T>
where
    T: Serialize + DeserializeOwned,
{
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Helper {
            key: String,
            value: serde_json::Value,
            options: Option<SetItemOptions>,
        }

        let helper = Helper::deserialize(deserializer)?;

        let value = serde_json::from_value(helper.value).map_err(serde::de::Error::custom)?;

        Ok(SetRequest {
            key: helper.key,
            value,
            options: helper.options,
        })
    }
}

/// Request to get an item from the cache
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetRequest {
    /// The key to retrieve
    pub key: String,
}

/// Request to remove an item from the cache
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoveRequest {
    /// The key to remove
    pub key: String,
}

/// Request to check if an item exists in the cache
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HasRequest {
    /// The key to check
    pub key: String,
}

/// Enhanced statistics about the cache
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CacheStats {
    /// Total number of items in the cache (including expired items that haven't been cleaned up yet)
    pub total_size: usize,
    /// Number of active (non-expired) items in the cache
    pub active_size: usize,
}

/// Response containing a boolean value
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BooleanResponse {
    /// The boolean value
    pub value: bool,
}

/// Empty response for operations that don't return a value
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EmptyResponse {}

/// Enhanced configuration for cache compression
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct CompressionConfig {
    /// Enable or disable compression
    pub enabled: bool,
    /// Compression level (0-9, where 0 is no compression and 9 is max compression)
    pub level: u32,
    /// Threshold in bytes after which compression is applied
    pub threshold: usize,
}

impl Default for CompressionConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            level: 6, // Default compression level
            threshold: COMPRESSION_THRESHOLD,
        }
    }
}

/// Configuration options for the cache plugin
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CacheConfig {
    /// Custom directory path for storing cache files
    pub cache_dir: Option<String>,
    /// Custom file name for the cache file
    pub cache_file_name: Option<String>,
    /// Cleanup interval in seconds
    pub cleanup_interval: Option<u64>,
    /// Default compression setting for new items
    pub default_compression: Option<bool>,
    /// Compression level (0-9, where 0 is no compression and 9 is max compression)
    pub compression_level: Option<u32>,
    /// Threshold in bytes after which compression is applied
    pub compression_threshold: Option<usize>,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            cache_dir: None,
            cache_file_name: None,
            cleanup_interval: Some(60),        // Default 60 seconds
            default_compression: Some(false),  // Default no compression
            compression_level: Some(6),        // Default medium compression level
            compression_threshold: Some(1024), // Default 1KB threshold
        }
    }
}
