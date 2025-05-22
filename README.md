# Tauri Plugin Cache

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://github.com/Taiizor/tauri-plugin-cache/blob/develop/LICENSE)

An advanced, versatile, and performance-focused disk caching solution for Tauri applications. This plugin features powerful compression support, memory caching layer, configurable time-to-live (TTL) management, automatic cleanup, and cross-platform compatibility. It enables persistent storage of data on disk for fast access and optimizes application performance through intelligent caching strategies. Working seamlessly on both desktop and mobile platforms, it significantly enhances the data management capabilities of your Tauri applications.

## Features

- **Disk-based Cache**: Persistent data storage and retrieval
- **Customizable Storage**: Configure where cache files are stored
- **Optional TTL**: Set expiration times for cache items
- **Data Compression**: Enable compression for large data items
- **Smart Compression**: Configurable compression levels and thresholds
- **Memory Caching**: In-memory caching layer for improved performance
- **Configurable Cache Location**: Customize where cache files are stored
- **Cross-Platform**: Works on desktop and mobile
- **Type Safety**: Full TypeScript typings
- **Automatic Cleanup**: Background task to remove expired items
- **Cache Statistics**: Monitor cache usage
- **Performance Optimized**: Buffered I/O and chunked processing for large datasets

## Installation

### Using Tauri CLI (Recommended)

The easiest way to install this plugin is using the Tauri CLI, which automatically adds both Rust and JavaScript dependencies to your project:

```bash
# Using npm
npm run tauri add cache

# Using pnpm
pnpm tauri add cache

# Using yarn
yarn tauri add cache
```

This will:
- Add the `tauri-plugin-cache` crate to your `Cargo.toml`
- Install the `tauri-plugin-cache-api` npm package
- Set up the necessary configurations

### Manual Installation

If you prefer to manually install the plugin, you can follow these steps:

#### Rust Dependencies

Add this plugin to your project using one of these methods:

```bash
# Using cargo add
cargo add tauri-plugin-cache
```

Or manually add to your `Cargo.toml` file:

```toml
[dependencies]
tauri = { version = "2.5.1" }
tauri-plugin-cache = "0.1.2"
```

#### JavaScript/TypeScript API

Add the plugin API package to your project:

```bash
pnpm install tauri-plugin-cache-api
# or
npm install tauri-plugin-cache-api
# or
yarn add tauri-plugin-cache-api
```

## Setup

Register the plugin in your `tauri.conf.json` and/or in your Rust code:

```rust
// Basic setup with default configuration
fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_cache::init())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

// Or with custom configuration
fn main() {
    let cache_config = tauri_plugin_cache::CacheConfig {
        cache_dir: Some("my_app_cache".into()),           // Custom subdirectory within app's cache directory
        cache_file_name: Some("cache_data.json".into()),  // Custom cache file name
        cleanup_interval: Some(120),                      // Clean expired items every 120 seconds
        default_compression: Some(true),                  // Enable compression by default
        compression_level: Some(7),                       // Higher compression level (0-9, where 9 is max)
        compression_threshold: Some(4096),                // Only compress items larger than 4KB
    };
    
    tauri::Builder::default()
        .plugin(tauri_plugin_cache::init_with_config(cache_config))
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

> **Note:** When specifying `cache_dir`, it's recommended to use relative paths instead of absolute paths. The plugin will create this directory inside the app's default cache directory location. If an absolute path is provided, only the last component of the path will be used as a subdirectory name within the app's cache directory.

## Permissions

By default all plugin commands are blocked and cannot be accessed. You must modify the permissions in your `capabilities` configuration to enable these.

See the [Capabilities Overview](https://v2.tauri.app/security/capabilities) for more information.

### Example Capability Configuration

```json
{
  "$schema": "../gen/schemas/desktop-schema.json",
  "identifier": "cache-access",
  "description": "Capability to access the cache functionality",
  "windows": ["main"],
  "permissions": [
    "cache:default"
  ]
}
```

Then enable this capability in your `tauri.conf.json`:

```json
{
  "app": {
    "security": {
      "capabilities": ["cache-access"]
    }
  }
}
```

### Default Permission

The `cache:default` permission set configures which cache features are exposed by default.

#### Granted Permissions
This enables all cache operations including setting, getting, and removing cached data.

#### This default permission set includes the following:
- `cache:allow-set`
- `cache:allow-get`
- `cache:allow-has`
- `cache:allow-remove`
- `cache:allow-clear`
- `cache:allow-stats`

### Permission Table

| Identifier | Description |
| ---------- | ----------- |
| cache:allow-set | Allows setting data in the cache |
| cache:allow-get | Allows retrieving data from the cache |
| cache:allow-has | Allows checking if data exists in the cache |
| cache:allow-remove | Allows removing data from the cache |
| cache:allow-clear | Allows clearing all data from the cache |
| cache:allow-stats | Allows retrieving statistics about the cache |

## Usage

### JavaScript/TypeScript Example

```typescript
import { set, get, has, remove, clear, stats } from 'tauri-plugin-cache-api';

// Store a value with TTL
await set('user', { name: 'John', age: 30 }, { ttl: 60 }); // Expires in 60 seconds

// Store a large value with compression
await set('largeData', largeObject, { compress: true });

// Store a value with both TTL and compression
await set('temporaryData', data, { ttl: 300, compress: true });

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
const stats = await stats();
console.log(`Cache has ${stats.totalSize} items (${stats.activeSize} active)`);

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
    let options = Some(tauri_plugin_cache::SetItemOptions { 
        ttl: Some(60),
        compress: None, // Use default compression setting
    });
    cache.set("key".to_string(), "value", options).map_err(|e| e.to_string())?;
    
    // Store a value with compression
    let compress_options = Some(tauri_plugin_cache::SetItemOptions {
        ttl: None,
        compress: Some(true), // Enable compression
    });
    cache.set("large_key".to_string(), large_value, compress_options).map_err(|e| e.to_string())?;
    
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

Sets an item in the cache with optional TTL and compression.

- `key`: The key to store the value under
- `value`: The value to store (will be JSON serialized)
- `options`: Optional settings
  - `ttl`: Time-to-live in seconds (item will be deleted after this time)
  - `compress`: Whether to compress the data before storing

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
  - `totalSize`: Total number of items in the cache
  - `activeSize`: Number of active (non-expired) items

## Compression

This plugin supports data compression to reduce the disk space used by cache items. You can enable compression for individual items or set it as the default for all cache items.

### Benefits of Compression

- **Reduced Disk Usage**: Compresses data to save disk space
- **Improved I/O Performance**: Smaller data sizes mean faster read/write operations
- **Network Efficiency**: If you sync cache data over a network, compressed data reduces bandwidth usage

### Configuration Options

You can configure the default compression behavior when initializing the plugin:

```rust
let cache_config = tauri_plugin_cache::CacheConfig {
    default_compression: Some(true), // Enable compression by default
    // Other options...
};
```

### Per-Item Compression

You can override the default compression setting for individual items:

```typescript
// Force compression for this item regardless of default setting
await set('largeData', largeObject, { compress: true });

// Force no compression for this item
await set('smallData', smallObject, { compress: false });
```

## Platform Compatibility

This plugin supports both desktop and mobile platforms:

- Desktop: Windows, macOS, Linux
- Mobile: Android, iOS

## License

This project is released under the [MIT License](https://github.com/Taiizor/tauri-plugin-cache/blob/develop/LICENSE).