use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
pub struct CacheEntry {
    pub key: String,
    pub value: String,
    pub expires_at: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SetRequest {
    pub key: String,
    pub value: String,
    pub ttl: Option<i64>,  // Time-to-live in seconds, None means no expiration
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GetRequest {
    pub key: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GetResponse {
    pub value: Option<String>,
    pub exists: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HasRequest {
    pub key: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HasResponse {
    pub exists: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RemoveRequest {
    pub key: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct KeysResponse {
    pub keys: Vec<String>,
}
