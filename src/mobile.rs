use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::de::DeserializeOwned;
use tauri::{
  plugin::{PluginApi, PluginHandle},
  AppHandle, Runtime,
};

use crate::models::*;
use crate::{Error, Result};

#[cfg(target_os = "ios")]
tauri::ios_plugin_binding!(init_plugin_cache);

/// Access to the cache APIs.
pub struct Cache<R: Runtime> {
  handle: PluginHandle<R>,
  cache: Arc<Mutex<HashMap<String, CacheEntry>>>,
}

// initializes the Kotlin or Swift plugin classes
pub fn init<R: Runtime, C: DeserializeOwned>(
  _app: &AppHandle<R>,
  api: PluginApi<R, C>,
) -> Result<Cache<R>> {
  #[cfg(target_os = "android")]
  let handle = api.register_android_plugin("app.tauri", "cache.CachePlugin")?;
  #[cfg(target_os = "ios")]
  let handle = api.register_ios_plugin(init_plugin_cache)?;
  
  Ok(Cache {
    handle,
    cache: Arc::new(Mutex::new(HashMap::new())),
  })
}

impl<R: Runtime> Cache<R> {
  pub async fn set(&self, key: &str, value: &str, ttl: Option<i64>) -> Result<()> {
    let expires_at = ttl.map(|ttl| {
      SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_else(|_| Duration::from_secs(0))
        .as_secs() as i64 + ttl
    });

    let entry = CacheEntry {
      key: key.to_string(),
      value: value.to_string(),
      expires_at,
    };

    let mut cache = self.cache.lock().map_err(|_| Error::Other("Failed to acquire cache lock".into()))?;
    cache.insert(key.to_string(), entry);

    // For mobile, we also send the command to the native side
    #[cfg(any(target_os = "android", target_os = "ios"))]
    {
      let request = SetRequest {
        key: key.to_string(),
        value: value.to_string(),
        ttl,
      };
      self.handle.run_mobile_plugin("set", request).map_err(Into::into)?;
    }

    Ok(())
  }

  pub async fn get(&self, key: &str) -> Result<Option<String>> {
    let mut cache = self.cache.lock().map_err(|_| Error::Other("Failed to acquire cache lock".into()))?;
    
    // For mobile, we try to get from native side first
    #[cfg(any(target_os = "android", target_os = "ios"))]
    {
      let request = GetRequest {
        key: key.to_string(),
      };
      if let Ok(response) = self.handle.run_mobile_plugin::<_, GetResponse>("get", request) {
        if response.exists {
          return Ok(response.value);
        }
      }
    }
    
    if let Some(entry) = cache.get(key) {
      // Check if the entry has expired
      if let Some(expires_at) = entry.expires_at {
        let now = SystemTime::now()
          .duration_since(UNIX_EPOCH)
          .unwrap_or_else(|_| Duration::from_secs(0))
          .as_secs() as i64;

        if now > expires_at {
          // Entry has expired, remove it
          cache.remove(key);
          return Ok(None);
        }
      }

      Ok(Some(entry.value.clone()))
    } else {
      Ok(None)
    }
  }

  pub async fn has(&self, key: &str) -> Result<bool> {
    // For mobile, we try native side first
    #[cfg(any(target_os = "android", target_os = "ios"))]
    {
      let request = HasRequest {
        key: key.to_string(),
      };
      if let Ok(response) = self.handle.run_mobile_plugin::<_, HasResponse>("has_key", request) {
        return Ok(response.exists);
      }
    }
    
    let value = self.get(key).await?;
    Ok(value.is_some())
  }

  pub async fn remove(&self, key: &str) -> Result<()> {
    let mut cache = self.cache.lock().map_err(|_| Error::Other("Failed to acquire cache lock".into()))?;
    cache.remove(key);
    
    // For mobile, we also send the command to the native side
    #[cfg(any(target_os = "android", target_os = "ios"))]
    {
      let request = RemoveRequest {
        key: key.to_string(),
      };
      self.handle.run_mobile_plugin("remove", request).map_err(Into::into)?;
    }
    
    Ok(())
  }

  pub async fn clear(&self) -> Result<()> {
    let mut cache = self.cache.lock().map_err(|_| Error::Other("Failed to acquire cache lock".into()))?;
    cache.clear();
    
    // For mobile, we also send the command to the native side
    #[cfg(any(target_os = "android", target_os = "ios"))]
    {
      self.handle.run_mobile_plugin::<_, ()>("clear", ()).map_err(Into::into)?;
    }
    
    Ok(())
  }

  pub async fn keys(&self) -> Result<Vec<String>> {
    // For mobile, we try native side first
    #[cfg(any(target_os = "android", target_os = "ios"))]
    {
      if let Ok(response) = self.handle.run_mobile_plugin::<_, KeysResponse>("keys", ()) {
        return Ok(response.keys);
      }
    }
    
    let cache = self.cache.lock().map_err(|_| Error::Other("Failed to acquire cache lock".into()))?;
    let now = SystemTime::now()
      .duration_since(UNIX_EPOCH)
      .unwrap_or_else(|_| Duration::from_secs(0))
      .as_secs() as i64;
    
    // Filter out expired keys
    let keys: Vec<String> = cache
      .iter()
      .filter(|(_, v)| {
        if let Some(expires_at) = v.expires_at {
          now <= expires_at
        } else {
          true
        }
      })
      .map(|(k, _)| k.clone())
      .collect();

    Ok(keys)
  }
}