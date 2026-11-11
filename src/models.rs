use serde::{de::DeserializeOwned, Deserialize, Serialize};

// The size threshold in bytes after which compression will be applied
pub const COMPRESSION_THRESHOLD: usize = 1024; // 1KB

/// Supported compression methods
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum CompressionMethod {
    /// Zlib compression (default, balanced speed/ratio)
    #[default]
    Zlib,
    /// LZMA2 compression (better compression ratio, slower)
    Lzma2,
    /// Zstandard compression (fast compression and decompression with good ratio)
    Zstd,
}

/// Options for setting an item in the cache
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetItemOptions {
    /// Time-to-live in seconds
    pub ttl: Option<u64>,
    /// Whether to compress the data before storing
    pub compress: Option<bool>,
    /// Compression method to use (overrides default)
    pub compression_method: Option<CompressionMethod>,
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

/// Detailed information and compression metrics about a cached item
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemDetails {
    /// The cache key
    pub key: String,
    /// Original uncompressed size of the serialized value in bytes
    pub original_size: usize,
    /// Size of the stored payload in bytes (compressed payload if compressed, raw if uncompressed)
    pub compressed_size: usize,
    /// Whether the item is compressed
    pub is_compressed: bool,
    /// The compression method used if compressed
    pub compression_method: Option<CompressionMethod>,
    /// Percentage of space saved (0.0 - 100.0)
    pub compression_ratio: f64,
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
    /// Compression method to use
    pub method: CompressionMethod,
}

impl Default for CompressionConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            level: 6, // Default compression level
            threshold: COMPRESSION_THRESHOLD,
            method: CompressionMethod::Zlib,
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
    /// Compression method to use (zlib or lzma2)
    pub compression_method: Option<CompressionMethod>,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            cache_dir: None,
            cache_file_name: None,
            cleanup_interval: Some(60),        // Default 60 seconds
            default_compression: Some(true),   // Default compression enabled
            compression_level: Some(6),        // Default medium compression level
            compression_threshold: Some(1024), // Default 1KB threshold
            compression_method: Some(CompressionMethod::Zlib), // Default to Zlib
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compression_method_serde() {
        assert_eq!(
            serde_json::to_string(&CompressionMethod::Zlib).unwrap(),
            "\"zlib\""
        );
        assert_eq!(
            serde_json::to_string(&CompressionMethod::Lzma2).unwrap(),
            "\"lzma2\""
        );
        assert_eq!(
            serde_json::to_string(&CompressionMethod::Zstd).unwrap(),
            "\"zstd\""
        );
        assert_eq!(
            serde_json::from_str::<CompressionMethod>("\"zlib\"").unwrap(),
            CompressionMethod::Zlib
        );
        assert_eq!(
            serde_json::from_str::<CompressionMethod>("\"lzma2\"").unwrap(),
            CompressionMethod::Lzma2
        );
        assert_eq!(
            serde_json::from_str::<CompressionMethod>("\"zstd\"").unwrap(),
            CompressionMethod::Zstd
        );
    }

    #[test]
    fn test_cache_config_default() {
        let config = CacheConfig::default();
        assert_eq!(config.cleanup_interval, Some(60));
        assert_eq!(config.default_compression, Some(true));
        assert_eq!(config.compression_level, Some(6));
        assert_eq!(config.compression_threshold, Some(1024));
        assert_eq!(config.compression_method, Some(CompressionMethod::Zlib));
        assert!(config.cache_dir.is_none());
        assert!(config.cache_file_name.is_none());
    }

    #[test]
    fn test_compression_config_default() {
        let config = CompressionConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.level, 6);
        assert_eq!(config.threshold, COMPRESSION_THRESHOLD);
        assert_eq!(config.method, CompressionMethod::Zlib);
    }

    #[test]
    fn test_requests_serde() {
        let json = r#"{"key":"user","value":{"name":"Alice","age":30},"options":{"ttl":120,"compress":true,"compressionMethod":"zlib"}}"#;
        let set_req: SetRequest<serde_json::Value> = serde_json::from_str(json).unwrap();
        assert_eq!(set_req.key, "user");
        assert_eq!(set_req.value["name"], "Alice");
        assert_eq!(set_req.options.as_ref().unwrap().ttl, Some(120));
        assert_eq!(set_req.options.as_ref().unwrap().compress, Some(true));
        assert_eq!(
            set_req.options.as_ref().unwrap().compression_method,
            Some(CompressionMethod::Zlib)
        );

        let get_json = r#"{"key":"user"}"#;
        let get_req: GetRequest = serde_json::from_str(get_json).unwrap();
        assert_eq!(get_req.key, "user");

        let remove_json = r#"{"key":"user"}"#;
        let rem_req: RemoveRequest = serde_json::from_str(remove_json).unwrap();
        assert_eq!(rem_req.key, "user");

        let has_json = r#"{"key":"user"}"#;
        let has_req: HasRequest = serde_json::from_str(has_json).unwrap();
        assert_eq!(has_req.key, "user");
    }

    #[test]
    fn test_cache_stats_serde() {
        let stats = CacheStats {
            total_size: 10,
            active_size: 7,
        };
        let serialized = serde_json::to_string(&stats).unwrap();
        assert!(serialized.contains("\"totalSize\":10"));
        assert!(serialized.contains("\"activeSize\":7"));

        let deserialized: CacheStats = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized.total_size, 10);
        assert_eq!(deserialized.active_size, 7);
    }

    #[test]
    fn test_item_details_serde() {
        let details = ItemDetails {
            key: "test-item".to_string(),
            original_size: 1024,
            compressed_size: 256,
            is_compressed: true,
            compression_method: Some(CompressionMethod::Zstd),
            compression_ratio: 75.0,
        };
        let serialized = serde_json::to_string(&details).unwrap();
        assert!(serialized.contains("\"key\":\"test-item\""));
        assert!(serialized.contains("\"originalSize\":1024"));
        assert!(serialized.contains("\"compressedSize\":256"));
        assert!(serialized.contains("\"isCompressed\":true"));
        assert!(serialized.contains("\"compressionMethod\":\"zstd\""));
        assert!(serialized.contains("\"compressionRatio\":75.0"));

        let deserialized: ItemDetails = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized, details);
    }
}
