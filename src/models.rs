use serde::{Deserialize, Serialize, de::DeserializeOwned};

/// Options for setting an item in the cache
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetItemOptions {
    /// Time-to-live in seconds
    pub ttl: Option<u64>,
}

/// A cache item with its value and expiration time
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CacheItem<T> {
    /// The stored value
    pub value: T,
    /// Unix timestamp in seconds when this item expires (if applicable)
    pub expires_at: Option<u64>,
}

/// Request to set an item in the cache
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetRequest<T: Serialize + DeserializeOwned> {
    /// The key to store the value under
    pub key: String,
    /// The value to store
    pub value: T,
    /// Options for this cache item
    pub options: Option<SetItemOptions>,
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

/// Simple stats about the cache
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CacheStats {
    /// Number of items in the cache
    pub size: usize,
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
