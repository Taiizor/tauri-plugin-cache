# Tauri Plugin Cache

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

A Tauri v2 plugin for caching data in memory with optional time-to-live (TTL) functionality. This plugin allows your Tauri application to store and retrieve data with expiration management.

## Features

- **In-Memory Cache**: Fast data storage and retrieval
- **Optional TTL**: Set expiration times for cache items
- **Cross-Platform**: Works on desktop and mobile
- **Type Safety**: Full TypeScript typings
- **Automatic Cleanup**: Background task to remove expired items
- **Cache Statistics**: Monitor cache usage

## Installation

### Rust Dependencies

Add this plugin as a dependency in your project's `Cargo.toml` file:

```toml
[dependencies]
tauri = { version = "2.0.0" }
tauri-plugin-cache = "0.1.0"
```

### JavaScript/TypeScript API

Add the plugin API package to your project:

```bash
npm install tauri-plugin-cache
# or
yarn add tauri-plugin-cache
```

## Setup

Register the plugin in your `tauri.conf.json` and/or in your Rust code:

```rust
// Rust (often in main.rs or lib.rs)
fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_cache::init())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

## Usage

### JavaScript/TypeScript Example

```typescript
import { set, get, has, remove, clear, stats } from 'tauri-plugin-cache-api';

// Store a value with TTL
await set('user', { name: 'John', age: 30 }, { ttl: 60 }); // Expires in 60 seconds

// Retrieve a value (returns null if not found or expired)
const user = await get<{ name: string, age: number }>('user');
if (user) {
  console.log(user.name); // "John"
}

// Check if a key exists and is not expired
const exists = await has('user');
if (exists) {
  console.log('User exists in cache');
}

// Remove a value
await remove('user');

// Get cache statistics
const cacheStats = await stats();
console.log(`Cache has ${cacheStats.totalSize} items (${cacheStats.activeSize} active)`);

// Clear all values
await clear();
```

### Rust Example

```rust
use tauri::Manager;
use tauri_plugin_cache::CacheExt;

// In a command or elsewhere with access to the app handle
#[tauri::command]
async fn demo_cache(app_handle: tauri::AppHandle) -> Result<String, String> {
    // Access the cache
    let cache = app_handle.cache();
    
    // Store a value with TTL
    let options = Some(tauri_plugin_cache::SetItemOptions { ttl: Some(60) });
    cache.set("key".to_string(), "value", options).map_err(|e| e.to_string())?;
    
    // Get a value
    let value: Option<String> = cache.get("key")
        .map_err(|e| e.to_string())?
        .and_then(|v| serde_json::from_value(v).ok());
        
    // Check if a key exists
    let exists = cache.has("key").map_err(|e| e.to_string())?.value;
    
    // Remove a value
    cache.remove("key").map_err(|e| e.to_string())?;
    
    // Clear all values
    cache.clear().map_err(|e| e.to_string())?;
    
    Ok("Cache operations completed".to_string())
}
```

## API

### JavaScript/TypeScript API

#### `set(key: string, value: any, options?: SetItemOptions): Promise<void>`

Sets an item in the cache with optional TTL.

- `key`: The key to store the value under
- `value`: The value to store (will be JSON serialized)
- `options`: Optional settings
  - `ttl`: Time-to-live in seconds (item will be deleted after this time)

#### `get<T = any>(key: string): Promise<T | null>`

Gets an item from the cache.

- `key`: The key to retrieve
- Returns: The stored value (type T) or null if not found or expired

#### `has(key: string): Promise<boolean>`

Checks if an item exists in the cache and is not expired.

- `key`: The key to check
- Returns: True if the item exists and is not expired

#### `remove(key: string): Promise<void>`

Removes an item from the cache.

- `key`: The key to remove

#### `clear(): Promise<void>`

Clears all items from the cache.

#### `stats(): Promise<CacheStats>`

Gets cache statistics.

- Returns: An object with statistics about the cache
  - `size`: Number of items in the cache

## Mobile Support

This plugin supports both desktop and mobile platforms:

- Desktop: Windows, macOS, Linux
- Mobile: Android, iOS

## License

This project is released under the MIT License.