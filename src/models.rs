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
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetRequest<T> 
where 
    T: Serialize + DeserializeOwned
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
    T: Serialize + DeserializeOwned
{
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>
    {
        #[derive(Deserialize)]
        struct Helper {
            key: String,
            value: serde_json::Value,
            options: Option<SetItemOptions>,
        }

        let helper = Helper::deserialize(deserializer)?;
        
        let value = serde_json::from_value(helper.value)
            .map_err(serde::de::Error::custom)?;

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
